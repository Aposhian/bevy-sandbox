use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::Path;
use std::sync::Arc;

use tiled::{Loader, ObjectShape, Tileset};

use crate::simple_figure::{GameLayer, SimpleFigureSpawnEvent};

pub struct TiledPlugin;

/// When this resource is present, `process_object_layers` skips spawning
/// objects from the Tiled map (used during load to avoid duplicate spawns).
#[derive(Resource)]
pub struct SuppressObjectSpawn;

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
    pub path: String,
}

fn load_texture(tileset: &Tileset, asset_server: &Res<AssetServer>) -> Option<Handle<Image>> {
    if let Some(image) = &tileset.image {
        let path = image
            .source
            .strip_prefix("assets/")
            .unwrap_or(&image.source);
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
        transform: Transform::from_xyz(layer.offset_x, -layer.offset_y, layer_index as f32),
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
        let tiled_map = loader.load_tmx_map(Path::new(&spawn_event.path)).unwrap();

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

        commands.entity(tilemap_entity).insert((
            TiledMapComponent(tiled_map),
            Transform::from_xyz(0.0, 0.0, 0.0),
        ));
    }
}

fn process_object_layers(
    suppress: Option<Res<SuppressObjectSpawn>>,
    tiled_map_query: Query<&TiledMapComponent, Changed<TiledMapComponent>>,
    mut spawn_event: MessageWriter<SimpleFigureSpawnEvent>,
) {
    if suppress.is_some() {
        return;
    }
    for TiledMapComponent(tiled_map) in tiled_map_query.iter() {
        if let Some(object_layer) = tiled_map
            .layers()
            .find_map(|layer| match layer.layer_type() {
                tiled::LayerType::Objects(object_layer) => Some(object_layer),
                _ => None,
            })
        {
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

/// A world-space axis-aligned rectangle, stored by center and half-extents.
struct WorldRect {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

/// Merge a list of axis-aligned rectangles by joining touching neighbours.
///
/// Pass 1 — horizontal: group by (y, h), sort by x, join rects whose right
/// edge meets the next rect's left edge.
///
/// Pass 2 — vertical: group by (x, w), sort by y, join rects whose top edge
/// meets the next rect's bottom edge.
fn merge_rects(mut rects: Vec<WorldRect>) -> Vec<WorldRect> {
    // Quantise a float to an i64 key (0.001-pixel precision).
    fn q(v: f32) -> i64 {
        (v * 1000.0).round() as i64
    }

    // --- Pass 1: horizontal ---
    rects.sort_unstable_by_key(|r| (q(r.y), q(r.h), q(r.x)));

    let mut pass1: Vec<WorldRect> = Vec::with_capacity(rects.len());
    let mut it = rects.into_iter();
    if let Some(first) = it.next() {
        let mut cur = first;
        for next in it {
            let same_row = q(cur.y) == q(next.y) && q(cur.h) == q(next.h);
            let touching = (cur.x + cur.w / 2.0 - (next.x - next.w / 2.0)).abs() < 0.1;
            if same_row && touching {
                let lo = cur.x - cur.w / 2.0;
                let hi = next.x + next.w / 2.0;
                cur.w = hi - lo;
                cur.x = (lo + hi) / 2.0;
            } else {
                pass1.push(cur);
                cur = next;
            }
        }
        pass1.push(cur);
    }

    // --- Pass 2: vertical ---
    pass1.sort_unstable_by_key(|r| (q(r.x), q(r.w), q(r.y)));

    let mut pass2: Vec<WorldRect> = Vec::with_capacity(pass1.len());
    let mut it = pass1.into_iter();
    if let Some(first) = it.next() {
        let mut cur = first;
        for next in it {
            let same_col = q(cur.x) == q(next.x) && q(cur.w) == q(next.w);
            let touching = (cur.y + cur.h / 2.0 - (next.y - next.h / 2.0)).abs() < 0.1;
            if same_col && touching {
                let lo = cur.y - cur.h / 2.0;
                let hi = next.y + next.h / 2.0;
                cur.h = hi - lo;
                cur.y = (lo + hi) / 2.0;
            } else {
                pass2.push(cur);
                cur = next;
            }
        }
        pass2.push(cur);
    }

    pass2
}

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

        // Build a map of tile ID -> collision objects.
        let mut collider_data: HashMap<u32, Vec<_>> = HashMap::new();
        for (id, tile) in tileset.tiles() {
            if let Some(col) = &tile.collision {
                collider_data.insert(id, col.object_data().to_vec());
            }
        }

        let grid_w = tiled_map.tile_width as f32;
        let grid_h = tiled_map.tile_height as f32;
        let tileset_w = tileset.tile_width as f32;
        let tileset_h = tileset.tile_height as f32;

        // Collect all world-space rects, split by whether they are axis-aligned.
        let mut axis_aligned: Vec<WorldRect> = Vec::new();
        // (rect, counter-clockwise rotation in radians)
        let mut rotated: Vec<(WorldRect, f32)> = Vec::new();

        for (tile_pos, texture_index, tilemap_id) in tile_query.iter() {
            let Some(objects) = collider_data.get(&texture_index.0) else {
                continue;
            };

            let tilemap_offset = tilemap_transform_query
                .get(tilemap_id.0)
                .map(|t| t.translation.truncate())
                .unwrap_or(Vec2::ZERO);

            // bevy_ecs_tilemap places the tile CENTER at (pos * grid_size).
            let tile_cx = tilemap_offset.x + tile_pos.x as f32 * grid_w;
            let tile_cy = tilemap_offset.y + tile_pos.y as f32 * grid_h;

            for object in objects {
                let ObjectShape::Rect { width, height } = &object.shape else {
                    warn!("Unsupported object shape: {:?}", object.shape);
                    continue;
                };

                // Collision object coords are relative to the tile's top-left
                // corner (Tiled: y down). Convert to offset from tile center
                // (Bevy: y up).
                let rel_x = -tileset_w / 2.0 + object.x + width / 2.0;
                let rel_y = tileset_h / 2.0 - object.y - height / 2.0;

                let rect = WorldRect {
                    x: tile_cx + rel_x,
                    y: tile_cy + rel_y,
                    w: *width,
                    h: *height,
                };

                let cw_rot = object.rotation.to_radians();
                if cw_rot.abs() < 1e-4 {
                    axis_aligned.push(rect);
                } else {
                    rotated.push((rect, TAU - cw_rot));
                }
            }
        }

        let raw = axis_aligned.len() + rotated.len();
        let merged = merge_rects(axis_aligned);
        info!(
            "Wall colliders: {} raw → {} after merge ({} axis-aligned → {})",
            raw,
            merged.len() + rotated.len(),
            raw - rotated.len(),
            merged.len(),
        );

        let layers = || {
            CollisionLayers::new(
                LayerMask::from([GameLayer::Wall]),
                LayerMask::from([GameLayer::Character, GameLayer::Ball, GameLayer::Wall]),
            )
        };

        for rect in merged {
            commands.spawn((
                WallTag,
                RigidBody::Static,
                Collider::rectangle(rect.w, rect.h),
                layers(),
                Transform::from_translation(Vec3::new(rect.x, rect.y, 0.0)),
            ));
        }

        for (rect, ccw_rot) in rotated {
            commands.spawn((
                WallTag,
                RigidBody::Static,
                Collider::rectangle(rect.w, rect.h),
                layers(),
                Transform::from_translation(Vec3::new(rect.x, rect.y, 0.0))
                    .with_rotation(Quat::from_rotation_z(ccw_rot)),
            ));
        }
    }
}
