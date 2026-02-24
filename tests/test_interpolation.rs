//! Tests for guest-side entity interpolation smoothness.
//!
//! These tests simulate the pattern of server updates arriving at various
//! cadences and verify that the rendered positions are smooth (no large
//! frame-to-frame jumps).

use bevy::prelude::*;

/// Mirror of the guest-side interpolation logic so we can test it in isolation.
/// This must stay in sync with `NetInterpolation` in `src/net/guest.rs`.
mod interp {
    use bevy::prelude::*;
    use std::collections::VecDeque;

    pub const SERVER_TICK_DURATION: f32 = 1.0 / 64.0;
    /// Maximum number of buffered snapshots before we skip ahead.
    const MAX_BUFFER: usize = 4;

    /// Buffered interpolation using a position timeline.
    ///
    /// Server positions are placed on a timeline spaced by SERVER_TICK_DURATION.
    /// A playback cursor advances with real time, always staying within the
    /// buffered range. The rendered position is linearly interpolated between
    /// the two surrounding timeline entries.
    ///
    /// If the buffer grows too large (client falling behind), entries are
    /// discarded from the front to catch up.
    #[derive(Clone, Debug)]
    pub struct NetInterpolation {
        /// Timeline of positions. Entry 0 is at time `base_time`.
        /// Each subsequent entry is SERVER_TICK_DURATION later.
        pub timeline: VecDeque<Vec3>,
        /// The time of timeline[0].
        pub base_time: f32,
        /// Current playback cursor (absolute time).
        pub cursor: f32,
    }

    impl NetInterpolation {
        pub fn new(pos: Vec3) -> Self {
            Self {
                timeline: VecDeque::from([pos]),
                // base_time and cursor start at 0. The cursor will naturally
                // trail the newest data by one tick once the buffer fills.
                base_time: 0.0,
                cursor: 0.0,
            }
        }

        /// Enqueue new server positions. Each is one SERVER_TICK_DURATION
        /// after the last entry on the timeline.
        pub fn push_updates(&mut self, updates: &[Vec3]) {
            let was_starved = self.timeline.len() < 2;
            for &pos in updates {
                self.timeline.push_back(pos);
            }
            if was_starved && self.timeline.len() >= 2 {
                self.cursor = self.base_time;
            }
        }

        pub fn step(&mut self, dt: f32) -> Vec3 {
            self.cursor += dt.min(SERVER_TICK_DURATION);
            let pos = self.current_pos();

            while self.timeline.len() > 2
                && self.cursor >= self.base_time + SERVER_TICK_DURATION
            {
                self.timeline.pop_front();
                self.base_time += SERVER_TICK_DURATION;
            }

            pos
        }

        pub fn current_pos(&self) -> Vec3 {
            if self.timeline.len() < 2 {
                return *self.timeline.back().unwrap_or(&Vec3::ZERO);
            }

            let end_time = self.base_time
                + (self.timeline.len() - 1) as f32 * SERVER_TICK_DURATION;
            let clamped = self.cursor.clamp(self.base_time, end_time);

            let local = clamped - self.base_time;
            let seg = (local / SERVER_TICK_DURATION) as usize;
            let seg = seg.min(self.timeline.len() - 2);
            let t = (local - seg as f32 * SERVER_TICK_DURATION) / SERVER_TICK_DURATION;

            self.timeline[seg].lerp(self.timeline[seg + 1], t)
        }
    }
}

use interp::*;

/// Helper: compute the maximum frame-to-frame position delta across a
/// sequence of rendered positions.
fn max_frame_delta(positions: &[Vec3]) -> f32 {
    positions
        .windows(2)
        .map(|w| (w[1] - w[0]).length())
        .fold(0.0_f32, f32::max)
}

/// Simulate the full drain-and-apply logic: push all received updates
/// into the interpolation buffer.
fn drain_and_apply(
    interp: &mut NetInterpolation,
    pending: &[Vec3],
) {
    if pending.is_empty() {
        return;
    }
    interp.push_updates(pending);
}

