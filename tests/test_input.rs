mod common;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_sandbox::input::{MoveAction, PlayerTag};
use bevy_sandbox::simple_figure::SimpleFigureTag;
use common::TestApp;

/// Spawn a minimal player entity for input tests (no sprite/animation needed).
fn spawn_test_player(app: &mut TestApp) {
    app.app.world_mut().spawn((
        PlayerTag,
        SimpleFigureTag,
        MoveAction::default(),
        Transform::default(),
        LinearVelocity::default(),
        RigidBody::Dynamic,
    ));
}

#[test]
fn w_key_sets_positive_y_velocity() {
    let mut app = TestApp::new();
    app.start_game_no_map();
    spawn_test_player(&mut app);

    app.press_key(KeyCode::KeyW);
    app.tick();

    let world = app.app.world_mut();
    let mut q = world.query_filtered::<&MoveAction, With<PlayerTag>>();
    let move_action = q.iter(world).next().expect("player should exist");
    assert!(
        move_action.desired_velocity.y > 0.0,
        "W key should set positive y velocity, got {:?}",
        move_action.desired_velocity
    );
}

#[test]
fn s_key_sets_negative_y_velocity() {
    let mut app = TestApp::new();
    app.start_game_no_map();
    spawn_test_player(&mut app);

    app.press_key(KeyCode::KeyS);
    app.tick();

    let world = app.app.world_mut();
    let mut q = world.query_filtered::<&MoveAction, With<PlayerTag>>();
    let move_action = q.iter(world).next().expect("player should exist");
    assert!(
        move_action.desired_velocity.y < 0.0,
        "S key should set negative y velocity, got {:?}",
        move_action.desired_velocity
    );
}

#[test]
fn diagonal_input_is_normalized() {
    let mut app = TestApp::new();
    app.start_game_no_map();
    spawn_test_player(&mut app);

    app.press_key(KeyCode::KeyW);
    app.press_key(KeyCode::KeyD);
    app.tick();

    let world = app.app.world_mut();
    let mut q = world.query_filtered::<&MoveAction, With<PlayerTag>>();
    let move_action = q.iter(world).next().expect("player should exist");
    let len = move_action.desired_velocity.length();
    assert!(
        (len - 1.0).abs() < 0.01,
        "diagonal input should be normalized to length ~1.0, got {len}"
    );
}

#[test]
fn move_action_drives_linear_velocity() {
    let mut app = TestApp::new();
    app.start_game_no_map();
    spawn_test_player(&mut app);

    app.press_key(KeyCode::KeyD);
    app.tick();

    let world = app.app.world_mut();
    let mut q = world.query_filtered::<&LinearVelocity, With<PlayerTag>>();
    let vel = q.iter(world).next().expect("player should exist");
    assert!(
        vel.x > 0.0,
        "D key should result in positive x LinearVelocity, got {:?}",
        vel.0
    );
}
