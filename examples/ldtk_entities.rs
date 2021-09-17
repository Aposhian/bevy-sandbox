// Load a simple LDTK level

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

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
    mut map_events: EventReader<AssetEvent<LdtkMap>>,
    mut spawn_writer: EventWriter<SimpleFigureSpawnEvent>,
    maps: Res<Assets<LdtkMap>>,
) {
    for event in map_events.iter() {
        match event {
            AssetEvent::Created { handle } => {
                info!("Map added!");
                if let Some(map) = maps.get(handle) {
                    let level = &map.project.levels[0];
        
                    for entity in level.layer_instances.as_ref().unwrap()[0].entity_instances.iter() {
                        match entity.identifier.as_str() {
                            "SimpleFigure" => {
                                spawn_writer.send(entity.into());
                            }
                            _ => {
                                warn!("Unknown entity: {}", entity.identifier);
                            }
                        }
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