// =============================================================================
// Test: steady 1:1 ratio (one server update per frame, ideal case)
// =============================================================================
#[test]
fn steady_one_update_per_frame() {
    let mut interp = NetInterpolation::new(Vec3::ZERO);
    let speed = 160.0; // pixels per second
    let dt = SERVER_TICK_DURATION; // frame time == tick time

    let mut positions = vec![interp.current_pos()];

    for tick in 1..=60 {
        // Server sends one position per tick
        let server_pos = Vec3::new(speed * tick as f32 * SERVER_TICK_DURATION, 0.0, 0.0);
        drain_and_apply(&mut interp, &[server_pos]);
        let pos = interp.step(dt);
        positions.push(pos);
    }

    let max_delta = max_frame_delta(&positions);
    let expected_per_frame = speed * dt;

    // The maximum frame delta should not exceed 2x the expected per-frame movement
    assert!(
        max_delta <= expected_per_frame * 2.0,
        "steady 1:1 — max frame delta {max_delta:.2} exceeds 2x expected {:.2}",
        expected_per_frame * 2.0
    );

    // Should also not be zero (entity must be moving)
    assert!(
        max_delta > 0.0,
        "entity should be moving"
    );
}

// =============================================================================
// Test: 144fps client, 64Hz server — some frames get 0 updates, some get 1-2
// This is the most common real-world scenario.
// =============================================================================
#[test]
fn high_fps_client_with_server_64hz() {
    let mut interp = NetInterpolation::new(Vec3::ZERO);
    let speed = 160.0;
    let client_dt = 1.0 / 144.0; // ~6.94ms per frame

    let mut positions = vec![interp.current_pos()];
    let mut server_time = 0.0_f32;
    let mut client_time = 0.0_f32;
    // Simulate 2 seconds
    for _frame in 0..288 {
        client_time += client_dt;

        // Collect server updates that have "arrived" by this client frame
        let mut pending = Vec::new();
        while server_time + SERVER_TICK_DURATION <= client_time {
            server_time += SERVER_TICK_DURATION;
            let server_pos = Vec3::new(speed * server_time, 0.0, 0.0);
            pending.push(server_pos);
        }

        drain_and_apply(&mut interp, &pending);
        let pos = interp.step(client_dt);
        positions.push(pos);
    }

    let max_delta = max_frame_delta(&positions);
    let expected_per_frame = speed * client_dt;

    println!("144fps test: max_delta={max_delta:.4}, expected_per_frame={expected_per_frame:.4}");
    println!("  ratio: {:.2}x", max_delta / expected_per_frame);

    // Print first 20 deltas to see the pattern
    println!("  first 20 frame deltas:");
    for (i, w) in positions.windows(2).take(20).enumerate() {
        let delta = (w[1] - w[0]).length();
        println!("    frame {i}: delta={delta:.4}, pos={:.2}", w[1].x);
    }

    // With proper timeline interpolation, every frame delta should be close
    // to the expected per-frame movement. Allow up to 1.5x for rounding.
    assert!(
        max_delta <= expected_per_frame * 1.5,
        "144fps — max frame delta {max_delta:.4} exceeds 1.5x expected {:.4}. \
         This indicates visible jitter.",
        expected_per_frame * 1.5
    );
}

