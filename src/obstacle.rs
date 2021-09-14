use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use nalgebra::Isometry2;

pub struct ObstaclePlugin;

impl Plugin for ObstaclePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_plugin(RapierRenderPlugin);
    }
}

pub fn spawn(mut commands: Commands) {
    let collider = ColliderBundle {
        shape: ColliderShape::cuboid(1.0, 1.0),
        position: Isometry2::new(
            [3.0, 3.0].into(),
            0.0,
        )
        .into(),
        ..Default::default()
    };
    commands
        .spawn_bundle(collider)
        .insert(ColliderDebugRender::with_id(2))
        .insert(ColliderPositionSync::Discrete);
}