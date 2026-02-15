// Load a player and an obstacle

use bevy::prelude::*;
use bevy_sandbox::SandboxPlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SandboxPlugins)
        .add_systems(Startup, (
            bevy_sandbox::simple_figure::default_spawn,
            bevy_sandbox::obstacle::spawn,
        ))
        .run();
}
