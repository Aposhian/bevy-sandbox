// Load a player and an obstacle

use bevy::prelude::*;
use bevy_sandbox::SandboxPlugins;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .add_startup_system(bevy_sandbox::simple_figure::spawn_playable.system())
        .add_startup_system(bevy_sandbox::obstacle::spawn.system())
        .run();
}