use bevy::prelude::*;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, camera_follow);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
}

#[derive(Component)]
pub struct CameraTarget;

const X_DEAD_ZONE: f32 = 32.0;
const Y_DEAD_ZONE: f32 = 32.0;

fn camera_follow(
    target_query: Query<&Transform, (With<CameraTarget>, Without<Camera2d>)>,
    mut camera_query: Query<&mut Transform, With<Camera2d>>,
) {
    let Some(target_transform) = target_query.iter().next() else {
        return;
    };
    let Ok(mut camera_transform) = camera_query.single_mut() else {
        return;
    };

    let translation = target_transform.translation;
    let x_diff = translation.x - camera_transform.translation.x;
    let y_diff = translation.y - camera_transform.translation.y;
    if x_diff.abs() > X_DEAD_ZONE {
        camera_transform.translation.x = translation.x - x_diff.signum() * X_DEAD_ZONE;
    }
    if y_diff.abs() > Y_DEAD_ZONE {
        camera_transform.translation.y = translation.y - y_diff.signum() * Y_DEAD_ZONE;
    }
}
