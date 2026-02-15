// Render an empty window

use bevy::prelude::*;
use bevy_sandbox::SandboxPlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_plugins(SandboxPlugins)
        .run();
}
