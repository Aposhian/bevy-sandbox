// Testing tiled integration

use bevy::prelude::*;
use bevy_sandbox::{
    tiled::TilemapSpawnEvent,
    SandboxPlugins,
};
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SandboxPlugins)
        .add_systems(Startup, spawn_tilemap)
        .run();
}

fn spawn_tilemap(mut tilemap_spawn_event: MessageWriter<TilemapSpawnEvent>) {
    let _ = tilemap_spawn_event.write(TilemapSpawnEvent {
        path: "assets/example.tmx".to_string(),
        objects_enabled: true,
    });
}
