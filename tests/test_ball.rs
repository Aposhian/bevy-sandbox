mod common;

use bevy::prelude::*;
use bevy_sandbox::ball::{BallSpawnEvent, BallTag};
use common::TestApp;

#[test]
fn ball_spawn_event_creates_ball_entity() {
    let mut app = TestApp::new();
    app.start_game_no_map();

    assert_eq!(app.count::<BallTag>(), 0, "no balls initially");

    app.app.world_mut().write_message(BallSpawnEvent {
        position: Vec2::new(100.0, 200.0),
        velocity: Vec2::new(10.0, 0.0),
    });

    app.tick();

    assert_eq!(
        app.count::<BallTag>(),
        1,
        "BallSpawnEvent should create one ball entity"
    );
}
