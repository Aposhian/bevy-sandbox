use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub fn spawn(mut commands: Commands) {
    commands
        .spawn_bundle(TransformBundle::from(Transform::from_xyz(
            100.0, 100.0, 0.0,
        )))
        .insert(Collider::cuboid(100.0, 100.0));
}
