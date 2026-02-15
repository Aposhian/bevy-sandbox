// Testing tiled integration

use bevy::prelude::*;
use bevy_ecs_tilemap::prelude::*;
use bevy_sandbox::{
    tiled::{TiledPlugin, TilemapSpawnEvent},
    SandboxPlugins,
};
use std::path::Path;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SandboxPlugins)
        .add_plugins((TiledPlugin, TilemapPlugin))
        .add_systems(Startup, spawn_tilemap)
        .run();
}

fn spawn_tilemap(mut tilemap_spawn_event: MessageWriter<TilemapSpawnEvent>) {
    tilemap_spawn_event.write(TilemapSpawnEvent {
        path: Path::new("assets/example.tmx"),
    })
}
