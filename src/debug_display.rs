use std::collections::VecDeque;

use bevy::prelude::*;

use crate::net::sync::TickSyncState;
use crate::net::{HostTick, NetworkRole};

pub struct DebugDisplayPlugin;

/// Length of the rolling window used for avg/min/max stats.
const WINDOW_SECS: f32 = 10.0;

impl Plugin for DebugDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FpsBuffer>()
            .init_resource::<DebugDisplayVisible>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (update_buffer, update_text, toggle_visibility).chain(),
            );
    }
}

// ---------------------------------------------------------------------------
// Visibility toggle
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct DebugDisplayVisible(bool);

impl Default for DebugDisplayVisible {
    fn default() -> Self {
        DebugDisplayVisible(true)
    }
}

// ---------------------------------------------------------------------------
// Rolling frame-time buffer
// ---------------------------------------------------------------------------

#[derive(Resource)]
struct FpsBuffer {
    /// Individual frame delta-times in seconds, oldest first.
    frames: VecDeque<f32>,
    /// Running sum of all deltas currently in the buffer.
    total_secs: f32,
}

impl Default for FpsBuffer {
    fn default() -> Self {
        FpsBuffer {
            frames: VecDeque::new(),
            total_secs: 0.0,
        }
    }
}

impl FpsBuffer {
    fn push(&mut self, delta: f32) {
        self.frames.push_back(delta);
        self.total_secs += delta;
        // Evict oldest frames until the window fits within WINDOW_SECS.
        // Keep at least one frame so current_fps() always has data.
        while self.total_secs > WINDOW_SECS && self.frames.len() > 1 {
            if let Some(old) = self.frames.pop_front() {
                self.total_secs -= old;
            }
        }
    }

    /// FPS of the most recent frame.
    fn current(&self) -> f32 {
        self.frames.back().map(|&d| 1.0 / d).unwrap_or(0.0)
    }

    /// Mean FPS over the whole window.
    fn avg(&self) -> f32 {
        if self.total_secs > 0.0 {
            self.frames.len() as f32 / self.total_secs
        } else {
            0.0
        }
    }

    /// Lowest FPS in the window (corresponds to the longest frame).
    fn min(&self) -> f32 {
        let max_delta = self.frames.iter().copied().fold(0.0_f32, f32::max);
        if max_delta > 0.0 {
            1.0 / max_delta
        } else {
            0.0
        }
    }

    /// Highest FPS in the window (corresponds to the shortest frame).
    fn max(&self) -> f32 {
        let min_delta = self
            .frames
            .iter()
            .copied()
            .fold(f32::INFINITY, f32::min);
        if min_delta.is_finite() && min_delta > 0.0 {
            1.0 / min_delta
        } else {
            0.0
        }
    }
}

// ---------------------------------------------------------------------------
// UI
// ---------------------------------------------------------------------------

#[derive(Component)]
struct DebugDisplayText;

fn setup(mut commands: Commands) {
    commands.spawn((
        DebugDisplayText,
        Text::new(""),
        TextFont {
            font_size: 13.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(8.0),
            left: Val::Px(8.0),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
    ));
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

fn toggle_visibility(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut visible: ResMut<DebugDisplayVisible>,
    mut query: Query<&mut Node, With<DebugDisplayText>>,
) {
    if keyboard.just_pressed(KeyCode::F2) {
        visible.0 = !visible.0;
    }
    for mut node in query.iter_mut() {
        node.display = if visible.0 {
            Display::DEFAULT
        } else {
            Display::None
        };
    }
}

fn update_buffer(time: Res<Time>, mut buffer: ResMut<FpsBuffer>) {
    let delta = time.delta_secs();
    if delta > 0.0 {
        buffer.push(delta);
    }
}

fn update_text(
    buffer: Res<FpsBuffer>,
    role: Res<NetworkRole>,
    host_tick: Res<HostTick>,
    sync_state: Res<TickSyncState>,
    mut query: Query<&mut Text, With<DebugDisplayText>>,
) {
    let Some(mut text) = query.iter_mut().next() else {
        return;
    };

    let elapsed = buffer.total_secs.min(WINDOW_SECS);
    let mut lines = format!(
        "cur {:.0}  avg {:.0}  min {:.0}  max {:.0} fps  ({:.0}s)",
        buffer.current(),
        buffer.avg(),
        buffer.min(),
        buffer.max(),
        elapsed,
    );

    // Network info
    match &*role {
        NetworkRole::Offline => {
            lines.push_str("\noffline");
        }
        NetworkRole::Host { port } => {
            lines.push_str(&format!("\nhost :{port}  tick {}", host_tick.0));
        }
        NetworkRole::Guest { addr } => {
            lines.push_str(&format!("\nguest -> {addr}"));
            lines.push_str(&format!("\nhost tick {}", sync_state.last_host_tick));
            lines.push_str(&format!("  local tick {}", sync_state.local_tick));

            if !sync_state.drift_samples.is_empty() {
                let samples = &sync_state.drift_samples;
                let n = samples.len() as f64;
                let avg = samples.iter().sum::<i64>() as f64 / n;
                let min = samples.iter().copied().min().unwrap_or(0);
                let max = samples.iter().copied().max().unwrap_or(0);
                lines.push_str(&format!(
                    "\nslew {:.2}x  drift avg {avg:.1} min {min} max {max}",
                    sync_state.current_speed,
                ));
            }
        }
    }

    text.0 = lines;
}
