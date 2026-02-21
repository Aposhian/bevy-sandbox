use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use avian2d::prelude::*;
use bevy::prelude::*;
use prost::Message;
use serde::{Deserialize, Serialize};

use crate::ball::{BallTag, BallTextureHandle};
use crate::camera::CameraTarget;
use crate::game_state::GameState;
use crate::health::{CollisionDamage, CollisionSelfDamage, DamageKind, DamageKindMask, Health};
use crate::input::{MoveAction, PlayerTag};
use crate::simple_figure::{
    AnimationIndices, AnimationTimer, GameLayer, SimpleFigureTag, SimpleFigureTextureAtlasHandle,
};
use crate::tiled::{SuppressObjectSpawn, TiledMapComponent, TilemapSpawnEvent, WallTag};
use crate::PIXELS_PER_METER;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/save.rs"));
}

const AUTO_SAVE_SECS: f32 = 60.0;

/// GC tier thresholds in seconds.
const GC_TIER_SECS: [u64; 3] = [5 * 60, 15 * 60, 30 * 60];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveTrigger {
    Auto,
    User,
    Game,
}

impl SaveTrigger {
    fn to_proto(self) -> i32 {
        match self {
            SaveTrigger::Auto => proto::SaveTrigger::Auto as i32,
            SaveTrigger::User => proto::SaveTrigger::User as i32,
            SaveTrigger::Game => proto::SaveTrigger::Game as i32,
        }
    }

    pub fn from_proto(v: i32) -> Self {
        match v {
            1 => SaveTrigger::Auto,
            2 => SaveTrigger::User,
            3 => SaveTrigger::Game,
            _ => SaveTrigger::Auto,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SaveTrigger::Auto => "Auto",
            SaveTrigger::User => "Manual",
            SaveTrigger::Game => "Game",
        }
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AutoSaveTimer(Timer::from_seconds(
            AUTO_SAVE_SECS,
            TimerMode::Repeating,
        )))
        .insert_resource(SaveDir(save_directory()))
        .insert_resource(CurrentMapPath("assets/example.tmx".to_string()))
        .add_message::<SaveGameRequest>()
        .add_message::<LoadGameRequest>()
        .add_systems(Update, (execute_save, execute_load, auto_save_tick));
    }
}

fn save_directory() -> PathBuf {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));
    dir.join("saves")
}

#[derive(Resource)]
struct AutoSaveTimer(Timer);

#[derive(Resource)]
pub struct SaveDir(pub PathBuf);

#[derive(Resource)]
pub struct CurrentMapPath(pub String);

#[derive(bevy::prelude::Message)]
pub struct SaveGameRequest {
    pub trigger: SaveTrigger,
}

