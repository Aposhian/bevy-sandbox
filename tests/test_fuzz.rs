mod common;

use bevy::prelude::*;
use bevy_sandbox::game_state::GameState;
use bevy_sandbox::net::GuestInputEvent;
use common::TestApp;
use rand::prelude::*;

const FUZZ_ITERATIONS: usize = 100;
const FUZZ_SEED: u64 = 42;

/// All keys that the game handles — WASD, arrows, Escape, Space.
const ALL_KEYS: &[KeyCode] = &[
    KeyCode::KeyW,
    KeyCode::KeyA,
    KeyCode::KeyS,
    KeyCode::KeyD,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::Escape,
    KeyCode::Space,
];

#[test]
fn fuzz_random_key_sequences() {
    let mut rng = StdRng::seed_from_u64(FUZZ_SEED);
    let mut app = TestApp::new();
    app.start_game_no_map();

    for _ in 0..FUZZ_ITERATIONS {
        // Press 1–3 random keys
        let num_keys = rng.random_range(1..=3);
        let mut pressed_keys = Vec::new();
        for _ in 0..num_keys {
            let key = ALL_KEYS[rng.random_range(0..ALL_KEYS.len())];
            app.press_key(key);
            pressed_keys.push(key);
        }

        // Tick a few times
        let ticks = rng.random_range(1..=5);
        app.tick_n(ticks);

        // Release the keys
        for key in &pressed_keys {
            app.release_key(*key);
        }
        app.tick();
    }
    // If we got here without panic, the test passes.
}

#[test]
fn fuzz_random_game_state_transitions() {
    let mut rng = StdRng::seed_from_u64(FUZZ_SEED + 1);
    let mut app = TestApp::new();
    app.start_game_no_map();

    for _ in 0..FUZZ_ITERATIONS {
        // Randomly press Escape to toggle pause
        if rng.random_bool(0.5) {
            app.press_key(KeyCode::Escape);
            app.tick();
            app.tick();
            app.release_key(KeyCode::Escape);
            app.tick();
        } else {
            app.tick();
        }

        let state = app.game_state();
        assert!(
            matches!(state, GameState::Playing | GameState::Paused),
            "State should be Playing or Paused, got {state:?}"
        );
    }
}

#[test]
fn fuzz_random_guest_input_events() {
    let mut rng = StdRng::seed_from_u64(FUZZ_SEED + 2);
    let mut app = TestApp::new();
    let channels = app.setup_host_mode();
    app.start_game_no_map();

    for _ in 0..FUZZ_ITERATIONS {
        let guest_id = rng.random_range(1..=10);
        let direction = Vec2::new(
            rng.random_range(-1.0f32..=1.0),
            rng.random_range(-1.0f32..=1.0),
        );
        let shoot = if rng.random_bool(0.3) {
            Some(Vec2::new(
                rng.random_range(-1.0f32..=1.0),
                rng.random_range(-1.0f32..=1.0),
            ))
        } else {
            None
        };
        let paused = rng.random_bool(0.2);

        channels
            .input_tx
            .send(GuestInputEvent {
                guest_id,
                move_direction: direction,
                shoot_direction: shoot,
                client_tick: rng.random_range(0..1000),
                paused,
            })
            .unwrap();

        app.tick();
    }
    // No panic = pass
}

#[test]
fn fuzz_random_world_updates() {
    use bevy_sandbox::net::proto;

    let mut rng = StdRng::seed_from_u64(FUZZ_SEED + 3);
    let mut app = TestApp::new();

    // Set up as guest mode by inserting the required resources
    let (update_tx, update_rx) = crossbeam_channel::unbounded::<proto::WorldUpdate>();
    let (input_tx, _input_rx) = tokio::sync::mpsc::channel::<proto::GuestInput>(64);
    app.app
        .world_mut()
        .insert_resource(bevy_sandbox::net::GuestChannels { update_rx, input_tx });
    app.app
        .world_mut()
        .insert_resource(bevy_sandbox::net::LocalGuestId {
            guest_id: 1,
            entity_id: 999,
        });
    app.app
        .world_mut()
        .insert_resource(bevy_sandbox::net::guest::EntityMap::default());
    app.app
        .world_mut()
        .insert_resource(bevy_sandbox::net::NetworkRole::Guest {
            addr: "test".to_string(),
        });
    app.start_game_no_map();

    for _ in 0..FUZZ_ITERATIONS {
        let num_entities = rng.random_range(0..=5);
        let entities: Vec<proto::EntityState> = (0..num_entities)
            .map(|_| proto::EntityState {
                entity_id: rng.random_range(1..=1000),
                position: Some(proto::Vec2 {
                    x: rng.random_range(-1000.0f32..=1000.0),
                    y: rng.random_range(-1000.0f32..=1000.0),
                }),
                velocity: Some(proto::Vec2 {
                    x: rng.random_range(-100.0f32..=100.0),
                    y: rng.random_range(-100.0f32..=100.0),
                }),
                health_max: rng.random_range(0..=100),
                health_current: rng.random_range(0..=100),
                kind: rng.random_range(0..=4),
                guest_id: rng.random_range(0..=10),
            })
            .collect();

        let num_despawns = rng.random_range(0..=3);
        let despawned: Vec<u64> = (0..num_despawns)
            .map(|_| rng.random_range(1..=1000))
            .collect();

        let update = proto::WorldUpdate {
            host_tick: rng.random_range(1..=10000),
            timestamp_us: 0,
            entities,
            despawned,
            all_paused: rng.random_bool(0.2),
        };

        update_tx.send(update).unwrap();
        app.tick();
    }
    // No panic = pass
}
