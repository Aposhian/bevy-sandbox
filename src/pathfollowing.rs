use avian2d::prelude::*;
use bevy::prelude::*;

use crate::input::MoveAction;
use crate::pathfinding::Path;
use crate::PIXELS_PER_METER;

pub struct PathfollowingPlugin;

impl Plugin for PathfollowingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (reset_carrot, go_to_carrot, goal_checker));
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
        (&mut MoveAction, &Transform, &Carrot, &Path),
        Or<(Added<Carrot>, Changed<Carrot>)>,
    >,
) {
    for (mut move_action, transform, carrot, path) in q.iter_mut() {
        if let Some(&carrot_position) = path.points.get(carrot.index) {
            // Path points are in physics meters; convert transform position to meters
            let current_position = transform.translation.truncate() / PIXELS_PER_METER;

            let delta = (carrot_position - current_position).normalize_or_zero();
            move_action.desired_velocity = VELOCITY_SCALE * delta;
        }
    }
}

const GOAL_TOLERANCE: f32 = 0.1;

fn goal_checker(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Carrot, &mut LinearVelocity, &Transform, &Path)>,
) {
    for (entity, mut carrot, mut vel, transform, path) in q.iter_mut() {
        if let Some(carrot_position) = path.points.get(carrot.index) {
            let current_position = transform.translation.truncate() / PIXELS_PER_METER;
            if carrot_position.distance_squared(current_position) < GOAL_TOLERANCE {
                carrot.index += 1;
                info!("Reached carrot {}", carrot.index);
                if carrot.index >= path.points.len() {
                    info!("Removing Carrot and Path");
                    vel.0 = Vec2::ZERO;
                    commands.entity(entity).remove::<Path>().remove::<Carrot>();
                }
            }
        }
    }
}
