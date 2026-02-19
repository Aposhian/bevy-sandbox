use std::time::Duration;

use avian2d::prelude::*;
use bevy::prelude::*;
use std::f32::consts::FRAC_PI_4;

use crate::camera::CameraTarget;
use crate::health::Health;
use crate::input::{MoveAction, PlayerTag};
use crate::PIXELS_PER_METER;

pub struct SimpleFigurePlugin;

impl Plugin for SimpleFigurePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimpleFigureTextureAtlasHandle>()
            .add_message::<SimpleFigureSpawnEvent>()
            .add_systems(Update, (animation_control, animate_sprite, spawn));
    }
}

pub struct SpriteSheetConfig {
    path: &'static str,
    tile_size: UVec2,
    columns: u32,
    rows: u32,
}

const SPRITE_SHEET: SpriteSheetConfig = SpriteSheetConfig {
    path: "spritesheets/simple_figure_32x32.png",
    tile_size: UVec2::new(32, 32),
    columns: 3,
    rows: 6,
};

/// Resource for holding texture and atlas layout
#[derive(Resource)]
pub struct SimpleFigureTextureAtlasHandle {
    texture: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
}

impl FromWorld for SimpleFigureTextureAtlasHandle {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let texture = asset_server.load(SPRITE_SHEET.path);
        let layout = TextureAtlasLayout::from_grid(
            SPRITE_SHEET.tile_size,
            SPRITE_SHEET.columns,
            SPRITE_SHEET.rows,
            None,
            None,
        );
        let mut layouts = world.get_resource_mut::<Assets<TextureAtlasLayout>>().unwrap();
        let layout_handle = layouts.add(layout);
        SimpleFigureTextureAtlasHandle {
            texture,
            layout: layout_handle,
        }
    }
}

/// Animation indices for a sprite sheet animation range
#[derive(Component, Clone)]
pub struct AnimationIndices {
    pub first: usize,
    pub last: usize,
}

/// Timer that drives sprite animation
#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

/// Defines the six animation states for a simple figure
struct AnimationSet {
    front_stationary: AnimationIndices,
    front_walk: AnimationIndices,
    profile_stationary: AnimationIndices,
    profile_walk: AnimationIndices,
    back_stationary: AnimationIndices,
    back_walk: AnimationIndices,
}

const ANIMATIONS: AnimationSet = AnimationSet {
    front_stationary: AnimationIndices { first: 0, last: 2 },
    front_walk: AnimationIndices { first: 3, last: 5 },
    profile_stationary: AnimationIndices { first: 6, last: 8 },
    profile_walk: AnimationIndices { first: 9, last: 11 },
    back_stationary: AnimationIndices { first: 12, last: 14 },
    back_walk: AnimationIndices { first: 15, last: 17 },
};

impl AnimationSet {
    fn walking(&self, velocity: Vec2) -> &AnimationIndices {
        let angle = velocity.angle_to(Vec2::new(1.0, 0.0));
        if (-FRAC_PI_4 <= angle && angle <= FRAC_PI_4)
            || (3.0 * FRAC_PI_4 <= angle || angle <= -3.0 * FRAC_PI_4)
        {
            &self.profile_walk
        } else if velocity.y >= 0.0 {
            &self.back_walk
        } else {
            &self.front_walk
        }
    }

    fn stationary(&self, current_indices: &AnimationIndices) -> &AnimationIndices {
        if current_indices.first == self.profile_walk.first
            || current_indices.first == self.profile_stationary.first
        {
            &self.profile_stationary
        } else if current_indices.first == self.back_walk.first
            || current_indices.first == self.back_stationary.first
        {
            &self.back_stationary
        } else {
            &self.front_stationary
        }
    }
}

#[derive(Default, Component)]
pub struct SimpleFigureTag;

#[derive(Debug, Message)]
pub struct SimpleFigureSpawnEvent {
    pub position: Vec2,
    pub scale: f32,
    pub z: f32,
    pub playable: bool,
}

impl Default for SimpleFigureSpawnEvent {
    fn default() -> Self {
        SimpleFigureSpawnEvent {
            position: Vec2::ZERO,
            scale: 1.0,
            z: 2.0,
            playable: false,
        }
    }
}

/// Should be used for debugging only
pub fn default_spawn(mut spawn_event: MessageWriter<SimpleFigureSpawnEvent>) {
    spawn_event.write(SimpleFigureSpawnEvent {
        playable: true,
        ..Default::default()
    });
}

/// Spawn entities in response to spawn events
fn spawn(
    mut commands: Commands,
    atlas_handle: Res<SimpleFigureTextureAtlasHandle>,
    mut spawn_events: MessageReader<SimpleFigureSpawnEvent>,
) {
    for spawn_event in spawn_events.read() {
        let mut entity_commands = commands.spawn((
            SimpleFigureTag,
            Sprite::from_atlas_image(
                atlas_handle.texture.clone(),
                TextureAtlas {
                    layout: atlas_handle.layout.clone(),
                    index: ANIMATIONS.front_stationary.first,
                },
            ),
            Transform::from_translation(Vec3::new(
                spawn_event.position.x,
                spawn_event.position.y,
                spawn_event.z,
            ))
            .with_scale(Vec3::splat(spawn_event.scale)),
            ANIMATIONS.front_stationary.clone(),
            AnimationTimer(Timer::new(Duration::from_millis(100), TimerMode::Repeating)),
            // Physics
            RigidBody::Dynamic,
            Collider::rectangle(
                0.36 * PIXELS_PER_METER,
                0.80 * PIXELS_PER_METER,
            ),
            CollisionLayers::new(
                LayerMask::from([GameLayer::Character]),
                LayerMask::from([GameLayer::Character, GameLayer::Wall]),
            ),
            CollisionEventsEnabled,
            LockedAxes::ROTATION_LOCKED,
            MoveAction::default(),
        ));
        if spawn_event.playable {
            entity_commands.insert((PlayerTag, CameraTarget));
        } else {
            entity_commands.insert(Health::from_max(5));
        }
    }
}

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());

        if timer.just_finished() {
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = if atlas.index >= indices.last {
                    indices.first
                } else {
                    atlas.index + 1
                };
            }
        }
    }
}

fn animation_control(
    mut query: Query<(
        &SimpleFigureTag,
        &LinearVelocity,
        &mut Sprite,
        &mut AnimationIndices,
    )>,
) {
    for (_tag, velocity, mut sprite, mut indices) in query.iter_mut() {
        let new_indices = if velocity.0.length_squared() == 0.0 {
            ANIMATIONS.stationary(&indices).clone()
        } else {
            ANIMATIONS.walking(velocity.0).clone()
        };

        if new_indices.first != indices.first {
            *indices = new_indices;
            // Reset sprite index to start of new animation
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = indices.first;
            }
        }

        if velocity.x < 0.0 {
            sprite.flip_x = true;
        } else if velocity.x > 0.0 {
            sprite.flip_x = false;
        }
    }
}

/// Physics collision layers
#[derive(avian2d::prelude::PhysicsLayer, Clone, Copy, Debug, Default)]
pub enum GameLayer {
    #[default]
    Character,
    Ball,
    Wall,
}
