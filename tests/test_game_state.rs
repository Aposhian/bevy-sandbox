mod common;

use bevy::prelude::*;
use bevy_sandbox::game_state::GameState;
use common::TestApp;

#[test]
fn app_starts_in_main_menu() {
    let app = TestApp::new();
    assert_eq!(app.game_state(), GameState::MainMenu);
}

#[test]
fn esc_from_playing_transitions_to_paused() {
    let mut app = TestApp::new();
    app.start_game_no_map();
    assert_eq!(app.game_state(), GameState::Playing);

    // Write the key event, then tick to process it. The toggle_pause system
    // sets NextState, which applies on the next tick.
    app.press_key(KeyCode::Escape);
    app.tick(); // toggle_pause sees just_pressed, sets NextState
    app.tick(); // state transition applies
    assert_eq!(app.game_state(), GameState::Paused);
}

#[test]
fn esc_from_paused_transitions_to_playing() {
    let mut app = TestApp::new();
    app.start_game_no_map();

    // Go to Paused
    app.press_key(KeyCode::Escape);
    app.tick();
    app.tick();
    assert_eq!(app.game_state(), GameState::Paused);

    // Release ESC first (key is still held from previous press)
    app.release_key(KeyCode::Escape);
    app.tick();

    // Press ESC again to go back to Playing
    app.press_key(KeyCode::Escape);
    app.tick();
    app.tick();
    assert_eq!(app.game_state(), GameState::Playing);
}

#[test]
fn esc_in_main_menu_stays_in_main_menu() {
    let mut app = TestApp::new();
    assert_eq!(app.game_state(), GameState::MainMenu);

    app.press_key(KeyCode::Escape);
    app.tick();
    app.tick();
    assert_eq!(app.game_state(), GameState::MainMenu);
}
