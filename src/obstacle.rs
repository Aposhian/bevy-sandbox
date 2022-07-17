use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

pub struct ObstaclePlugin;

impl Plugin for ObstaclePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(RapierDebugRenderPlugin::default());
    }
}

pub fn spawn(mut commands: Commands) {
    commands
        .spawn()
        .insert(Collider::cuboid(1.0, 1.0))
        .insert(Transform::from_translation(Vec3::new(1000.0, 1000.0, 0.0)));
}
