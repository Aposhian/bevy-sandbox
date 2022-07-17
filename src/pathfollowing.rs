use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use futures_lite::future;

use crate::input::MoveAction;
use crate::pathfinding::ComputePath;

pub struct PathfollowingPlugin;

impl Plugin for PathfollowingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(reset_carrot)
            .add_system(go_to_carrot)
            .add_system(goal_checker);
    }
}

#[derive(Component)]
pub struct Carrot {
    index: usize,
}

impl Default for Carrot {
    fn default() -> Self {
        Carrot { index: 0 }
    }
}

fn reset_carrot(
    mut commands: Commands,
    q: Query<Entity, Or<(Added<ComputePath>, Changed<ComputePath>)>>,
) {
    for entity in q.iter() {
        commands.entity(entity).insert(Carrot::default());
    }
}

const VELOCITY_SCALE: f32 = 0.5;

fn go_to_carrot(
    mut q: Query<
        (
            &mut MoveAction,
            &RigidBodyPositionComponent,
            &Carrot,
            &mut ComputePath,
        ),
        Or<(Added<Carrot>, Changed<Carrot>)>,
    >,
) {
    for (mut move_action, pos, carrot, mut path) in q.iter_mut() {
        if let Some(Some(path)) = future::block_on(future::poll_once(&mut path.task)) {
            if let Some(&carrot_position) = path.points.get(carrot.index) {
                let current_position: Vec2 = pos.position.translation.into();

                let delta = (carrot_position - current_position).normalize_or_zero();
                move_action.desired_velocity = (VELOCITY_SCALE * delta).into();
            }
        }
    }
}

const GOAL_TOLERANCE: f32 = 0.1;

fn goal_checker(
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &mut Carrot,
        &mut RigidBodyVelocityComponent,
        &RigidBodyPositionComponent,
        &mut ComputePath,
    )>,
) {
    for (entity, mut carrot, mut vel, pos, mut path) in q.iter_mut() {
        if let Some(Some(path)) = future::block_on(future::poll_once(&mut path.task)) {
            if let Some(carrot_position) = path.points.get(carrot.index) {
                let current_position: Vec2 = pos.position.translation.into();
                if carrot_position.distance_squared(current_position) < GOAL_TOLERANCE {
                    carrot.index += 1;
                    if carrot.index >= path.points.len() {
                        vel.linvel = Vec2::ZERO.into();
                        commands
                            .entity(entity)
                            .remove::<ComputePath>()
                            .remove::<Carrot>();
                    }
                }
            }
        }
    }
}
