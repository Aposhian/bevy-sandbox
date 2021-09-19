
use bevy::{
    input::{keyboard::KeyCode, Input},
    prelude::*,
};
use bevy_rapier2d::prelude::*;

pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(keyboard.system())
            .add_system(movement.system());
    }
}

/// Generic move action for all movable things
#[derive(Default)]
pub struct MoveAction {
    pub desired_velocity: Vec2
}

/// Tag that marks entity as playable
pub struct PlayerTag;

fn keyboard(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&PlayerTag, &mut MoveAction)>
    ) {
    for (_tag, mut move_action) in query.iter_mut() {
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

fn movement(mut query: Query<(&MoveAction, &mut RigidBodyVelocity)>) {
    for (move_action, mut velocity) in query.iter_mut() {
        // TODO: use forces or impulses rather than setting velocity
        velocity.linvel = (move_action.desired_velocity * 10.0).into();
    }
}
