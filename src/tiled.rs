use bevy::{prelude::*, render::render_resource::TextureUsages};
use bevy_ecs_tilemap::prelude::*;
use bevy_rapier2d::prelude::*;
use nalgebra::{Isometry2, UnitComplex};
use std::f32::consts::TAU;
use std::{path::Path, sync::Arc};

use tiled::{Loader, ObjectShape, Tileset};

use crate::simple_figure::SimpleFigureSpawnEvent;

// TODO: change this from a constant so we can handle multiple maps
const MAP_ID: u16 = 0u16;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TilemapSpawnEvent>()
            // .add_plugin(RapierRenderPlugin)
            .add_system(spawn)
            .add_system(set_texture_filters_to_nearest)
            .add_system(process_object_layers)
            .add_system(add_colliders);
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

fn load_texture_atlas(tileset: &Tileset, asset_server: &Res<AssetServer>) -> Option<Handle<Image>> {
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
    ecs_map: &mut bevy_ecs_tilemap::Map,
) {
    info!("loading layer {:?}", layer.id());
    if layer.visible {
        info!("layer {:?} is visible", layer.id());
        const CHUNK_SIZE: u32 = 256;

        let mut layer_settings = LayerSettings::new(
            MapSize(
                (tiled_map.width as f32 / CHUNK_SIZE as f32).ceil() as u32,
                (tiled_map.height as f32 / CHUNK_SIZE as f32).ceil() as u32,
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

        if let tiled::LayerType::Tiles(tile_layer) = layer.layer_type() {
            let finite_tile_layer = match tile_layer {
                tiled::TileLayer::Finite(data) => data,
                tiled::TileLayer::Infinite(_) => {
                    panic!("infinite tilemaps not supported");
                }
            };

            let layer_entity = LayerBuilder::<TileBundle>::new_batch(
                commands,
                layer_settings.clone(),
                meshes,
                texture_handle.clone(),
                MAP_ID,
                layer.id() as u16,
                |mut tile_pos| {
                    if tile_pos.0 >= tiled_map.width || tile_pos.1 >= tiled_map.height {
                        return None;
                    }

                    if tiled_map.orientation == tiled::Orientation::Orthogonal {
                        tile_pos.1 = (tiled_map.height - 1) as u32 - tile_pos.1;
                    }

                    let tile = &finite_tile_layer
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
        };
    }
}

/// Spawn entities in response to spawn events
fn spawn(
    mut spawn_events: EventReader<TilemapSpawnEvent>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for spawn_event in spawn_events.iter() {
        let mut loader = Loader::new();
        let tiled_map = loader.load_tmx_map(spawn_event.path).unwrap();

        let map_entity = commands.spawn().id();
        let mut ecs_map = bevy_ecs_tilemap::Map::new(MAP_ID, map_entity);

        let tileset = tiled_map.tilesets().first().unwrap();
        // TODO: make this handle multiple textures
        let texture_handle = load_texture_atlas(&tileset, &asset_server).unwrap();

        for layer in tiled_map.layers() {
            process_layer(
                &mut commands,
                &mut meshes,
                &layer,
                &tileset,
                &texture_handle,
                &tiled_map,
                &mut ecs_map,
            );
        }
        commands.spawn_bundle(TiledMapBundle {
            ecs_map,
            tiled_map: TiledMapComponent(tiled_map),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
        });
    }
}

fn process_object_layers(
    tiled_map_query: Query<&TiledMapComponent, Changed<TiledMapComponent>>,
    mut spawn_event: EventWriter<SimpleFigureSpawnEvent>,
    rc: Res<RapierConfiguration>,
) {
    for TiledMapComponent(tiled_map) in tiled_map_query.iter() {
        if let Some(object_layer) = tiled_map.layers().find_map(|layer| {
            return match layer.layer_type() {
                tiled::LayerType::Objects(object_layer) => Some(object_layer),
                _ => None,
            };
        }) {
            info!("Found object layer");
            for object in object_layer.objects() {
                if object.obj_type.as_str() == "simple_figure" {
                    let y_pixels = (tiled_map.height * tiled_map.tile_height) as f32 - object.y;

                    if let ObjectShape::Rect {
                        width: _,
                        height: _,
                    } = object.shape
                    {
                        let playable = match object
                            .properties
                            .get("playable")
                            .unwrap_or(&tiled::PropertyValue::BoolValue(true))
                        {
                            tiled::PropertyValue::BoolValue(playable) => *playable,
                            _ => false,
                        };
                        spawn_event.send(SimpleFigureSpawnEvent {
                            playable,
                            position: Isometry2::new(
                                [object.x / rc.scale, y_pixels / rc.scale].into(),
                                0.0,
                            ),
                            ..Default::default()
                        })
                    }
                }
            }
        }
    }
}

#[derive(Component, Default)]
pub struct WallTag;

#[derive(Bundle, Default)]
pub struct WallColliderBundle {
    #[bundle]
    rigid_body_bundle: RigidBodyBundle,
    #[bundle]
    collider_bundle: ColliderBundle,
    wall_tag: WallTag,
}

fn add_colliders(
    rc: Res<RapierConfiguration>,
    mut commands: Commands,
    tile_query: Query<&Tile>,
    mut map_query: MapQuery,
    tiled_map_query: Query<&TiledMapComponent, Changed<TiledMapComponent>>,
) {
    for TiledMapComponent(tiled_map) in tiled_map_query.iter() {
        let mut collider_spawners = std::collections::HashMap::new();
        if let Some(tileset) = tiled_map.tilesets().first() {
            for (id, tile) in tileset.tiles() {
                if let Some(object_layer_data) = &tile.collision {
                    // Clone these so we can just move them into the closure
                    let object_layer_data = object_layer_data.clone();
                    let physics_scale = rc.scale;
                    collider_spawners.insert(
                        id,
                        move |commands: &mut Commands, column: u32, row: u32| -> Vec<Entity> {
                            object_layer_data
                                .object_data()
                                .iter()
                                .filter_map(|object| {
                                    let physics_tile_height =
                                        tiled_map.tile_height as f32 / physics_scale;

                                    let x = (column * tiled_map.tile_width) as f32 / physics_scale;
                                    let y = (row * tiled_map.tile_height) as f32 / physics_scale;
                                    let x_offset = object.x / physics_scale;
                                    let y_offset = object.y / physics_scale;
                                    match &object.shape {
                                        ObjectShape::Rect { width, height } => {
                                            let physics_width = width / physics_scale;
                                            let physics_height = height / physics_scale;
                                            // The collider position is measured from the center in rapier,
                                            // but in tiled it is from the top-left corner.
                                            // In rapier2d, y increases up, but in tiled, y increases down
                                            let mut collider_position = Isometry2::new(
                                                [
                                                    physics_width / 2.0 + x_offset,
                                                    -physics_height / 2.0 - y_offset,
                                                ]
                                                .into(),
                                                0.0,
                                            );
                                            // Tiled rotates about the top-left corner
                                            let clockwise_rotation = object.rotation.to_radians();
                                            let counterclockwise_rotation =
                                                TAU - clockwise_rotation;

                                            // This needs to rotate about the top-left corner
                                            collider_position.append_rotation_wrt_point_mut(
                                                &UnitComplex::new(counterclockwise_rotation),
                                                &Point::new(x_offset, -y_offset),
                                            );
                                            Some(
                                                commands
                                                    .spawn_bundle(WallColliderBundle {
                                                        rigid_body_bundle: RigidBodyBundle {
                                                            body_type: RigidBodyTypeComponent(
                                                                RigidBodyType::Static,
                                                            ),
                                                            // Use top-left corner instead of bottom-left corner
                                                            position: Isometry2::new(
                                                                [x, y + physics_tile_height].into(),
                                                                0.0,
                                                            )
                                                            .into(),
                                                            ..Default::default()
                                                        },
                                                        collider_bundle: ColliderBundle {
                                                            // Convert dimensions into half-extants
                                                            shape: ColliderShape::cuboid(
                                                                physics_width / 2.0,
                                                                physics_height / 2.0,
                                                            )
                                                            .into(),
                                                            position: collider_position.into(),
                                                            ..Default::default()
                                                        },
                                                        ..Default::default()
                                                    })
                                                    .insert(ColliderDebugRender::with_id(
                                                        id as usize,
                                                    ))
                                                    .insert(ColliderPositionSync::Discrete)
                                                    .id(),
                                            )
                                        }
                                        _ => {
                                            warn!("Unsupported object shape: {:?}", object.shape);
                                            None
                                        }
                                    }
                                })
                                .collect()
                        },
                    );
                } else {
                    warn!("No collision data for tile id: {id}");
                }
            }
        }

        for x in 0..tiled_map.width {
            for y in 0..tiled_map.height {
                if let Ok(tile_entity) = map_query.get_tile_entity(TilePos(x, y), MAP_ID, 1) {
                    if let Ok(tile) = tile_query.get(tile_entity) {
                        if let Some(spawner) = collider_spawners.get(&(tile.texture_index as u32)) {
                            let object_entities = spawner(&mut commands, x, y);
                            commands
                                .entity(tile_entity)
                                .push_children(object_entities.as_slice());
                        }
                    }
                }
            }
        }
    }
}
