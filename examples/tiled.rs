// Testing tiled integration

use bevy::prelude::*;
use bevy_sandbox::{tiled::{TiledPlugin, TilemapSpawnEvent}, SandboxPlugins};
use std::path::Path;

use bevy_sandbox::simple_figure::default_spawn;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .add_plugin(TiledPlugin)
        .add_startup_system(default_spawn)
        .add_startup_system(spawn_tilemap)
        .run();
}

fn spawn_tilemap(mut tilemap_spawn_event: EventWriter<TilemapSpawnEvent>) {
    tilemap_spawn_event.send(TilemapSpawnEvent {
        path: Path::new("assets/tiled/maps/example.tmx"),
    })
}
