use crate::ball::BallSpawnEvent;
use bevy::math::Vec3Swizzles;
use bevy::math::Vec4Swizzles;
use bevy::render::camera::Camera;
use bevy::{
    input::{keyboard::KeyCode, Input},
    prelude::*,
};
use bevy_rapier2d::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(keyboard)
            .add_system(mouse_aim)
            .add_system(movement);
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
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut MoveAction, With<PlayerTag>>,
) {
    for mut move_action in query.iter_mut() {
        let mut desired_velocity = Vec2::splat(0.0);

        if keyboard_input.pressed(KeyCode::W) || keyboard_input.pressed(KeyCode::Up) {
            desired_velocity.y += 1.0;
        }

        if keyboard_input.pressed(KeyCode::S) || keyboard_input.pressed(KeyCode::Down) {
            desired_velocity.y -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::A) || keyboard_input.pressed(KeyCode::Left) {
            desired_velocity.x -= 1.0;
        }

        if keyboard_input.pressed(KeyCode::D) || keyboard_input.pressed(KeyCode::Right) {
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
    buttons: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    player_query: Query<&GlobalTransform, With<PlayerTag>>,
    camera_query: Query<&Transform, With<Camera>>,
    mut ball_spawn_event: EventWriter<BallSpawnEvent>,
) {
    for player_tf in player_query.iter() {
        if let Some(window) = windows.get_primary() {
            if let Some(cursor_pos) = window.cursor_position() {
                if buttons.just_pressed(MouseButton::Left) {
                    let size = Vec2::new(window.width() as f32, window.height() as f32);

                    // https://bevy-cheatbook.github.io/cookbook/cursor2world.html
                    // the default orthographic projection is in pixels from the center;
                    // just undo the translation
                    let p = cursor_pos - size / 2.0;

                    // assuming there is exactly one main camera entity, so this is OK
                    let camera_transform = camera_query.single();

                    // apply the camera transform
                    let cursor_world_pos =
                        camera_transform.compute_matrix() * p.extend(0.0).extend(1.0);

                    let player_pos = (player_tf.translation).xy();
                    let cursor_real_pos = (cursor_world_pos).xy();
                    let direction = (cursor_real_pos - player_pos).normalize_or_zero();

                    info!("goal_position: {:?}", cursor_real_pos);

                    ball_spawn_event.send(BallSpawnEvent {
                        transform: Transform::from_translation(
                            (player_pos + direction).extend(2.0),
                        ),
                        velocity: direction * 10.0,
                        ..Default::default()
                    });
                }
            }
        }
    }
}

fn movement(mut query: Query<(&MoveAction, &mut Velocity)>) {
    for (move_action, mut velocity) in query.iter_mut() {
        // TODO: use forces or impulses rather than setting velocity
        velocity.linvel = (move_action.desired_velocity * 5.0).into();
    }
}
