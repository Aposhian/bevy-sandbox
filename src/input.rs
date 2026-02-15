use crate::ball::BallSpawnEvent;
use crate::PIXELS_PER_METER;
use avian2d::prelude::*;
use bevy::math::Vec3Swizzles;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (keyboard, mouse_aim, movement));
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
    window_query: Query<&Window, With<PrimaryWindow>>,
    player_query: Query<&GlobalTransform, With<PlayerTag>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    mut ball_spawn_event: MessageWriter<BallSpawnEvent>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };

    for player_tf in player_query.iter() {
        if let Some(cursor_pos) = window.cursor_position() {
            if buttons.just_pressed(MouseButton::Left) {
                // Convert cursor screen position to world position
                let Ok(cursor_world_pos) =
                    camera.viewport_to_world_2d(camera_transform, cursor_pos)
                else {
                    continue;
                };

                let player_pos = player_tf.translation().xy();
                let direction = (cursor_world_pos - player_pos).normalize_or_zero();

                info!("goal_position: {:?}", cursor_world_pos);

                ball_spawn_event.write(BallSpawnEvent {
                    position: player_pos + direction * PIXELS_PER_METER,
                    velocity: direction * 10.0 * PIXELS_PER_METER,
                });
            }
        }
    }
}

fn movement(mut query: Query<(&MoveAction, &mut LinearVelocity)>) {
    for (move_action, mut velocity) in query.iter_mut() {
        velocity.0 = move_action.desired_velocity * 5.0 * PIXELS_PER_METER;
    }
}
