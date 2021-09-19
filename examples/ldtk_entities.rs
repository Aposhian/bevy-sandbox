// Load a simple LDTK level

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_rapier2d::{na::Isometry2, prelude::*};

use bevy_sandbox::{SandboxPlugins, simple_figure::SimpleFigureSpawnEvent};


fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>
) {
    let handle: Handle<LdtkMap> = asset_server.load("basic.ldtk");

    let map_entity = commands.spawn().id();

    commands.entity(map_entity)
        .insert_bundle(LdtkMapBundle {
            ldtk_map: handle,
            map: Map::new(0u16, map_entity),
            transform: Transform::from_scale(Vec3::splat(1.0)),
            ..Default::default()
        });
}

fn spawn_entities(
    mut commands: Commands,
    mut map_events: EventReader<AssetEvent<LdtkMap>>,
    mut spawn_writer: EventWriter<SimpleFigureSpawnEvent>,
    maps: Res<Assets<LdtkMap>>,
) {
    for event in map_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                info!("Map added!");
                if let Some(map) = maps.get(handle) {
                    // TODO: don't only look at the first level
                    let level = &map.project.levels[0];
        
                    if let Some(layer_instances) = level.layer_instances.as_ref() {
                        layer_instances.iter()
                            .rev()
                            .enumerate()
                            .for_each(|(layer_id, layer)| {
                                layer.entity_instances.iter()
                                    .for_each(|entity| {
                                        match entity.identifier.as_str() {
                                            "SimpleFigure" => {
                                                spawn_writer.send((entity, layer_id as u16).into());
                                            }
                                            _ => {
                                                warn!("Unknown entity: {}", entity.identifier);
                                            }
                                        }
                                    })
                            });
                        layer_instances.iter()
                            .filter(|layer| {
                                layer.layer_instance_type == "IntGrid"
                            })
                            .for_each(|layer| {
                                for y in 0..layer.c_hei {
                                    for x in 0..layer.c_wid {
                                        match layer.int_grid_csv[(y * layer.c_hei + x) as usize] {
                                            2 => {
                                                commands.spawn()
                                                .insert_bundle(ColliderBundle {
                                                    shape: ColliderShape::cuboid(0.5, 0.5),
                                                    position: Isometry2::new(
                                                        [(x * layer.grid_size) as f32 / 32.0, (-y * layer.grid_size) as f32 / 32.0].into(),
                                                        0.0
                                                    ).into(),
                                                    ..Default::default()
                                                });
                                            },
                                            _ => {}
                                        }
                                    }
                                }
                            });
                    }
                }
            }
            AssetEvent::Modified { handle: _ } => {
                info!("Map changed!");
            }
            AssetEvent::Removed { handle: _ } => {
                info!("Map removed!");
            }
        }
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .add_plugin(TilemapPlugin)
        .add_plugin(LdtkPlugin)
        .add_startup_system(load_assets.system())
        .add_system(spawn_entities.system())
        .run();
}
