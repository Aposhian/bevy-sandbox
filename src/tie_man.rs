use std::time::Duration;

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use benimator::{Play, SpriteSheetAnimation};

pub struct TieManPlugin;

impl Plugin for TieManPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(spawn.system());
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

pub fn get_texture_atlas(asset_server: &Res<AssetServer>, sprite_sheet: &SpriteSheetConfig) -> TextureAtlas {
    let texture_handle = asset_server.load(sprite_sheet.path);
    TextureAtlas::from_grid(texture_handle, Vec2::from(sprite_sheet.tile_size), sprite_sheet.columns, sprite_sheet.rows)
}

fn spawn(mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut animations: ResMut<Assets<SpriteSheetAnimation>>) {

    let texture_atlas = get_texture_atlas(&asset_server, &SPRITE_SHEET);
    let texture_atlas_handle =  texture_atlases.add(texture_atlas);

    let animation_handle = animations.add(SpriteSheetAnimation::from_range(
        0..=2,                               // Indices of the sprite atlas
        Duration::from_secs_f64(1.0 / 12.0), // Duration of each frame
    ));

    commands.spawn_bundle(SpriteSheetBundle {
        texture_atlas: texture_atlas_handle,
        transform: Transform::from_scale(Vec3::splat(SPRITE_SHEET.scale_factor)),
        ..Default::default()
    })
    .insert_bundle(ColliderBundle {
        shape: ColliderShape::cuboid(1.0, 1.0),
        ..Default::default()
    })
    .insert(ColliderPositionSync::Discrete)
    .insert(animation_handle)
    .insert(Play);
}