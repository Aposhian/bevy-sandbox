use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::pathfinding::Path;

pub struct PathfollowingPlugin;

impl Plugin for PathfollowingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system(create_carrot.system())
            .add_system(go_to_carrot.system())
            .add_system(goal_checker.system());
    }
}

pub struct Carrot {
    index: usize
}

impl Default for Carrot {
    fn default() -> Self {
        Carrot {
            index: 0
        }
    }
}

fn create_carrot(
    mut commands: Commands,
    q: Query<Entity, Added<Path>>
) {
    for entity in q.iter() {
        info!("Inserting carrot");
        commands.entity(entity).insert(Carrot::default());
    }
}

fn go_to_carrot(
    mut q: Query<(&mut RigidBodyVelocity, &RigidBodyPosition, &Carrot, &Path), Or<(Added<Carrot>, Changed<Carrot>)>>
) {
    for (mut vel, pos, carrot, path) in q.iter_mut() {
        let carrot_position = path.points[carrot.index];
        let current_position: Vec2 = pos.position.translation.into();

        let delta = carrot_position - current_position;

        // info!("Applying force");
        // forces.apply_force_at_point(&RigidBodyMassProps::default(), delta.into(), current_position.into());
        info!("Setting velocity");
        vel.linvel = delta.into();
    }
}

const GOAL_TOLERANCE: f32 = 0.1;

fn goal_checker(
    mut commands: Commands,
    mut q: Query<(Entity, &mut Carrot, &RigidBodyPosition, &Path)>
) {
    for (entity , mut carrot, pos, path) in q.iter_mut() {
        let carrot_position = path.points[carrot.index];
        let current_position: Vec2 = pos.position.translation.into();

        if carrot_position.distance_squared(current_position) < GOAL_TOLERANCE {
            info!("Reached carrot!");
            carrot.index += 1;
            if carrot.index > path.points.len() {
                commands.entity(entity)
                    .remove::<Path>()
                    .remove::<Carrot>();
            }
        }
    }
}
