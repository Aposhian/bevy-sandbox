// Load a movable player

use bevy::prelude::*;
use bevy_sandbox::SandboxPlugins;

use bevy_sandbox::simple_figure::spawn_playable;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .add_startup_system(spawn_playable.system())
        .run();
}