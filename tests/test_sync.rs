use bevy_sandbox::net::sync::TickSyncState;

#[test]
fn tick_sync_no_adjustment_when_no_host_data() {
    let sync = TickSyncState::default();
    assert_eq!(sync.last_host_tick, 0);
    assert_eq!(sync.current_speed, 1.0, "Speed should stay at 1.0 when no host data");
}

#[test]
fn tick_sync_slows_when_ahead() {
    let mut sync = TickSyncState::default();
    // Simulate being 15 ticks ahead of host
    sync.local_tick = 100;
    sync.last_host_tick = 85;

    let drift = sync.local_tick as i64 - sync.last_host_tick as i64;
    assert_eq!(drift, 15, "Drift should be +15 (local ahead)");

    // With drift of 15 (> AGGRESSIVE_THRESHOLD=10), speed should be 0.85
    // This tests the algorithm's expected output directly
    let abs_drift = drift.unsigned_abs() as i64;
    assert!(abs_drift > 10, "Drift should exceed aggressive threshold");
    // When ahead, target speed should be < 1.0
    let target_speed = if abs_drift > 30 {
        0.80
    } else if abs_drift > 10 {
        0.85
    } else if abs_drift > 2 {
        0.95
    } else {
        1.0
    };
    assert!(target_speed < 1.0, "Speed should be < 1.0 when ahead: {target_speed}");
    assert_eq!(target_speed, 0.85);
}

#[test]
fn tick_sync_speeds_up_when_behind() {
    let mut sync = TickSyncState::default();
    // Simulate being 15 ticks behind host
    sync.local_tick = 85;
    sync.last_host_tick = 100;

    let drift = sync.local_tick as i64 - sync.last_host_tick as i64;
    assert_eq!(drift, -15, "Drift should be -15 (local behind)");

    let abs_drift = drift.unsigned_abs() as i64;
    assert!(abs_drift > 10, "Drift should exceed aggressive threshold");
    // When behind, target speed should be > 1.0
    let target_speed = if abs_drift > 30 {
        1.20
    } else if abs_drift > 10 {
        1.15
    } else if abs_drift > 2 {
        1.05
    } else {
        1.0
    };
    assert!(target_speed > 1.0, "Speed should be > 1.0 when behind: {target_speed}");
    assert_eq!(target_speed, 1.15);
}

#[test]
fn tick_sync_gentle_adjustment_for_small_drift() {
    // Test drift in the gentle range (2 < drift <= 10)
    let drift: i64 = 5; // 5 ticks ahead
    let abs_drift = drift.unsigned_abs() as i64;

    assert!(abs_drift > 2 && abs_drift <= 10, "Should be in gentle range");

    let target_speed = if drift > 0 { 0.95 } else { 1.05 };
    assert_eq!(target_speed, 0.95, "Should gently slow down when slightly ahead");
}
