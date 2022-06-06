// Testing tiled integration

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_sandbox::{tiled::{TiledPlugin, TilemapSpawnEvent}, SandboxPlugins};
use std::path::Path;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .add_plugin(TiledPlugin)
        .add_plugin(TilemapPlugin)
        .add_startup_system(spawn_tilemap)
        .run();
}

fn spawn_tilemap(mut tilemap_spawn_event: EventWriter<TilemapSpawnEvent>) {
    tilemap_spawn_event.send(TilemapSpawnEvent {
        path: Path::new("assets/example.tmx"),
    })
}
