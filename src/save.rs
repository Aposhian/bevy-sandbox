use std::fs;
use std::path::PathBuf;
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

const MAX_SLOTS: usize = 5;
const AUTO_SAVE_SECS: f32 = 60.0;

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
    pub slot: usize,
}

#[derive(bevy::prelude::Message)]
pub struct LoadGameRequest {
    pub slot: usize,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SlotInfo {
    pub slot: usize,
    pub timestamp_secs: u64,
    pub filename: String,
}

#[derive(Serialize, Deserialize, Default)]
pub struct SaveIndex {
    pub slots: Vec<SlotInfo>,
}

impl SaveIndex {
    pub fn load(dir: &PathBuf) -> Self {
        let path = dir.join("index.json");
        fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, dir: &PathBuf) {
        let path = dir.join("index.json");
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    fn next_slot(&self) -> usize {
        if self.slots.len() < MAX_SLOTS {
            self.slots.len()
        } else {
            // Overwrite the oldest
            self.slots
                .iter()
                .enumerate()
                .min_by_key(|(_, s)| s.timestamp_secs)
                .map(|(i, _)| i)
                .unwrap_or(0)
        }
    }

    pub fn find_slot(&self, slot: usize) -> Option<&SlotInfo> {
        self.slots.iter().find(|s| s.slot == slot)
    }

    fn upsert(&mut self, info: SlotInfo) {
        if let Some(existing) = self.slots.iter_mut().find(|s| s.slot == info.slot) {
            *existing = info;
        } else {
            self.slots.push(info);
        }
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
        };

        let filename = format!("save_{}.binpb", req.slot);
        let filepath = save_dir.0.join(&filename);
        let encoded = save_game.encode_to_vec();
        if let Err(e) = fs::write(&filepath, &encoded) {
            error!("Failed to write save file: {e}");
            continue;
        }

        let mut index = SaveIndex::load(&save_dir.0);
        index.upsert(SlotInfo {
            slot: req.slot,
            timestamp_secs: now,
            filename,
        });
        index.save(&save_dir.0);

        info!("Game saved to slot {}", req.slot);
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
        let index = SaveIndex::load(&save_dir.0);
        let Some(slot_info) = index.find_slot(req.slot) else {
            warn!("No save in slot {}", req.slot);
            continue;
        };

        let filepath = save_dir.0.join(&slot_info.filename);
        let data = match fs::read(&filepath) {
            Ok(d) => d,
            Err(e) => {
                error!("Failed to read save file: {e}");
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

        // Remove suppress after this frame (deferred; the tilemap systems will
        // process the spawn event next frame, then we remove suppress)
        commands.remove_resource::<SuppressObjectSpawn>();

        next_state.set(GameState::Playing);
        info!("Game loaded from slot {}", req.slot);
    }
}

fn auto_save_tick(
    time: Res<Time>,
    state: Res<State<GameState>>,
    mut timer: ResMut<AutoSaveTimer>,
    save_dir: Res<SaveDir>,
    mut save_requests: MessageWriter<SaveGameRequest>,
) {
    if *state.get() != GameState::Playing {
        return;
    }
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        let index = SaveIndex::load(&save_dir.0);
        let slot = index.next_slot();
        save_requests.write(SaveGameRequest { slot });
        info!("Auto-save triggered to slot {slot}");
    }
}
