use bevy::prelude::{App, DefaultPlugins};
use bevy_sandbox::SandboxPlugins;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(SandboxPlugins)
        .run();
}