use std::time::Duration;

use benimator::{Play, SpriteSheetAnimation};
use bevy::prelude::*;
use bevy_rapier2d::{na::Isometry2, prelude::*};
use std::f32::consts::FRAC_PI_4;

use crate::camera::CameraTarget;
use crate::health::Health;
use crate::input::{MoveAction, PlayerTag};

pub struct SimpleFigurePlugin;

impl Plugin for SimpleFigurePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimpleFigureTextureAtlasHandle>()
            .init_resource::<SimpleFigureAnimationHandles>()
            .add_event::<SimpleFigureSpawnEvent>()
            .add_startup_system(setup_physics)
            .add_system(animation_control)
            .add_system(spawn);
    }
}

pub struct SpriteSheetConfig {
    path: &'static str,
    tile_size: (f32, f32),
    columns: usize,
    rows: usize,
}

const SPRITE_SHEET: SpriteSheetConfig = SpriteSheetConfig {
    path: "spritesheets/simple_figure_32x32.png",
    tile_size: (32.0, 32.0),
    columns: 3,
    rows: 6,
};

pub fn get_texture_atlas(
    asset_server: &AssetServer,
    sprite_sheet: &SpriteSheetConfig,
) -> TextureAtlas {
    let texture_handle = asset_server.load(sprite_sheet.path);
    TextureAtlas::from_grid(
        texture_handle,
        Vec2::from(sprite_sheet.tile_size),
        sprite_sheet.columns,
        sprite_sheet.rows,
    )
}

/// Resource for holding texture atlas
pub struct SimpleFigureTextureAtlasHandle {
    handle: Handle<TextureAtlas>,
}

impl FromWorld for SimpleFigureTextureAtlasHandle {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let texture_atlas = get_texture_atlas(asset_server, &SPRITE_SHEET);
        let mut texture_atlases = world.get_resource_mut::<Assets<TextureAtlas>>().unwrap();
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        SimpleFigureTextureAtlasHandle {
            handle: texture_atlas_handle,
        }
    }
}

/// Resource for holding animation handles
pub struct SimpleFigureAnimationHandles {
    front_stationary: Handle<SpriteSheetAnimation>,
    front_walk: Handle<SpriteSheetAnimation>,
    profile_stationary: Handle<SpriteSheetAnimation>,
    profile_walk: Handle<SpriteSheetAnimation>,
    back_stationary: Handle<SpriteSheetAnimation>,
    back_walk: Handle<SpriteSheetAnimation>,
}

impl SimpleFigureAnimationHandles {
    fn walking(&self, velocity: Vec2) -> &Handle<SpriteSheetAnimation> {
        assert!(velocity.length_squared() != 0.0);
        let angle = velocity.angle_between(Vec2::new(1.0, 0.0));
        if (-FRAC_PI_4 <= angle && angle <= FRAC_PI_4)
            || (3.0 * FRAC_PI_4 <= angle || angle <= -3.0 * FRAC_PI_4)
        {
            &self.profile_walk
        } else {
            if velocity.y >= 0.0 {
                &self.back_walk
            } else {
                &self.front_walk
            }
        }
    }

    fn stationary(
        &self,
        previous_handle: &Handle<SpriteSheetAnimation>,
    ) -> &Handle<SpriteSheetAnimation> {
        if [self.profile_walk.id, self.profile_stationary.id].contains(&previous_handle.id) {
            &self.profile_stationary
        } else if [self.back_walk.id, self.back_stationary.id].contains(&previous_handle.id) {
            &self.back_stationary
        } else {
            &self.front_stationary
        }
    }
}

impl FromWorld for SimpleFigureAnimationHandles {
    fn from_world(world: &mut World) -> Self {
        let mut animations = world
            .get_resource_mut::<Assets<SpriteSheetAnimation>>()
            .unwrap();
        SimpleFigureAnimationHandles {
            front_stationary: animations.add(SpriteSheetAnimation::from_range(
                0..=2,
                Duration::from_millis(100),
            )),
            front_walk: animations.add(SpriteSheetAnimation::from_range(
                3..=5,
                Duration::from_millis(100),
            )),
            profile_stationary: animations.add(SpriteSheetAnimation::from_range(
                6..=8,
                Duration::from_millis(100),
            )),
            profile_walk: animations.add(SpriteSheetAnimation::from_range(
                9..=11,
                Duration::from_millis(100),
            )),
            back_stationary: animations.add(SpriteSheetAnimation::from_range(
                12..=14,
                Duration::from_millis(100),
            )),
            back_walk: animations.add(SpriteSheetAnimation::from_range(
                15..=17,
                Duration::from_millis(100),
            )),
        }
    }
}

