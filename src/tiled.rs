use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use std::path::Path;
use std::collections::HashMap;

use tiled::map::Map;
use tiled::tileset::Tileset;
use tiled::layers::LayerData;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TilemapSpawnEvent>().add_system(spawn);
    }
}

pub struct TilemapSpawnEvent {
    pub path: &'static Path,
}

fn load_texture_atlas(
    tileset: &Tileset,
    asset_server: &Res<AssetServer>,
    texture_atlas_assets: &mut ResMut<Assets<TextureAtlas>>
) -> Option<Handle<TextureAtlas>> {
    if let Some(image) = &tileset.image {
        let path = std::fs::canonicalize(&image.source).unwrap();
        let texture_handle = asset_server.load(path);
        let texture_atlas = TextureAtlas::from_grid(
            texture_handle,
            Vec2::new(tileset.tile_width as f32, tileset.tile_height as f32),
            image.width as usize / tileset.tile_width as usize,
            image.height as usize / tileset.tile_height as usize
        );
        return Some(texture_atlas_assets.add(texture_atlas));
    }
    None
}

/// Spawn entities in response to spawn events
fn spawn(
    mut spawn_events: EventReader<TilemapSpawnEvent>,
    asset_server: Res<AssetServer>,
    mut texture_atlas_assets: ResMut<Assets<TextureAtlas>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>
) {
    for spawn_event in spawn_events.iter() {
        let map = Map::parse_file(spawn_event.path).unwrap();
        let mut texture_atlas_by_tile = HashMap::new();
        for tileset in map.tilesets {
            if let Some(texture_atlas_handle) = load_texture_atlas(&tileset, &asset_server, &mut texture_atlas_assets) {
                for tile in tileset.tiles {
                    texture_atlas_by_tile.insert(tile.id, texture_atlas_handle.clone());
                }
            }

            for layer in &map.layers {
                if layer.visible {

                    const CHUNK_SIZE: u32 = 64;
    
                    let mut map_settings = LayerSettings::new(
                        MapSize(
                            map.width / CHUNK_SIZE,
                            map.height / CHUNK_SIZE
                        ),
                        ChunkSize(CHUNK_SIZE, CHUNK_SIZE),
                        TileSize(tileset.tile_width as f32, tileset.tile_height as f32),
                        // TODO: don't unwrap this
                        TextureSize(*&tileset.image.unwrap().width as f32, tileset.image.unwrap().height as f32)
                    );
                    map_settings.grid_size = Vec2::new(map.tile_width as f32, map.tile_height as f32);
                    map_settings.mesh_type = TilemapMeshType::Square;

                    let layer_entitity = LayerBuilder::<TileBundle>::new_batch(
                        &mut commands,
                        map_settings.clone(),
                        &mut meshes,
                        texture_atlas_by_tile.get(tileset.first_gid),
                        016,
                        layer.layer_index as u16,
                        move |mut tile_pos| {
                            if tile_pos.0 >= tiled_map.map.width
                                || tile_pos.1 >= tiled_map.map.height
                            {
                                return None;
                            }

                            if tiled_map.map.orientation == tiled::Orientation::Orthogonal {
                                tile_pos.1 = (tiled_map.map.height - 1) as u32 - tile_pos.1;
                            }

                            let x = tile_pos.0 as usize;
                            let y = tile_pos.1 as usize;

                            let map_tile = match &layer.tiles {
                                tiled::LayerData::Finite(tiles) => &tiles[y][x],
                                _ => panic!("Infinite maps not supported"),
                            };

                            if map_tile.gid < tileset.first_gid
                                || map_tile.gid
                                    >= tileset.first_gid + tileset.tilecount.unwrap()
                            {
                                return None;
                            }

                            let tile_id = map_tile.gid - tileset.first_gid;

                            let tile = Tile {
                                texture_index: tile_id as u16,
                                flip_x: map_tile.flip_h,
                                flip_y: map_tile.flip_v,
                                flip_d: map_tile.flip_d,
                                ..Default::default()
                            };

                            Some(TileBundle {
                                tile,
                                ..Default::default()
                            })
                    )
                    if let LayerData::Finite(layer_tiles) = layer.tiles {
                        for (index, tile) in layer_tiles.iter().enumerate() {
                            let row = index as u32 / layer.height;
                            let column = index as u32 % layer.width;
                            if let Some(texture_atlas) = texture_atlas_by_tile.get(&tile.gid) {
                                commands.spawn_bundle(SpriteSheetBundle {
                                    texture_atlas: texture_atlas.clone(),
                                    sprite: TextureAtlasSprite {
                                        index: (tile.gid - 1) as usize,
                                        flip_x: tile.flip_h,
                                        flip_y: tile.flip_v,
                                        ..Default::default()
                                    },
                                    transform: Transform::from_translation(Vec3::new(
                                        layer.offset_x + (column * 32) as f32,
                                        32.0 + layer.offset_y - (row * 32) as f32,
                                        layer.layer_index as f32
                                    )),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}
