use bevy::{prelude::*, render::render_resource::TextureUsages};
use bevy_ecs_tilemap::prelude::*;
use std::{path::Path, sync::Arc};

use tiled::{Tileset, Loader};

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_event::<TilemapSpawnEvent>()
            .add_system(spawn)
            .add_system(set_texture_filters_to_nearest);
    }
}

#[derive(Component)]
pub struct TiledMapComponent(tiled::Map);

#[derive(Bundle)]
pub struct TiledMapBundle {
    pub ecs_map: bevy_ecs_tilemap::Map,
    pub tiled_map: TiledMapComponent,
    pub transform: Transform,
}

pub struct TilemapSpawnEvent {
    pub path: &'static Path,
}

pub fn set_texture_filters_to_nearest(
    mut texture_events: EventReader<AssetEvent<Image>>,
    mut textures: ResMut<Assets<Image>>,
) {
    // quick and dirty, run this for all textures anytime a texture is created.
    for event in texture_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                if let Some(mut texture) = textures.get_mut(handle) {
                    texture.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
                        | TextureUsages::COPY_SRC
                        | TextureUsages::COPY_DST;
                }
            }
            _ => (),
        }
    }
}


fn load_texture_atlas(
    tileset: &Tileset,
    asset_server: &Res<AssetServer>
) -> Option<Handle<Image>> {
    if let Some(image) = &tileset.image {
        let path = std::fs::canonicalize(&image.source).unwrap();
        info!("loading texture: {path:?}");
        let texture_handle = asset_server.load(path);
        return Some(texture_handle);
    }
    None
}

fn process_layer(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    layer: &tiled::Layer,
    tileset: &Arc<Tileset>,
    texture_handle: &Handle<Image>,
    tiled_map: &tiled::Map,
    ecs_map: &mut bevy_ecs_tilemap::Map
) {
    info!("loading layer {:?}", layer.id());
    if layer.visible {
        info!("layer {:?} is visible", layer.id());
        const CHUNK_SIZE: u32 = 256;

        let mut layer_settings = LayerSettings::new(
            MapSize(
                (tiled_map.width as f32 / CHUNK_SIZE as f32).ceil() as u32,
                (tiled_map.height as f32 / CHUNK_SIZE as f32).ceil() as u32
            ),
            ChunkSize(CHUNK_SIZE, CHUNK_SIZE),
            TileSize(tileset.tile_width as f32, tileset.tile_height as f32),
            // TODO: don't unwrap this
            TextureSize(
                tileset.image.clone().unwrap().width as f32,
                tileset.image.clone().unwrap().height as f32,
            ),
        );
        layer_settings.grid_size =
            Vec2::new(tiled_map.tile_width as f32, tiled_map.tile_height as f32);
            layer_settings.mesh_type = TilemapMeshType::Square;

        let layer_type = layer.layer_type();
        let tile_layer = match layer_type {
            tiled::LayerType::TileLayer(layer) => match layer {
                tiled::TileLayer::Finite(data) => data,
                tiled::TileLayer::Infinite(_) => {
                    panic!("infinite tilemaps not supported");
                }
            },
            tiled::LayerType::ObjectLayer(_) => {
                panic!("object layers not supported yet")
            }
            tiled::LayerType::ImageLayer(_) => {
                panic!("image layers not supported yet")
            }
            tiled::LayerType::GroupLayer(_) => {
                panic!("image layers not supported yet")
            }
        };


        let layer_entity = LayerBuilder::<TileBundle>::new_batch(
            commands,
            layer_settings.clone(),
            meshes,
            texture_handle.clone(),
            0u16,
            layer.id() as u16,
            |mut tile_pos| {
                if tile_pos.0 >= tiled_map.width || tile_pos.1 >= tiled_map.height {
                    return None;
                }

                if tiled_map.orientation == tiled::Orientation::Orthogonal {
                    tile_pos.1 = (tiled_map.height - 1) as u32 - tile_pos.1;
                }

                let tile = &tile_layer
                    .get_tile(tile_pos.0 as i32, tile_pos.1 as i32)
                    .unwrap();

                let tile = Tile {
                    texture_index: tile.id() as u16,
                    flip_x: tile.flip_h,
                    flip_y: tile.flip_v,
                    flip_d: tile.flip_d,
                    ..Default::default()
                };

                Some(TileBundle {
                    tile,
                    ..Default::default()
                })
            },
        );

        ecs_map.add_layer(commands, layer.id() as u16, layer_entity);
        commands.entity(layer_entity).insert(Transform::from_xyz(
            layer.offset_y,
            -layer.offset_x,
            layer.id() as f32,
        ));
    }
}

/// Spawn entities in response to spawn events
fn spawn(
    mut spawn_events: EventReader<TilemapSpawnEvent>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>
) {
    for spawn_event in spawn_events.iter() {
        let mut loader = Loader::new();
        let tiled_map = loader.load_tmx_map(spawn_event.path).unwrap();

        let map_entity = commands.spawn().id();
        let mut ecs_map = bevy_ecs_tilemap::Map::new(0u16, map_entity);

        let tileset = tiled_map.tilesets().first().unwrap();
        // TODO: make this handle multiple textures
        let texture_handle =
            load_texture_atlas(&tileset, &asset_server).unwrap();

        for layer in tiled_map.layers() {
            process_layer(
                &mut commands,
                &mut meshes,
                &layer,
                &tileset,
                &texture_handle,
                &tiled_map,
                &mut ecs_map
            );
        }
        commands.spawn_bundle(TiledMapBundle {
            ecs_map,
            tiled_map: TiledMapComponent(tiled_map),
            transform: Transform::from_xyz(0.0, 0.0, 0.0)
        });
    }
}

// fn load_tilemap_colliders(
//     mut commands: Commands,
//     mut tile_query: Query<&mut Tile>,
//     mut map_query: MapQuery
// ) {
//     for x in 0..0 {
//         for y in 0..0 {
//             if let Ok(tile) = map_query.get_tile_entity(pos, 016, 1) {
                
//             }
//         }
//     }
// }