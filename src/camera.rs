use bevy::prelude::*;
use bevy::render::camera::Camera;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup.system())
            .add_system(camera_follow.system());
    }
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
}

pub struct CameraTarget;

const X_DEAD_ZONE: f32 = 32.0;
const Y_DEAD_ZONE: f32 = 32.0;

fn camera_follow(
    mut q: QuerySet<(
        Query<(&CameraTarget, &Transform)>,
        Query<(&Camera, &mut Transform)>
    )>
) {
    let translation = if let Some((_tag, target_transform)) = q.q0().iter().next() {
        Some(target_transform.translation)
    } else {
        None
    };

    if let Some((_current_camera, mut camera_transform)) = q.q1_mut().iter_mut().next() {
        if let Some(translation) = translation {
            let x_diff = translation.x - camera_transform.translation.x;
            let y_diff = translation.y -  camera_transform.translation.y;
            if x_diff.abs() > X_DEAD_ZONE {
                camera_transform.translation.x = translation.x - x_diff.signum() * X_DEAD_ZONE;
            }
            if y_diff.abs() > Y_DEAD_ZONE {
                camera_transform.translation.y = translation.y - y_diff.signum() * X_DEAD_ZONE;
            }
        }
    }
}