#[derive(bevy::prelude::Message)]
pub struct LoadGameRequest {
    pub filename: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SlotInfo {
    pub timestamp_secs: u64,
    pub filename: String,
    pub trigger: i32,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SaveIndex {
    pub slots: Vec<SlotInfo>,
}

impl SaveIndex {
    pub fn load(dir: &Path) -> Self {
        let path = dir.join("index.json");
        match fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str(&s) {
                Ok(idx) => idx,
                Err(e) => {
                    warn!("Corrupt index.json ({e}), rebuilding from directory");
                    Self::rebuild_from_directory(dir)
                }
            },
            Err(_) => Self::rebuild_from_directory(dir),
        }
    }

    fn rebuild_from_directory(dir: &Path) -> Self {
        let mut slots = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("save_") && name_str.ends_with(".binpb") {
                    if let Ok(data) = fs::read(entry.path()) {
                        if let Ok(sg) = proto::SaveGame::decode(data.as_slice()) {
                            slots.push(SlotInfo {
                                timestamp_secs: sg.timestamp_secs,
                                filename: name_str.into_owned(),
                                trigger: sg.trigger,
                            });
                        }
                    }
                }
            }
        }
        slots.sort_by(|a, b| b.timestamp_secs.cmp(&a.timestamp_secs));
        let idx = SaveIndex { slots };
        idx.save(dir);
        idx
    }

    pub fn save(&self, dir: &Path) {
        let path = dir.join("index.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn add_entry(&mut self, info: SlotInfo, dir: &Path) {
        self.slots.push(info);
        self.slots
            .sort_by(|a, b| b.timestamp_secs.cmp(&a.timestamp_secs));
        self.save(dir);
    }

    pub fn gc(&mut self, dir: &Path) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let mut retained: HashSet<String> = HashSet::new();

        // 1. Latest auto-save
        if let Some(s) = self
            .slots
            .iter()
            .find(|s| s.trigger == proto::SaveTrigger::Auto as i32)
        {
            retained.insert(s.filename.clone());
        }

        // 2. Latest user-save
        if let Some(s) = self
            .slots
            .iter()
            .find(|s| s.trigger == proto::SaveTrigger::User as i32)
        {
            retained.insert(s.filename.clone());
        }

        // 3-5. Tiered age thresholds
        for &threshold in &GC_TIER_SECS {
            if let Some(s) = self
                .slots
                .iter()
                .find(|s| now.saturating_sub(s.timestamp_secs) >= threshold)
            {
                retained.insert(s.filename.clone());
            }
        }

        // If retained set is empty (no saves match criteria), keep the most recent
        if retained.is_empty() {
            if let Some(s) = self.slots.first() {
                retained.insert(s.filename.clone());
            }
        }

        // Delete non-retained files
        let to_remove: Vec<SlotInfo> = self
            .slots
            .iter()
            .filter(|s| !retained.contains(&s.filename))
            .cloned()
            .collect();

        for info in &to_remove {
            let path = dir.join(&info.filename);
            let _ = fs::remove_file(path);
        }

        self.slots.retain(|s| retained.contains(&s.filename));
        self.slots
            .sort_by(|a, b| b.timestamp_secs.cmp(&a.timestamp_secs));
        self.save(dir);
    }
}

fn execute_save(
    mut requests: MessageReader<SaveGameRequest>,
    save_dir: Res<SaveDir>,
    map_path: Res<CurrentMapPath>,
    player_query: Query<(&Transform, &LinearVelocity), (With<PlayerTag>, With<SimpleFigureTag>)>,
    npc_query: Query<
        (&Transform, &LinearVelocity, &Health),
        (With<SimpleFigureTag>, Without<PlayerTag>),
    >,
    ball_query: Query<
        (
            &Transform,
            &LinearVelocity,
            &Health,
            &CollisionDamage,
            &CollisionSelfDamage,
        ),
        With<BallTag>,
    >,
    camera_query: Query<&Transform, With<Camera2d>>,
) {
    for req in requests.read() {
        let _ = fs::create_dir_all(&save_dir.0);

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let player = player_query
            .iter()
            .next()
            .map(|(tf, vel)| proto::PlayerState {
                position: Some(proto::Vec2 {
                    x: tf.translation.x,
                    y: tf.translation.y,
                }),
                velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
            });

        let npcs: Vec<proto::NpcState> = npc_query
            .iter()
            .map(|(tf, vel, health)| proto::NpcState {
                position: Some(proto::Vec2 {
                    x: tf.translation.x,
                    y: tf.translation.y,
                }),
                velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
                health_max: health.max,
                health_current: health.current,
                vulnerable_to_mask: health.vulnerable_to.0,
                goal_position: None,
            })
            .collect();

        let balls: Vec<proto::BallState> = ball_query
            .iter()
            .map(|(tf, vel, health, cd, csd)| proto::BallState {
                position: Some(proto::Vec2 {
                    x: tf.translation.x,
                    y: tf.translation.y,
                }),
                velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
                health_max: health.max,
                health_current: health.current,
                vulnerable_to_mask: health.vulnerable_to.0,
                collision_damage: cd.damage,
                collision_self_damage: csd.damage,
            })
            .collect();

        let camera_position = camera_query.iter().next().map(|tf| proto::Vec2 {
            x: tf.translation.x,
            y: tf.translation.y,
        });

        let save_game = proto::SaveGame {
            timestamp_secs: now,
            map_path: map_path.0.clone(),
            player,
            npcs,
            balls,
            camera_position,
            trigger: req.trigger.to_proto(),
        };

        let filename = format!("save_{now}.binpb");
        let filepath = save_dir.0.join(&filename);
        let encoded = save_game.encode_to_vec();
        if let Err(e) = fs::write(&filepath, &encoded) {
            error!("Failed to write save file: {e}");
            continue;
        }

        let mut index = SaveIndex::load(&save_dir.0);
        index.add_entry(
            SlotInfo {
                timestamp_secs: now,
                filename: filename.clone(),
                trigger: req.trigger.to_proto(),
            },
            &save_dir.0,
        );
        index.gc(&save_dir.0);

        info!(
            "Game saved: {} ({})",
            filename,
            req.trigger.label()
        );
    }
}

