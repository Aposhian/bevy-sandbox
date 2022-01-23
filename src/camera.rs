use bevy::prelude::*;
use bevy::render::camera::Camera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(setup).add_system(camera_follow);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

#[derive(Component)]
pub struct CameraTarget;

const X_DEAD_ZONE: f32 = 32.0;
const Y_DEAD_ZONE: f32 = 32.0;

fn camera_follow(
    mut q: QuerySet<(
        QueryState<(&CameraTarget, &Transform)>,
        QueryState<(&Camera, &mut Transform)>,
    )>,
) {
    let translation = if let Some((_tag, target_transform)) = q.q0().iter().next() {
        Some(target_transform.translation)
    } else {
        None
    };

    if let Some((_current_camera, mut camera_transform)) = q.q1().iter_mut().next() {
        if let Some(translation) = translation {
            let x_diff = translation.x - camera_transform.translation.x;
            let y_diff = translation.y - camera_transform.translation.y;
            if x_diff.abs() > X_DEAD_ZONE {
                camera_transform.translation.x = translation.x - x_diff.signum() * X_DEAD_ZONE;
            }
            if y_diff.abs() > Y_DEAD_ZONE {
                camera_transform.translation.y = translation.y - y_diff.signum() * X_DEAD_ZONE;
            }
        }
    }
}
