use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use std::f32::consts::TAU;
use std::path::Path;
use std::sync::Arc;

use tiled::{Loader, ObjectShape, Tileset};

use crate::simple_figure::{GameLayer, SimpleFigureSpawnEvent};

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<TilemapSpawnEvent>()
            .add_systems(Update, (spawn, process_object_layers, add_colliders));
    }
}

#[derive(Component)]
pub struct TiledMapComponent(tiled::Map);

#[derive(Message)]
pub struct TilemapSpawnEvent {
    pub path: &'static Path,
}

fn load_texture(tileset: &Tileset, asset_server: &Res<AssetServer>) -> Option<Handle<Image>> {
    if let Some(image) = &tileset.image {
        let path = image.source.strip_prefix("assets/").unwrap_or(&image.source);
        info!("loading texture: {path:?}");
        let texture_handle = asset_server.load(path.to_path_buf());
        return Some(texture_handle);
    }
    None
}

fn process_layer(
    commands: &mut Commands,
    layer: &tiled::Layer,
    tileset: &Arc<Tileset>,
    texture_handle: &Handle<Image>,
    tiled_map: &tiled::Map,
    tilemap_entity: Entity,
    layer_index: usize,
) {
    info!("loading layer {:?}", layer.id());
    if !layer.visible {
        return;
    }
    info!("layer {:?} is visible", layer.id());

    let tiled::LayerType::Tiles(tile_layer) = layer.layer_type() else {
        return;
    };

    let tiled::TileLayer::Finite(finite_tile_layer) = tile_layer else {
        panic!("infinite tilemaps not supported");
    };

    let map_size = TilemapSize {
        x: tiled_map.width,
        y: tiled_map.height,
    };
    let tile_size = TilemapTileSize {
        x: tileset.tile_width as f32,
        y: tileset.tile_height as f32,
    };
    let grid_size = TilemapGridSize {
        x: tiled_map.tile_width as f32,
        y: tiled_map.tile_height as f32,
    };

    let layer_entity = commands.spawn_empty().id();
    let mut tile_storage = TileStorage::empty(map_size);

    for x in 0..tiled_map.width {
        for y in 0..tiled_map.height {
            // Tiled y=0 is top, Bevy y=0 is bottom
            let tiled_y = if tiled_map.orientation == tiled::Orientation::Orthogonal {
                (tiled_map.height - 1) - y
            } else {
                y
            };

            if let Some(tile) = finite_tile_layer.get_tile(x as i32, tiled_y as i32) {
                let tile_pos = TilePos { x, y };
                let tile_entity = commands
                    .spawn(TileBundle {
                        position: tile_pos,
                        tilemap_id: TilemapId(layer_entity),
                        texture_index: TileTextureIndex(tile.id()),
                        flip: TileFlip {
                            x: tile.flip_h,
                            y: tile.flip_v,
                            d: tile.flip_d,
                        },
                        ..Default::default()
                    })
                    .id();
                tile_storage.set(&tile_pos, tile_entity);
            }
        }
    }

    commands.entity(layer_entity).insert(TilemapBundle {
        grid_size,
        map_type: TilemapType::Square,
        size: map_size,
        storage: tile_storage,
        texture: TilemapTexture::Single(texture_handle.clone()),
        tile_size,
        transform: Transform::from_xyz(
            layer.offset_x,
            -layer.offset_y,
            layer_index as f32,
        ),
        ..Default::default()
    });

    // Associate the layer with the map entity
    commands.entity(tilemap_entity).add_child(layer_entity);
}

