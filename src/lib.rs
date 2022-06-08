use benimator::AnimationPlugin;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy_prototype_lyon::plugin::ShapePlugin;
use bevy_rapier2d::prelude::*;

mod ai;
mod ball;
mod camera;
mod ecs;
mod health;
mod input;
pub mod obstacle;
mod pathfinding;
mod pathfollowing;
mod physics;
pub mod simple_figure;
pub mod tiled;

use crate::pathfinding::PathfindingPlugin;
use ai::AiPlugin;
use ball::BallPlugin;
use camera::CameraPlugin;
use ecs::DespawnPlugin;
use health::HealthPlugin;
use input::InputPlugin;
use pathfollowing::PathfollowingPlugin;
use simple_figure::SimpleFigurePlugin;
pub struct SandboxPlugins;

impl PluginGroup for SandboxPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(AnimationPlugin);
        group.add(RapierPhysicsPlugin::<NoUserData>::default());
        group.add(DefaultResources);
        group.add(InputPlugin);
        group.add(SimpleFigurePlugin);
        group.add(CameraPlugin);
        group.add(BallPlugin);
        group.add(HealthPlugin);
        group.add(PathfindingPlugin);
        group.add(ShapePlugin);
        group.add(PathfollowingPlugin);
        group.add(AiPlugin);
        group.add(DespawnPlugin);
    }
}

pub struct DefaultResources;

impl Plugin for DefaultResources {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::rgb(
            0xF9 as f32 / 255.0,
            0xF9 as f32 / 255.0,
            0xFF as f32 / 255.0,
        )))
        .insert_resource(Msaa::default());
    }
}
