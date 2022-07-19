// Load a player and an obstacle

use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_sandbox::SandboxPlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .add_plugin(RapierDebugRenderPlugin::default())
        .add_startup_system(bevy_sandbox::simple_figure::default_spawn)
        .add_startup_system(bevy_sandbox::obstacle::spawn)
        .run();
}
