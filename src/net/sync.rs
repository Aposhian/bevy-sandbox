use std::collections::VecDeque;

use bevy::prelude::*;

use crate::game_state::GameState;

use super::NetworkRole;

pub struct SyncPlugin;

impl Plugin for SyncPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TickSyncState>().add_systems(
            FixedUpdate,
            tick_sync
                .run_if(is_guest)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn is_guest(role: Res<NetworkRole>) -> bool {
    matches!(*role, NetworkRole::Guest { .. })
}

const DRIFT_WINDOW: usize = 30;
const GENTLE_THRESHOLD: i64 = 2;
const AGGRESSIVE_THRESHOLD: i64 = 10;
const RESYNC_THRESHOLD: i64 = 30;

#[derive(Resource)]
pub struct TickSyncState {
    pub last_host_tick: u64,
    pub local_tick: u64,
    pub drift_samples: VecDeque<i64>,
    pub current_speed: f64,
}

impl Default for TickSyncState {
    fn default() -> Self {
        TickSyncState {
            last_host_tick: 0,
            local_tick: 0,
            drift_samples: VecDeque::with_capacity(DRIFT_WINDOW),
            current_speed: 1.0,
        }
    }
}

fn tick_sync(mut sync: ResMut<TickSyncState>, mut virtual_time: ResMut<Time<Virtual>>) {
    sync.local_tick += 1;

    if sync.last_host_tick == 0 {
        return; // No data from host yet
    }

    let drift = sync.local_tick as i64 - sync.last_host_tick as i64;

    // Add to rolling window
    if sync.drift_samples.len() >= DRIFT_WINDOW {
        sync.drift_samples.pop_front();
    }
    sync.drift_samples.push_back(drift);

    // Compute rolling average
    let avg_drift: f64 =
        sync.drift_samples.iter().sum::<i64>() as f64 / sync.drift_samples.len() as f64;

    let abs_drift = avg_drift.abs() as i64;

    let target_speed = if abs_drift > RESYNC_THRESHOLD {
        // Extreme drift — should trigger full resync
        // For now, just aggressively slew
        if avg_drift > 0.0 {
            0.80
        } else {
            1.20
        }
    } else if abs_drift > AGGRESSIVE_THRESHOLD {
        if avg_drift > 0.0 {
            0.85 // We're ahead, slow down
        } else {
            1.15 // We're behind, speed up
        }
    } else if abs_drift > GENTLE_THRESHOLD {
        if avg_drift > 0.0 {
            0.95
        } else {
            1.05
        }
    } else {
        // Within tolerance, lerp back toward 1.0
        sync.current_speed + (1.0 - sync.current_speed) * 0.1
    };

    sync.current_speed = target_speed;
    virtual_time.set_relative_speed(target_speed as f32);
}