// =============================================================================
// Test: collision scenario — entity oscillates between two positions
// =============================================================================
#[test]
fn collision_oscillation_is_smooth() {
    let mut interp = NetInterpolation::new(Vec3::new(100.0, 0.0, 0.0));
    let dt = SERVER_TICK_DURATION;

    // Simulate: entity tries to move right but wall pushes it back
    // Server alternates between pos 100 and 102 (tiny oscillation)
    let server_positions = [
        Vec3::new(102.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(102.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
        Vec3::new(102.0, 0.0, 0.0),
        Vec3::new(100.0, 0.0, 0.0),
    ];

    let mut positions = vec![interp.current_pos()];

    for server_pos in &server_positions {
        drain_and_apply(&mut interp, &[*server_pos]);
        // Render several sub-frames per tick
        for _ in 0..3 {
            let pos = interp.step(dt / 3.0);
            positions.push(pos);
        }
    }

    let max_delta = max_frame_delta(&positions);

    // The oscillation is 2 pixels. With interpolation, each frame should move
    // at most ~2px (one full oscillation over one tick). Without interpolation
    // it would be 2px instantly.
    assert!(
        max_delta <= 2.5,
        "collision oscillation — max frame delta {max_delta:.2} should be <= 2.5 \
         (smooth interpolation over the 2px oscillation)"
    );
}

// =============================================================================
// Test: frames where no update arrives should NOT freeze then jump
// =============================================================================
#[test]
fn no_update_frames_dont_cause_jumps() {
    let mut interp = NetInterpolation::new(Vec3::ZERO);
    let speed = 160.0;
    let client_dt = 1.0 / 144.0;

    let mut positions = vec![interp.current_pos()];

    // Frame 1: server update arrives
    let pos1 = Vec3::new(speed * SERVER_TICK_DURATION, 0.0, 0.0);
    drain_and_apply(&mut interp, &[pos1]);
    positions.push(interp.step(client_dt));

    // Frames 2-3: no server update (still interpolating toward pos1)
    positions.push(interp.step(client_dt));
    positions.push(interp.step(client_dt));

    // Frame 4: next server update arrives
    let pos2 = Vec3::new(speed * SERVER_TICK_DURATION * 2.0, 0.0, 0.0);
    drain_and_apply(&mut interp, &[pos2]);
    positions.push(interp.step(client_dt));

    // Frame 5: no update
    positions.push(interp.step(client_dt));

    let max_delta = max_frame_delta(&positions);
    let expected_per_frame = speed * client_dt;

    println!("no-update test: positions:");
    for (i, p) in positions.iter().enumerate() {
        println!("  frame {i}: x={:.4}", p.x);
    }
    println!("  max_delta={max_delta:.4}, expected_per_frame={expected_per_frame:.4}");

    // The jump at frame 4 (when update arrives after a gap) is the key thing
    // to measure. It should not be much larger than a normal frame's movement.
    let jump_at_4 = (positions[4] - positions[3]).length();
    println!("  jump at frame 4: {jump_at_4:.4}");

    assert!(
        max_delta <= expected_per_frame * 4.0,
        "no-update frames — max frame delta {max_delta:.4} exceeds 4x expected {:.4}. \
         This means the entity freezes then jumps.",
        expected_per_frame * 4.0
    );
}

// =============================================================================
// Test: multiple updates drained in one frame (2-3 updates batched)
// =============================================================================
#[test]
fn batched_updates_dont_cause_large_jumps() {
    let mut interp = NetInterpolation::new(Vec3::ZERO);
    let speed = 160.0;
    let client_dt = 1.0 / 60.0; // 60fps client, slower than server

    let mut positions = vec![interp.current_pos()];

    // Frame 1: one update
    let pos1 = Vec3::new(speed * SERVER_TICK_DURATION, 0.0, 0.0);
    drain_and_apply(&mut interp, &[pos1]);
    positions.push(interp.step(client_dt));

    // Frame 2: two updates batched (ticks 2 and 3 arrived together)
    let pos2 = Vec3::new(speed * SERVER_TICK_DURATION * 2.0, 0.0, 0.0);
    let pos3 = Vec3::new(speed * SERVER_TICK_DURATION * 3.0, 0.0, 0.0);
    drain_and_apply(&mut interp, &[pos2, pos3]);
    positions.push(interp.step(client_dt));

    // Frame 3: one update
    let pos4 = Vec3::new(speed * SERVER_TICK_DURATION * 4.0, 0.0, 0.0);
    drain_and_apply(&mut interp, &[pos4]);
    positions.push(interp.step(client_dt));

    // Frame 4: no update
    positions.push(interp.step(client_dt));

    let max_delta = max_frame_delta(&positions);
    let expected_per_frame = speed * client_dt;

    // Each frame delta should be in the same ballpark as expected.
    assert!(
        max_delta <= expected_per_frame * 3.0,
        "batched updates — max frame delta {max_delta:.4} exceeds 3x expected {:.4}",
        expected_per_frame * 3.0
    );
}

// =============================================================================
// Test: 60fps client, 64Hz server — client is slower than server tick rate
// Updates accumulate and batch. This tests the common case for lower-end PCs.
// =============================================================================
#[test]
fn low_fps_client_with_server_64hz() {
    let mut interp = NetInterpolation::new(Vec3::ZERO);
    let speed = 160.0;
    let client_dt = 1.0 / 60.0; // ~16.67ms per frame

    let mut positions = vec![interp.current_pos()];
    let mut server_time = 0.0_f32;
    let mut client_time = 0.0_f32;

    // Simulate 2 seconds
    for _frame in 0..120 {
        client_time += client_dt;

        // Collect server updates that have "arrived" by this client frame
        let mut pending = Vec::new();
        while server_time + SERVER_TICK_DURATION <= client_time {
            server_time += SERVER_TICK_DURATION;
            let server_pos = Vec3::new(speed * server_time, 0.0, 0.0);
            pending.push(server_pos);
        }

        drain_and_apply(&mut interp, &pending);
        let pos = interp.step(client_dt);
        positions.push(pos);
    }

    let max_delta = max_frame_delta(&positions);
    let expected_per_frame = speed * client_dt;

    println!("60fps test: max_delta={max_delta:.4}, expected_per_frame={expected_per_frame:.4}");
    println!("  ratio: {:.2}x", max_delta / expected_per_frame);

    // Print first 20 deltas to see the pattern
    // Re-run with tracing around the worst frame
    {
        let mut interp2 = NetInterpolation::new(Vec3::ZERO);
        let mut server_time2 = 0.0_f32;
        let mut client_time2 = 0.0_f32;
        let mut prev_pos = Vec3::ZERO;
        for frame in 0..35 {
            client_time2 += client_dt;
            let mut pending = Vec::new();
            while server_time2 + SERVER_TICK_DURATION <= client_time2 {
                server_time2 += SERVER_TICK_DURATION;
                pending.push(Vec3::new(speed * server_time2, 0.0, 0.0));
            }
            if frame >= 26 && frame <= 32 {
                println!("  frame {frame}: pending={}, timeline_len={}, cursor={:.4}, base={:.4}",
                    pending.len(), interp2.timeline.len(), interp2.cursor, interp2.base_time);
            }
            drain_and_apply(&mut interp2, &pending);
            if frame >= 26 && frame <= 32 {
                println!("    after apply: timeline_len={}, cursor={:.4}, base={:.4}",
                    interp2.timeline.len(), interp2.cursor, interp2.base_time);
            }
            let pos = interp2.step(client_dt);
            if frame >= 26 && frame <= 32 {
                let delta = (pos - prev_pos).length();
                println!("    after step: pos={:.4}, delta={delta:.4}, timeline_len={}, cursor={:.4}, base={:.4}",
                    pos.x, interp2.timeline.len(), interp2.cursor, interp2.base_time);
            }
            prev_pos = pos;
        }
    }

    // Find and print the worst frames
    let mut deltas: Vec<(usize, f32)> = positions
        .windows(2)
        .enumerate()
        .map(|(i, w)| (i, (w[1] - w[0]).length()))
        .collect();
    deltas.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    println!("  worst 5 frame deltas:");
    for (i, delta) in deltas.iter().take(5) {
        println!("    frame {i}: delta={delta:.4}");
    }

    assert!(
        max_delta <= expected_per_frame * 1.5,
        "60fps — max frame delta {max_delta:.4} exceeds 1.5x expected {:.4}. \
         This indicates visible jitter.",
        expected_per_frame * 1.5
    );
}

// =============================================================================
// Test: entity at rest should stay still
// =============================================================================
#[test]
fn stationary_entity_stays_still() {
    let mut interp = NetInterpolation::new(Vec3::new(50.0, 50.0, 0.0));
    let dt = 1.0 / 144.0;

    let mut positions = vec![interp.current_pos()];

    for _ in 0..10 {
        // Server keeps reporting same position
        let same_pos = Vec3::new(50.0, 50.0, 0.0);
        drain_and_apply(&mut interp, &[same_pos]);
        positions.push(interp.step(dt));
    }

    let max_delta = max_frame_delta(&positions);
    assert!(
        max_delta < 0.001,
        "stationary entity should not move, got max delta {max_delta:.6}"
    );
}
