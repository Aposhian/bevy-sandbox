use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use std::path::Path;

use tiled::parse_file;

pub struct TiledPlugin;

impl Plugin for TiledPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_event::<TilemapSpawnEvent>()
            .add_system(spawn.system());
    }
}

pub struct TilemapSpawnEvent {
    pub path: &'static Path
}

/// Spawn entities in response to spawn events
fn spawn(
    mut spawn_events: EventReader<TilemapSpawnEvent>
) {
    for spawn_event in spawn_events.iter() {
        let map = parse_file(spawn_event.path).unwrap();
        println!("{:?}", map);
    }
}