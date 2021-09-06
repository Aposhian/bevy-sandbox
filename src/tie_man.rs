use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use benimator::{Play, SpriteSheetAnimation};

pub struct TieManPlugin;

impl Plugin for TieManPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .init_resource::<TieManTextureAtlasHandle>()
            .init_resource::<TieManAnimationHandles>()
            .add_startup_system(spawn.system());
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
    path: "tie_man_32x32.png",
    tile_size: (32.0,32.0),
    columns: 3,
    rows: 4,
    scale_factor: 3.0
};

pub fn get_texture_atlas(asset_server: &AssetServer, sprite_sheet: &SpriteSheetConfig) -> TextureAtlas {
    let texture_handle = asset_server.load(sprite_sheet.path);
    TextureAtlas::from_grid(texture_handle, Vec2::from(sprite_sheet.tile_size), sprite_sheet.columns, sprite_sheet.rows)
}

pub struct TieManTextureAtlasHandle {
    handle: Handle<TextureAtlas>
}

impl FromWorld for TieManTextureAtlasHandle {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let texture_atlas = get_texture_atlas(asset_server, &SPRITE_SHEET);
        let mut texture_atlases = world.get_resource_mut::<Assets<TextureAtlas>>().unwrap();
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        TieManTextureAtlasHandle {
            handle: texture_atlas_handle
        }
    }
}

pub struct TieManAnimationHandles {
    front_stationary: Handle<SpriteSheetAnimation>,
    profile_stationary: Handle<SpriteSheetAnimation>,
    back_stationary: Handle<SpriteSheetAnimation>,
    profile_walk: Handle<SpriteSheetAnimation>
}

impl FromWorld for TieManAnimationHandles {
    fn from_world(world: &mut World) -> Self {
        let mut animations = world.get_resource_mut::<Assets<SpriteSheetAnimation>>().unwrap();
        TieManAnimationHandles {
            front_stationary: animations.add(SpriteSheetAnimation::from_range(
                0..=2,
                Duration::from_millis(100)
            )),
            profile_stationary: animations.add(SpriteSheetAnimation::from_range(
                3..=5,
                Duration::from_millis(100)
            )),
            back_stationary: animations.add(SpriteSheetAnimation::from_range(
                6..=8,
                Duration::from_millis(100)
            )),
            profile_walk: animations.add(SpriteSheetAnimation::from_range(
                9..=11,
                Duration::from_millis(100)
            ))
        }
    }
}

fn spawn(mut commands: Commands,
    texture_atlas_handle: Res<TieManTextureAtlasHandle>,
    animations: Res<TieManAnimationHandles>) {
    commands.spawn_bundle(SpriteSheetBundle {
        texture_atlas: texture_atlas_handle.handle.clone(),
        transform: Transform::from_scale(Vec3::splat(SPRITE_SHEET.scale_factor)),
        ..Default::default()
    })
    .insert_bundle(ColliderBundle {
        shape: ColliderShape::cuboid(1.0, 1.0),
        ..Default::default()
    })
    .insert(ColliderPositionSync::Discrete)
    .insert(animations.front_stationary.clone())
    .insert(Play);
}