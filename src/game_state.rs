use avian2d::prelude::*;
use bevy::prelude::*;

pub struct GameStatePlugin;

#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameState {
    #[default]
    MainMenu,
    Playing,
    Paused,
}

impl Plugin for GameStatePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<GameState>()
            .add_systems(Update, toggle_pause)
            .add_systems(OnEnter(GameState::Paused), pause_physics)
            .add_systems(OnEnter(GameState::Playing), unpause_physics);
    }
}

fn toggle_pause(
    keyboard: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        match state.get() {
            GameState::Playing => next_state.set(GameState::Paused),
            GameState::Paused => next_state.set(GameState::Playing),
            GameState::MainMenu => {} // ignore ESC on main menu
        }
    }
}

fn pause_physics(mut time: ResMut<Time<Physics>>) {
    time.pause();
}

fn unpause_physics(mut time: ResMut<Time<Physics>>) {
    time.unpause();
}
