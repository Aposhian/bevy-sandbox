// Load a movable player

use bevy::prelude::*;
use bevy_sandbox::SandboxPlugins;

use bevy_sandbox::simple_figure::default_spawn;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .add_startup_system(default_spawn)
        .run();
}
