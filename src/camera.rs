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
            camera_transform.translation.x = translation.x;
            camera_transform.translation.y = translation.y;
        }
    }
}