/// Spawn entities in response to spawn events
fn spawn(
    mut spawn_events: MessageReader<TilemapSpawnEvent>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    for spawn_event in spawn_events.read() {
        let mut loader = Loader::new();
        let tiled_map = loader.load_tmx_map(spawn_event.path).unwrap();

        let tilemap_entity = commands.spawn_empty().id();

        let tileset = tiled_map.tilesets().first().unwrap();
        let texture_handle = load_texture(tileset, &asset_server).unwrap();

        for (layer_index, layer) in tiled_map.layers().enumerate() {
            process_layer(
                &mut commands,
                &layer,
                tileset,
                &texture_handle,
                &tiled_map,
                tilemap_entity,
                layer_index,
            );
        }

        commands
            .entity(tilemap_entity)
            .insert((
                TiledMapComponent(tiled_map),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
    }
}

fn process_object_layers(
    tiled_map_query: Query<&TiledMapComponent, Changed<TiledMapComponent>>,
    mut spawn_event: MessageWriter<SimpleFigureSpawnEvent>,
) {
    for TiledMapComponent(tiled_map) in tiled_map_query.iter() {
        if let Some(object_layer) = tiled_map.layers().find_map(|layer| {
            match layer.layer_type() {
                tiled::LayerType::Objects(object_layer) => Some(object_layer),
                _ => None,
            }
        }) {
            info!("Found object layer");
            for object in object_layer.objects() {
                if object.user_type.as_str() == "simple_figure" {
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
                        spawn_event.write(SimpleFigureSpawnEvent {
                            playable,
                            position: Vec2::new(object.x, y_pixels),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
}

#[derive(Component, Default)]
pub struct WallTag;

fn add_colliders(
    mut commands: Commands,
    tile_query: Query<(&TilePos, &TileTextureIndex, &TilemapId)>,
    tilemap_transform_query: Query<&Transform, With<TilemapSize>>,
    tiled_map_query: Query<&TiledMapComponent, Changed<TiledMapComponent>>,
) {
    for TiledMapComponent(tiled_map) in tiled_map_query.iter() {
        let Some(tileset) = tiled_map.tilesets().first() else {
            continue;
        };

        // Build a map of tile ID -> collision data
        let mut collider_data: std::collections::HashMap<u32, Vec<_>> = std::collections::HashMap::new();
        for (id, tile) in tileset.tiles() {
            if let Some(object_layer_data) = &tile.collision {
                collider_data.insert(id, object_layer_data.object_data().to_vec());
            }
        }

        // Map grid size (used for tile positioning in bevy_ecs_tilemap)
        let grid_w = tiled_map.tile_width as f32;
        let grid_h = tiled_map.tile_height as f32;

        // Tileset tile size (collision objects are in this coordinate space)
        let tileset_w = tileset.tile_width as f32;
        let tileset_h = tileset.tile_height as f32;

        for (tile_pos, texture_index, tilemap_id) in tile_query.iter() {
            if let Some(objects) = collider_data.get(&texture_index.0) {
                // Get the tilemap layer's local transform for layer offsets
                let tilemap_offset = tilemap_transform_query
                    .get(tilemap_id.0)
                    .map(|t| t.translation.truncate())
                    .unwrap_or(Vec2::ZERO);

                // bevy_ecs_tilemap (TilemapAnchor::None default) places the
                // tile CENTER at (tile_pos * grid_size) in tilemap local space.
                let tile_center_x = tile_pos.x as f32 * grid_w;
                let tile_center_y = tile_pos.y as f32 * grid_h;

                for object in objects {
                    let x_offset = object.x;
                    let y_offset = object.y;
                    match &object.shape {
                        ObjectShape::Rect { width, height } => {
                            // Collision object coords are relative to the
                            // tileset tile's top-left corner (Tiled: y down).
                            // Convert to offset from tile center (Bevy: y up).
                            let rel_x = -tileset_w / 2.0 + x_offset + width / 2.0;
                            let rel_y = tileset_h / 2.0 - y_offset - height / 2.0;

                            let center_x = tilemap_offset.x + tile_center_x + rel_x;
                            let center_y = tilemap_offset.y + tile_center_y + rel_y;

                            let clockwise_rotation = object.rotation.to_radians();
                            let counterclockwise_rotation = TAU - clockwise_rotation;

                            commands.spawn((
                                WallTag,
                                RigidBody::Static,
                                Collider::rectangle(*width, *height),
                                CollisionLayers::new(
                                    LayerMask::from([GameLayer::Wall]),
                                    LayerMask::from([
                                        GameLayer::Character,
                                        GameLayer::Ball,
                                        GameLayer::Wall,
                                    ]),
                                ),
                                Transform::from_translation(Vec3::new(center_x, center_y, 0.0))
                                    .with_rotation(Quat::from_rotation_z(
                                        counterclockwise_rotation,
                                    )),
                            ));
                        }
                        _ => {
                            warn!("Unsupported object shape: {:?}", object.shape);
                        }
                    }
                }
            }
        }
    }
}
