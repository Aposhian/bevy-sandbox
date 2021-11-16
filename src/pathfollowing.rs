use bevy::prelude::*;
use bevy_rapier2d::{na::Isometry2, prelude::*};

use crate::pathfinding::Path;

pub struct PathfollowingPlugin;

impl Plugin for PathfollowingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system(follow_path.system());
    }
}

fn find_closest_point<'a>(path: &'a Path, rb_position: &RigidBodyPosition) -> Option<(usize, &'a Vec2)> {
    let position = rb_position.position.translation.into();
    path.points.iter().enumerate().min_by_key(|(i, &point)| point.distance_squared(position) as i32)
}

fn follow_path(
    mut q: Query<(&mut RigidBodyForces, &RigidBodyPosition, &Path), Or<(Added<Path>, Changed<Path>)>>
) {
    for (forces, rb_position, path) in q.iter_mut() {
        if let Some((index, point)) = find_closest_point(path, rb_position) {

        }
    }
}
