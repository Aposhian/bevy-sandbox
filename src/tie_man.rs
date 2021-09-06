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
            .add_startup_system(add_texture_atlas.system())
            .add_startup_system(add_animations.system())
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

pub fn get_texture_atlas(asset_server: Res<AssetServer>, sprite_sheet: &SpriteSheetConfig) -> TextureAtlas {
    let texture_handle = asset_server.load(sprite_sheet.path);
    TextureAtlas::from_grid(texture_handle, Vec2::from(sprite_sheet.tile_size), sprite_sheet.columns, sprite_sheet.rows)
}

#[derive(Default)]
pub struct TieManTextureAtlasHandle {
    handle: Handle<TextureAtlas>
}

pub fn add_texture_atlas(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>
) {
    let texture_atlas = get_texture_atlas(asset_server, &SPRITE_SHEET);
    commands.insert_resource(TieManTextureAtlasHandle {
        handle: texture_atlases.add(texture_atlas)
    });
}


#[derive(Default)]
pub struct TieManAnimationHandles {
    front_stationary: Handle<SpriteSheetAnimation>,
    profile_stationary: Handle<SpriteSheetAnimation>,
    back_stationary: Handle<SpriteSheetAnimation>,
    profile_walk: Handle<SpriteSheetAnimation>
}

pub fn add_animations(
    mut commands: Commands,
    mut animations: ResMut<Assets<SpriteSheetAnimation>>
) {
    commands.insert_resource(TieManAnimationHandles {
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
    });
}

fn spawn(
    mut commands: Commands,
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