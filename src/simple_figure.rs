use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use benimator::{Play, SpriteSheetAnimation};
use std::f32::consts::FRAC_PI_4;

use crate::input::MoveAction;

pub struct SimpleFigurePlugin;

impl Plugin for SimpleFigurePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .init_resource::<SimpleFigureTextureAtlasHandle>()
            .init_resource::<SimpleFigureAnimationHandles>()
            .add_startup_system(setup_physics.system())
            .add_startup_system(spawn.system())
            .add_system(animation_control.system());
    }
}

pub struct SpriteSheetConfig {
    path: &'static str,
    tile_size: (f32, f32),
    columns: usize,
    rows: usize,
    scale_factor: f32
}

const SPRITE_SHEET: SpriteSheetConfig = SpriteSheetConfig {
    path: "simple_figure_32x32.png",
    tile_size: (32.0,32.0),
    columns: 3,
    rows: 6,
    scale_factor: 3.0
};

pub fn get_texture_atlas(asset_server: &AssetServer, sprite_sheet: &SpriteSheetConfig) -> TextureAtlas {
    let texture_handle = asset_server.load(sprite_sheet.path);
    TextureAtlas::from_grid(texture_handle, Vec2::from(sprite_sheet.tile_size), sprite_sheet.columns, sprite_sheet.rows)
}

/// Resource for holding texture atlas
pub struct SimpleFigureTextureAtlasHandle {
    handle: Handle<TextureAtlas>
}

impl FromWorld for SimpleFigureTextureAtlasHandle {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let texture_atlas = get_texture_atlas(asset_server, &SPRITE_SHEET);
        let mut texture_atlases = world.get_resource_mut::<Assets<TextureAtlas>>().unwrap();
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        SimpleFigureTextureAtlasHandle {
            handle: texture_atlas_handle
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
    back_walk: Handle<SpriteSheetAnimation>
}

impl SimpleFigureAnimationHandles {
    fn walking(&self, velocity: Vec2) -> &Handle<SpriteSheetAnimation> {
        assert!(velocity.length_squared() != 0.0);
        let angle = velocity.angle_between(Vec2::new(1.0, 0.0));
        if (-FRAC_PI_4 <= angle && angle <= FRAC_PI_4) || (3.0 * FRAC_PI_4 <= angle || angle <= - 3.0 * FRAC_PI_4) {
            &self.profile_walk
        } else {
            if velocity.y >= 0.0 {
                &self.back_walk
            } else {
                &self.front_walk
            }
        }
    }

    fn stationary(&self, previous_handle: &Handle<SpriteSheetAnimation>) -> &Handle<SpriteSheetAnimation> {
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
        let mut animations = world.get_resource_mut::<Assets<SpriteSheetAnimation>>().unwrap();
        SimpleFigureAnimationHandles {
            front_stationary: animations.add(SpriteSheetAnimation::from_range(
                0..=2,
                Duration::from_millis(100)
            )),
            front_walk: animations.add(SpriteSheetAnimation::from_range(
                3..=5,
                Duration::from_millis(100)
            )),
            profile_stationary: animations.add(SpriteSheetAnimation::from_range(
                6..=8,
                Duration::from_millis(100)
            )),
            profile_walk: animations.add(SpriteSheetAnimation::from_range(
                9..=11,
                Duration::from_millis(100)
            )),
            back_stationary: animations.add(SpriteSheetAnimation::from_range(
                12..=14,
                Duration::from_millis(100)
            )),
            back_walk: animations.add(SpriteSheetAnimation::from_range(
                15..=17,
                Duration::from_millis(100)
            )),
        }
    }
}

fn setup_physics(mut configuration: ResMut<RapierConfiguration>) {
    // This is pixels per physics meter
    configuration.scale = 50.0;
}

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
    move_action: MoveAction
}

fn spawn(mut commands: Commands,
    texture_atlas_handle: Res<SimpleFigureTextureAtlasHandle>,
    animations: Res<SimpleFigureAnimationHandles>) {
    commands.spawn_bundle(SimpleFigureBundle {
        tag: SimpleFigureTag,
        sprite_sheet_bundle: SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.handle.clone(),
            transform: Transform::from_scale(Vec3::splat(SPRITE_SHEET.scale_factor)),
            ..Default::default()
        },
        animation: animations.front_stationary.clone(),
        play: Play,
        rigid_body_bundle: RigidBodyBundle {
            forces: RigidBodyForces {
                gravity_scale: 0.0,
                ..Default::default()
            },
            ..Default::default()
        },
        position_sync: RigidBodyPositionSync::Discrete,
        collider_bundle: ColliderBundle {
            shape: ColliderShape::cuboid(1.0, 1.0),
            ..Default::default()
        },
        move_action: MoveAction::default()
    });
}

fn animation_control(
    animation_handles: Res<SimpleFigureAnimationHandles>,
    mut query: Query<(&SimpleFigureTag, &RigidBodyVelocity, &mut TextureAtlasSprite, &mut Handle<SpriteSheetAnimation>)>) {
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