use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use std::path::Path;

use tiled::{FilesystemResourceCache, Map, Tileset};

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TilemapSpawnEvent>().add_system(spawn);
    }
}

#[derive(Default, Bundle)]
pub struct TiledMapBundle {
    pub map: bevy_ecs_tilemap::Map,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
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
            texture_handle.clone(),
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
        let mut cache = FilesystemResourceCache::new();
        let map = Map::parse_file(spawn_event.path, &mut cache).unwrap();

        for tileset in map.tilesets() {
            // TODO: make this handle multiple textures
            let texture_atlas_handle = load_texture_atlas(&tileset, &asset_server, &mut texture_atlas_assets).unwrap();

            // TODO: take this out of this loop
            for layer in map.layers() {
                if layer.visible() {

                    // const CHUNK_SIZE: u32 = 64;
    
                    // let mut map_settings = LayerSettings::new(
                    //     MapSize(
                    //         map.width / CHUNK_SIZE,
                    //         map.height / CHUNK_SIZE
                    //     ),
                    //     ChunkSize(CHUNK_SIZE, CHUNK_SIZE),
                    //     TileSize(tileset.tile_width as f32, tileset.tile_height as f32),
                    //     // TODO: don't unwrap this
                    //     TextureSize(tileset.image.clone().unwrap().width as f32, tileset.image.clone().unwrap().height as f32)
                    // );
                    // map_settings.grid_size = Vec2::new(map.tile_width as f32, map.tile_height as f32);
                    // map_settings.mesh_type = TilemapMeshType::Square;

                    let layer_type = layer.layer_type();
                    let tile_layer =  match layer_type {
                        tiled::LayerType::TileLayer(layer) => match layer {
                            tiled::TileLayer::Finite(data) => data,
                            tiled::TileLayer::Infinite(_) => {
                                panic!("infinite tilemaps not supported");
                            }
                        },
                        tiled::LayerType::ObjectLayer(_) => {
                            panic!("object layers not supported yet")
                        },
                        tiled::LayerType::ImageLayer(_) => {
                            panic!("image layers not supported yet")
                        },
                        tiled::LayerType::GroupLayer(_) => {
                            panic!("image layers not supported yet")
                        }
                    };

                    // let layer_entity = LayerBuilder::<TileBundle>::new_batch(
                    //     &mut commands,
                    //     map_settings.clone(),
                    //     &mut meshes,
                    //     texture_handle.clone(),
                    //     016,
                    //     layer.id() as u16,
                    //     move |mut tile_pos| {
                    //         if tile_pos.0 >= map.width
                    //             || tile_pos.1 >= map.height
                    //         {
                    //             return None;
                    //         }

                    //         if map.orientation == tiled::Orientation::Orthogonal {
                    //             tile_pos.1 = (map.height - 1) as u32 - tile_pos.1;
                    //         }

                    //         let tile = &tile_layer.get_tile(tile_pos.0 as i32,tile_pos.1 as i32).unwrap();

                    //         let tile = Tile {
                    //             texture_index: tile.id() as u16,
                    //             flip_x: tile.flip_h(),
                    //             flip_y: tile.flip_v(),
                    //             flip_d: tile.flip_d(),
                    //             ..Default::default()
                    //         };

                    //         Some(TileBundle {
                    //             tile,
                    //             ..Default::default()
                    //         })
                    //     }
                    // );
                    // commands.entity(layer_entity).insert(Transform::from_xyz(
                    //     layer.offset_y(),
                    //     -layer.offset_x(),
                    //     layer.id() as f32
                    // ));
                    // info!("layer spawned!");
                    // let map_entity = commands.spawn().id();

                    // commands.spawn_bundle(TiledMapBundle {
                    //     map: bevy_ecs_tilemap::Map::new(0u16, map_entity),
                    //     transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    //     ..Default::default()
                    // });
                    // info!("map spawned!");
                    for x in 0..map.width {
                        for y in 0..map.height {
                            let tile = &tile_layer.get_tile(x as i32,y as i32).unwrap();
                            commands.spawn_bundle(SpriteSheetBundle {
                                texture_atlas: texture_atlas_handle.clone(),
                                sprite: TextureAtlasSprite {
                                    index: tile.id() as usize, // TODO: use gid here?
                                    flip_x: tile.flip_h(),
                                    flip_y: tile.flip_v(),
                                    ..Default::default()
                                },
                                transform: Transform::from_translation(Vec3::new(
                                    layer.offset_x() + (x * 32) as f32,
                                    32.0 + layer.offset_y() - (y * 32) as f32,
                                    layer.id() as f32
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