fn execute_load(
    mut commands: Commands,
    mut requests: MessageReader<LoadGameRequest>,
    save_dir: Res<SaveDir>,
    mut map_path: ResMut<CurrentMapPath>,
    mut next_state: ResMut<NextState<GameState>>,
    figures: Query<Entity, With<SimpleFigureTag>>,
    balls: Query<Entity, With<BallTag>>,
    walls: Query<Entity, With<WallTag>>,
    tilemap_entities: Query<Entity, With<TiledMapComponent>>,
    atlas_handle: Res<SimpleFigureTextureAtlasHandle>,
    ball_texture: Res<BallTextureHandle>,
    mut tilemap_spawn: MessageWriter<TilemapSpawnEvent>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    for req in requests.read() {
        let filepath = save_dir.0.join(&req.filename);
        let data = match fs::read(&filepath) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to read save file {}: {e}", req.filename);
                continue;
            }
        };

        let save_game = match proto::SaveGame::decode(data.as_slice()) {
            Ok(sg) => sg,
            Err(e) => {
                error!("Failed to decode save: {e}");
                continue;
            }
        };

        // Despawn all gameplay entities
        for entity in figures.iter() {
            commands.entity(entity).despawn();
        }
        for entity in balls.iter() {
            commands.entity(entity).despawn();
        }
        for entity in walls.iter() {
            commands.entity(entity).despawn();
        }
        for entity in tilemap_entities.iter() {
            commands.entity(entity).despawn();
        }

        // Suppress object spawning from tilemap
        commands.insert_resource(SuppressObjectSpawn);

        // Reload the tilemap
        map_path.0 = save_game.map_path.clone();
        tilemap_spawn.write(TilemapSpawnEvent {
            path: save_game.map_path.clone(),
            objects_enabled: false,
        });

        // Spawn player
        if let Some(ps) = &save_game.player {
            let pos = ps
                .position
                .as_ref()
                .map(|p| Vec2::new(p.x, p.y))
                .unwrap_or_default();
            let vel = ps
                .velocity
                .as_ref()
                .map(|v| Vec2::new(v.x, v.y))
                .unwrap_or_default();

            commands.spawn((
                SimpleFigureTag,
                Sprite::from_atlas_image(
                    atlas_handle.texture.clone(),
                    TextureAtlas {
                        layout: atlas_handle.layout.clone(),
                        index: 0,
                    },
                ),
                Transform::from_translation(Vec3::new(pos.x, pos.y, 2.0)),
                AnimationIndices { first: 0, last: 2 },
                AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
                RigidBody::Dynamic,
                Collider::capsule(0.18 * PIXELS_PER_METER, 0.6 * PIXELS_PER_METER),
                CollisionLayers::new(
                    LayerMask::from([GameLayer::Character]),
                    LayerMask::from([GameLayer::Character, GameLayer::Wall, GameLayer::Ball]),
                ),
                CollisionEventsEnabled,
                LockedAxes::ROTATION_LOCKED,
                MoveAction::default(),
                LinearVelocity(vel),
                PlayerTag,
                CameraTarget,
            ));
        }

        // Spawn NPCs
        for npc in &save_game.npcs {
            let pos = npc
                .position
                .as_ref()
                .map(|p| Vec2::new(p.x, p.y))
                .unwrap_or_default();
            let vel = npc
                .velocity
                .as_ref()
                .map(|v| Vec2::new(v.x, v.y))
                .unwrap_or_default();

            commands.spawn((
                SimpleFigureTag,
                Sprite::from_atlas_image(
                    atlas_handle.texture.clone(),
                    TextureAtlas {
                        layout: atlas_handle.layout.clone(),
                        index: 0,
                    },
                ),
                Transform::from_translation(Vec3::new(pos.x, pos.y, 2.0)),
                AnimationIndices { first: 0, last: 2 },
                AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
                RigidBody::Dynamic,
                Collider::capsule(0.18 * PIXELS_PER_METER, 0.6 * PIXELS_PER_METER),
                CollisionLayers::new(
                    LayerMask::from([GameLayer::Character]),
                    LayerMask::from([GameLayer::Character, GameLayer::Wall, GameLayer::Ball]),
                ),
                CollisionEventsEnabled,
                LockedAxes::ROTATION_LOCKED,
                MoveAction::default(),
                LinearVelocity(vel),
                Health {
                    max: npc.health_max,
                    current: npc.health_current,
                    vulnerable_to: DamageKindMask(npc.vulnerable_to_mask),
                },
            ));
        }

        // Spawn balls
        for ball in &save_game.balls {
            let pos = ball
                .position
                .as_ref()
                .map(|p| Vec2::new(p.x, p.y))
                .unwrap_or_default();
            let vel = ball
                .velocity
                .as_ref()
                .map(|v| Vec2::new(v.x, v.y))
                .unwrap_or_default();

            commands.spawn((
                BallTag,
                CollisionDamage {
                    damage: ball.collision_damage,
                    kind: DamageKind::Projectile,
                },
                CollisionSelfDamage {
                    damage: ball.collision_self_damage,
                    kind: DamageKind::Impact,
                },
                Health {
                    max: ball.health_max,
                    current: ball.health_current,
                    vulnerable_to: DamageKindMask(ball.vulnerable_to_mask),
                },
                Sprite::from_image(ball_texture.0.clone()),
                Transform::from_translation(Vec3::new(pos.x, pos.y, 2.0)),
                RigidBody::Dynamic,
                Collider::circle(0.1 * PIXELS_PER_METER),
                CollisionLayers::new(
                    LayerMask::from([GameLayer::Ball]),
                    LayerMask::from([GameLayer::Character, GameLayer::Ball, GameLayer::Wall]),
                ),
                CollisionEventsEnabled,
                Restitution::new(1.0),
                ColliderDensity(0.001),
                LockedAxes::ROTATION_LOCKED,
                LinearVelocity(vel),
            ));
        }

        // Set camera position
        if let Some(cam_pos) = &save_game.camera_position {
            if let Ok(mut cam_tf) = camera_query.single_mut() {
                cam_tf.translation.x = cam_pos.x;
                cam_tf.translation.y = cam_pos.y;
            }
        }

        // Remove suppress after this frame
        commands.remove_resource::<SuppressObjectSpawn>();

        next_state.set(GameState::Playing);
        info!("Game loaded from {}", req.filename);
    }
}

fn auto_save_tick(
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut timer: ResMut<AutoSaveTimer>,
    mut save_requests: MessageWriter<SaveGameRequest>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        save_requests.write(SaveGameRequest {
            trigger: SaveTrigger::Auto,
        });
        info!("Auto-save triggered");
    }
}
