use bevy::prelude::*;

mod core;
mod input;
mod tie_man;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(core::setup::setup.system())
        .add_plugins(core::plugins::CorePlugin)
        .add_plugins(tie_man::plugins::PlayerPlugin)
        .run();
}