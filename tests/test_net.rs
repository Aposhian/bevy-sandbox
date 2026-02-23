mod common;

use avian2d::prelude::*;
use bevy::prelude::*;
use bevy_sandbox::input::MoveAction;
use bevy_sandbox::net::{
    ConnectedGuests, GuestIdCounter, GuestInputEvent, GuestTag, LeaveEvent, PauseVotes,
};
use bevy_sandbox::simple_figure::SimpleFigureTag;
use common::TestApp;

#[test]
fn host_tick_increments_each_fixed_update() {
    let mut app = TestApp::new();
    app.setup_host_mode();
    app.start_game_no_map();

    let tick_before = app.host_tick();
    // In headless mode, each app.update() advances real time by the wall-clock
    // delta between updates. We sleep briefly so the FixedUpdate timestep
    // accumulates enough to fire.
    std::thread::sleep(std::time::Duration::from_millis(20));
    app.tick();
    std::thread::sleep(std::time::Duration::from_millis(20));
    app.tick();
    let tick_after = app.host_tick();

    assert!(
        tick_after > tick_before,
        "HostTick should increment: before={tick_before}, after={tick_after}"
    );
}

#[test]
fn pause_votes_all_paused_logic() {
    // Empty guest map, host not paused → not all paused
    let mut votes = PauseVotes::default();
    assert!(!votes.all_paused());

    // Host paused, no guests → all paused
    votes.host_paused = true;
    assert!(votes.all_paused());

    // Host paused, one guest not paused → not all paused
    votes.guest_paused.insert(1, false);
    assert!(!votes.all_paused());

    // Host paused, one guest paused → all paused
    votes.guest_paused.insert(1, true);
    assert!(votes.all_paused());

    // Host paused, two guests, one not paused → not all paused
    votes.guest_paused.insert(2, false);
    assert!(!votes.all_paused());

    // Both guests paused → all paused
    votes.guest_paused.insert(2, true);
    assert!(votes.all_paused());

    // Host un-pauses → not all paused
    votes.host_paused = false;
    assert!(!votes.all_paused());
}

#[test]
fn host_receives_guest_input() {
    let mut app = TestApp::new();
    let channels = app.setup_host_mode();
    app.start_game_no_map();

    let guest_id = 1u32;

    // Spawn a guest entity with GuestTag and MoveAction
    let guest_entity = app
        .app
        .world_mut()
        .spawn((
            GuestTag(guest_id),
            SimpleFigureTag,
            MoveAction::default(),
            Transform::default(),
            LinearVelocity::default(),
        ))
        .id();

    // Register in ConnectedGuests
    app.app
        .world_mut()
        .resource_mut::<ConnectedGuests>()
        .0
        .insert(guest_id, guest_entity);

    // Send input through the channel
    let direction = Vec2::new(1.0, 0.0);
    channels
        .input_tx
        .send(GuestInputEvent {
            guest_id,
            move_direction: direction,
            shoot_direction: None,
            client_tick: 1,
            paused: false,
        })
        .unwrap();

    app.tick();

    // Verify MoveAction was updated
    let move_action = app.app.world().get::<MoveAction>(guest_entity).unwrap();
    assert_eq!(
        move_action.desired_velocity, direction,
        "Guest entity's MoveAction should reflect the input"
    );
}

#[test]
fn host_handles_guest_leave() {
    let mut app = TestApp::new();
    let channels = app.setup_host_mode();
    app.start_game_no_map();

    let guest_id = 1u32;

    // Spawn a guest entity
    let guest_entity = app
        .app
        .world_mut()
        .spawn((
            GuestTag(guest_id),
            SimpleFigureTag,
            MoveAction::default(),
            Transform::default(),
            LinearVelocity::default(),
        ))
        .id();

    app.app
        .world_mut()
        .resource_mut::<ConnectedGuests>()
        .0
        .insert(guest_id, guest_entity);

    // Send leave event
    channels
        .leave_tx
        .send(LeaveEvent { guest_id })
        .unwrap();

    app.tick();

    // Entity should be despawned
    assert!(
        app.app.world().get_entity(guest_entity).is_err(),
        "Guest entity should be despawned after leave"
    );

    // ConnectedGuests should no longer have this guest
    assert!(
        !app.app
            .world()
            .resource::<ConnectedGuests>()
            .0
            .contains_key(&guest_id),
        "Guest should be removed from ConnectedGuests"
    );
}

#[test]
fn guest_id_counter_increments() {
    let counter = GuestIdCounter::default();
    let first = counter.next();
    let second = counter.next();
    let third = counter.next();

    assert_eq!(first, 1);
    assert_eq!(second, 2);
    assert_eq!(third, 3);
}

#[test]
fn host_receives_guest_pause_vote() {
    let mut app = TestApp::new();
    let channels = app.setup_host_mode();
    app.start_game_no_map();

    let guest_id = 1u32;

    // Spawn a guest entity
    let guest_entity = app
        .app
        .world_mut()
        .spawn((
            GuestTag(guest_id),
            SimpleFigureTag,
            MoveAction::default(),
            Transform::default(),
            LinearVelocity::default(),
        ))
        .id();

    app.app
        .world_mut()
        .resource_mut::<ConnectedGuests>()
        .0
        .insert(guest_id, guest_entity);

    // Send input with paused=true
    channels
        .input_tx
        .send(GuestInputEvent {
            guest_id,
            move_direction: Vec2::ZERO,
            shoot_direction: None,
            client_tick: 1,
            paused: true,
        })
        .unwrap();

    app.tick();

    let votes = app.app.world().resource::<PauseVotes>();
    assert_eq!(
        votes.guest_paused.get(&guest_id),
        Some(&true),
        "Guest pause vote should be recorded"
    );
}