fn setup_physics(mut configuration: ResMut<RapierConfiguration>) {
    // This is pixels per physics meter
    configuration.scale = 32.0;
}

#[derive(Default, Component)]
pub struct SimpleFigureTag;

#[derive(Bundle)]
pub struct SimpleFigureBundle {
    tag: SimpleFigureTag,
    #[bundle]
    sprite_sheet_bundle: SpriteSheetBundle,
    animation: Handle<SpriteSheetAnimation>,
    play: Play,
    #[bundle]
    rigid_body_bundle: RigidBodyBundle,
    position_sync: RigidBodyPositionSync,
    #[bundle]
    collider_bundle: ColliderBundle,
    move_action: MoveAction,
}

impl Default for SimpleFigureBundle {
    fn default() -> Self {
        SimpleFigureBundle {
            tag: Default::default(),
            sprite_sheet_bundle: SpriteSheetBundle::default(),
            animation: Default::default(),
            play: Default::default(),
            rigid_body_bundle: Default::default(),
            position_sync: RigidBodyPositionSync::Discrete,
            collider_bundle: ColliderBundle {
                shape: ColliderShape::cuboid(0.18, 0.40).into(),
                flags: ColliderFlags {
                    collision_groups: InteractionGroups::new(0b0111, 0b0111),
                    active_events: ActiveEvents::CONTACT_EVENTS,
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            },
            move_action: Default::default(),
        }
    }
}

#[derive(Debug)]
pub struct SimpleFigureSpawnEvent {
    pub position: Isometry2<f32>,
    pub scale: f32,
    pub z: f32,
    pub playable: bool,
}

impl Default for SimpleFigureSpawnEvent {
    fn default() -> Self {
        SimpleFigureSpawnEvent {
            position: Isometry2::identity(),
            scale: 1.0,
            z: 2.0,
            playable: false,
        }
    }
}

/// Should be used for debugging only
pub fn default_spawn(mut spawn_event: EventWriter<SimpleFigureSpawnEvent>) {
    spawn_event.send(SimpleFigureSpawnEvent {
        playable: true,
        ..Default::default()
    })
}

/// Spawn entities in response to spawn events
fn spawn(
    mut commands: Commands,
    texture_atlas_handle: Res<SimpleFigureTextureAtlasHandle>,
    animations: Res<SimpleFigureAnimationHandles>,
    mut spawn_events: EventReader<SimpleFigureSpawnEvent>,
) {
    for spawn_event in spawn_events.iter() {
        let mut entity_commands = commands.spawn_bundle(SimpleFigureBundle {
            sprite_sheet_bundle: SpriteSheetBundle {
                texture_atlas: texture_atlas_handle.handle.clone(),
                transform: Transform::from_scale(Vec3::splat(spawn_event.scale))
                    * Transform::from_translation(Vec3::new(0.0, 0.0, spawn_event.z)),
                ..Default::default()
            },
            animation: animations.front_stationary.clone(),
            rigid_body_bundle: RigidBodyBundle {
                mass_properties: RigidBodyMassPropsFlags::ROTATION_LOCKED.into(),
                forces: RigidBodyForces {
                    gravity_scale: 0.0,
                    ..Default::default()
                }
                .into(),
                position: spawn_event.position.into(),
                ..Default::default()
            },
            ..Default::default()
        });
        if spawn_event.playable {
            entity_commands.insert(PlayerTag).insert(CameraTarget);
        } else {
            entity_commands.insert(Health::from_max(5));
        }
    }
}

fn animation_control(
    animation_handles: Res<SimpleFigureAnimationHandles>,
    mut query: Query<(
        &SimpleFigureTag,
        &RigidBodyVelocityComponent,
        &mut TextureAtlasSprite,
        &mut Handle<SpriteSheetAnimation>,
    )>,
) {
    for (_tag, velocity, mut sprite, mut animation) in query.iter_mut() {
        if Vec2::from(velocity.linvel).length_squared() == 0.0 {
            *animation = animation_handles.stationary(&animation).clone();
        } else {
            *animation = animation_handles.walking(velocity.linvel.into()).clone();
        }

        if velocity.linvel.x < 0.0 {
            sprite.flip_x = true;
        } else if velocity.linvel.x > 0.0 {
            sprite.flip_x = false;
        }
    }
}
