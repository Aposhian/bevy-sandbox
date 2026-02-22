use crate::ball::BallSpawnEvent;
use crate::game_state::GameState;
use crate::net::NetworkRole;
use crate::PIXELS_PER_METER;
use avian2d::prelude::*;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct InputPlugin;

/// Seconds between shots while the mouse button is held.
const SHOOT_RATE: f32 = 0.15;

#[derive(Resource)]
struct ShootTimer(Timer);

impl Default for ShootTimer {
    fn default() -> Self {
        ShootTimer(Timer::from_seconds(SHOOT_RATE, TimerMode::Repeating))
    }
}

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShootTimer>()
            .add_systems(
                Update,
                (keyboard, mouse_aim)
                    .run_if(in_state(GameState::Playing))
                    .run_if(not_guest),
            )
            .add_systems(
                Update,
                movement.run_if(in_state(GameState::Playing)),
            );
    }
}

/// Generic move action for all movable things
#[derive(Default, Component)]
pub struct MoveAction {
    pub desired_velocity: Vec2,
}

/// Tag that marks entity as playable
#[derive(Component)]
pub struct PlayerTag;

fn not_guest(role: Res<NetworkRole>) -> bool {
    !matches!(*role, NetworkRole::Guest { .. })
}

fn keyboard(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut MoveAction, With<PlayerTag>>,
) {
    for mut move_action in query.iter_mut() {
        let mut desired_velocity = Vec2::splat(0.0);

        if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
            desired_velocity.y += 1.0;
        }

        if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
            desired_velocity.y -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
            desired_velocity.x -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
            desired_velocity.x += 1.0;
        }

        move_action.desired_velocity = if desired_velocity.length_squared() != 0.0 {
            desired_velocity.normalize()
        } else {
            desired_velocity
        };
    }
}

fn mouse_aim(
    buttons: Res<ButtonInput<MouseButton>>,
    time: Res<Time>,
    mut shoot_timer: ResMut<ShootTimer>,
    window_query: Query<&Window, With<PrimaryWindow>>,
    player_query: Query<&GlobalTransform, With<PlayerTag>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut ball_spawn_event: MessageWriter<BallSpawnEvent>,
    ui_interaction: Query<&Interaction, With<Button>>,
) {
    // Don't fire when clicking on UI buttons
    let clicking_ui = ui_interaction
        .iter()
        .any(|i| *i != Interaction::None);
    if clicking_ui {
        return;
    }

    let just_pressed = buttons.just_pressed(MouseButton::Left);
    let held = buttons.pressed(MouseButton::Left);

    shoot_timer.0.tick(time.delta());

    // On initial press, fire immediately and reset the timer so the first
    // auto-fire interval starts from this click rather than whenever the
    // timer last happened to finish.
    if just_pressed {
        shoot_timer.0.reset();
    }

    let should_fire = just_pressed || (held && shoot_timer.0.just_finished());
    if !should_fire {
        return;
    }

    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    for player_tf in player_query.iter() {
        let Some(cursor_pos) = window.cursor_position() else {
            continue;
        };
        let Ok(cursor_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos)
        else {
            continue;
        };

        let player_pos = player_tf.translation().xy();
        let direction = (cursor_world_pos - player_pos).normalize_or_zero();

        ball_spawn_event.write(BallSpawnEvent {
            position: player_pos + direction * PIXELS_PER_METER,
            velocity: direction * 10.0 * PIXELS_PER_METER,
        });
    }
}

fn movement(mut query: Query<(&MoveAction, &mut LinearVelocity)>) {
    for (move_action, mut velocity) in query.iter_mut() {
        velocity.0 = move_action.desired_velocity * 5.0 * PIXELS_PER_METER;
    }
}
