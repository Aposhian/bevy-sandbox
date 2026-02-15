// Load a movable player

use bevy::prelude::*;
use bevy_sandbox::SandboxPlugins;

use bevy_sandbox::simple_figure::default_spawn;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SandboxPlugins)
        .add_systems(Startup, default_spawn)
        .run();
}
