use avian2d::prelude::*;
use bevy::prelude::*;

use crate::net::{NetworkRole, PauseVotes};

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
            .add_systems(OnEnter(GameState::Paused), on_enter_paused)
            .add_systems(OnEnter(GameState::Playing), on_enter_playing)
            .add_systems(
                Update,
                sync_physics_pause.run_if(not(in_state(GameState::MainMenu))),
            );
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

/// When entering the Paused state, update the host's pause vote.
fn on_enter_paused(
    role: Res<NetworkRole>,
    mut pause_votes: ResMut<PauseVotes>,
    mut time: ResMut<Time<Physics>>,
) {
    match *role {
        NetworkRole::Offline => {
            // Single player: pause immediately
            time.pause();
            info!("Paused game (single-player)");
        }
        NetworkRole::Host { .. } => {
            pause_votes.host_paused = true;
            // Physics pause is handled by sync_physics_pause
            info!("Paused game (host)");
        }
        NetworkRole::Guest { .. } => {
            // Guest pause state is sent via GuestInput; guest doesn't control physics
            info!("Paused game (guest)");
        }
    }
}

/// When entering the Playing state, update the host's pause vote.
fn on_enter_playing(
    role: Res<NetworkRole>,
    mut pause_votes: ResMut<PauseVotes>,
    mut time: ResMut<Time<Physics>>,
) {
    match *role {
        NetworkRole::Offline => {
            // Single player: unpause immediately
            time.unpause();
            info!("Unpaused game (single-player)");
        }
        NetworkRole::Host { .. } => {
            pause_votes.host_paused = false;
            // Physics unpause is handled by sync_physics_pause
            info!("Unpaused game (host)");
        }
        NetworkRole::Guest { .. } => {
            // Guest pause state is sent via GuestInput
            info!("Unpaused game (guest)");
        }
    }
}

/// For multiplayer host: pause/unpause physics based on whether ALL players have paused.
fn sync_physics_pause(
    role: Res<NetworkRole>,
    pause_votes: Res<PauseVotes>,
    mut time: ResMut<Time<Physics>>,
) {
    if !matches!(*role, NetworkRole::Host { .. }) {
        return;
    }

    if pause_votes.all_paused() {
        debug!(
            "All players have paused; pausing physics. {:?}",
            pause_votes
        );
        time.pause();
    } else {
        debug!(
            "Not all players have paused; unpausing physics. {:?}",
            pause_votes
        );
        time.unpause();
    }
}
