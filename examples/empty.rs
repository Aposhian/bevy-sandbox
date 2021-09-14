// Render an empty window

use bevy::prelude::*;
use bevy_sandbox::SandboxPlugins;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .run();
}