use benimator::AnimationPlugin;
use bevy_rapier2d::prelude::*;
use bevy::render::pass::ClearColor;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod simple_figure;
mod input;
pub mod obstacle;
mod camera;
mod ball;

use simple_figure::SimpleFigurePlugin;
use input::InputPlugin;
use obstacle::ObstaclePlugin;
use camera::CameraPlugin;
use ball::BallPlugin;
pub struct SandboxPlugins;

impl PluginGroup for SandboxPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(AnimationPlugin);
        group.add(RapierPhysicsPlugin::<NoUserData>::default());
        group.add(DefaultResources);
        group.add(InputPlugin);
        group.add(SimpleFigurePlugin);
        group.add(ObstaclePlugin);
        group.add(CameraPlugin);
        group.add(BallPlugin);
    }
}

pub struct DefaultResources;

impl Plugin for DefaultResources {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(ClearColor(Color::rgb(
                0xF9 as f32 / 255.0,
                0xF9 as f32 / 255.0,
                0xFF as f32 / 255.0,
            )))
            .insert_resource(Msaa::default());
    }
}
