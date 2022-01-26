use bevy::prelude::*;
use std::path::Path;

use tiled::map::Map;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TilemapSpawnEvent>().add_system(spawn);
    }
}

pub struct TilemapSpawnEvent {
    pub path: &'static Path,
}

/// Spawn entities in response to spawn events
fn spawn(
    mut spawn_events: EventReader<TilemapSpawnEvent>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_assets: ResMut<Assets<TextureAtlas>>
) {
    for spawn_event in spawn_events.iter() {
        let map = Map::parse_file(spawn_event.path).unwrap();
        for tileset in map.tilesets {
            if let Some(image) = tileset.image {
                let path = std::fs::canonicalize(image.source).unwrap();
                let texture_handle = asset_server.load(path);
                let texture_atlas = TextureAtlas::from_grid(
                    texture_handle,
                    Vec2::new(tileset.tile_width as f32, tileset.tile_height as f32),
                    image.width as usize / tileset.tile_width as usize,
                    image.height as usize / tileset.tile_height as usize
                );
                texture_atlas_assets.add(texture_atlas);
            }
        }
    }
}
