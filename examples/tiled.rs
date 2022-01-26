// Testing tiled integration

use bevy::prelude::*;
use bevy_sandbox::tiled::{TiledPlugin, TilemapSpawnEvent};
use std::path::Path;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(TiledPlugin)
        .add_startup_system(spawn_tilemap)
        .run();
}

fn spawn_tilemap(mut tilemap_spawn_event: EventWriter<TilemapSpawnEvent>) {
    tilemap_spawn_event.send(TilemapSpawnEvent {
        path: Path::new("assets/tiled/maps/open.tmx"),
    })
}
