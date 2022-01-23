use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::input::MoveAction;
use crate::pathfinding::Path;

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

fn reset_carrot(mut commands: Commands, q: Query<Entity, Or<(Added<Path>, Changed<Path>)>>) {
    for entity in q.iter() {
        info!("Inserting or resetting carrot");
        commands.entity(entity).insert(Carrot::default());
    }
}

const VELOCITY_SCALE: f32 = 0.5;

fn go_to_carrot(
    mut q: Query<
        (&mut MoveAction, &RigidBodyPositionComponent, &Carrot, &Path),
        Or<(Added<Carrot>, Changed<Carrot>)>,
    >,
) {
    for (mut move_action, pos, carrot, path) in q.iter_mut() {
        if let Some(&carrot_position) = path.points.get(carrot.index) {
            let current_position: Vec2 = pos.position.translation.into();

            let delta = (carrot_position - current_position).normalize_or_zero();
            move_action.desired_velocity = (VELOCITY_SCALE * delta).into();
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
        &Path,
    )>,
) {
    for (entity, mut carrot, mut vel, pos, path) in q.iter_mut() {
        if let Some(carrot_position) = path.points.get(carrot.index) {
            let current_position: Vec2 = pos.position.translation.into();
            if carrot_position.distance_squared(current_position) < GOAL_TOLERANCE {
                carrot.index += 1;
                info!("Reached carrot {}", carrot.index);
                if carrot.index >= path.points.len() {
                    info!("Removing Carrot and Path");
                    vel.linvel = Vec2::ZERO.into();
                    commands.entity(entity).remove::<Path>().remove::<Carrot>();
                }
            }
        }
    }
}
