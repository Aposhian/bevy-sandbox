// Load a simple LDTK level

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;

use bevy_sandbox::SandboxPlugins;


fn startup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let handle: Handle<LdtkMap> = asset_server.load("basic.ldtk");

    let map_entity = commands.spawn().id();

    commands.entity(map_entity)
        .insert_bundle(LdtkMapBundle {
            ldtk_map: handle,
            map: Map::new(0u16, map_entity),
            transform: Transform::from_scale(Vec3::splat(2.0)),
            ..Default::default()
        });
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .add_plugin(TilemapPlugin)
        .add_plugin(LdtkPlugin)
        .add_startup_system(startup.system())
        .run();
}
