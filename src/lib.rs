use avian2d::prelude::*;
use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;
use bevy_prototype_lyon::plugin::ShapePlugin;

mod ai;
mod ball;
mod camera;
mod ecs;
mod health;
mod input;
pub mod obstacle;
mod pathfinding;
mod pathfollowing;
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

/// Pixels per physics meter, used to convert between world (pixel) coordinates
/// and game-logic "meter" coordinates.
pub const PIXELS_PER_METER: f32 = 32.0;

pub struct SandboxPlugins;

/// Wrapper plugin to add avian2d PhysicsPlugins (which is a PluginGroup)
struct PhysicsSetup;

impl Plugin for PhysicsSetup {
    fn build(&self, app: &mut App) {
        app.add_plugins(PhysicsPlugins::default().with_length_unit(PIXELS_PER_METER));
    }
}

impl PluginGroup for SandboxPlugins {
    fn build(self) -> PluginGroupBuilder {
        let builder = PluginGroupBuilder::start::<Self>().add(PhysicsSetup);

        #[cfg(feature = "physics_debug")]
        let builder = builder.add(PhysicsDebugPlugin);

        builder
            .add(DefaultResources)
            .add(InputPlugin)
            .add(SimpleFigurePlugin)
            .add(CameraPlugin)
            .add(BallPlugin)
            .add(HealthPlugin)
            .add(PathfindingPlugin)
            .add(ShapePlugin)
            .add(PathfollowingPlugin)
            .add(AiPlugin)
            .add(DespawnPlugin)
    }
}

pub struct DefaultResources;

impl Plugin for DefaultResources {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(Color::srgb(
            0xF9 as f32 / 255.0,
            0xF9 as f32 / 255.0,
            0xFF as f32 / 255.0,
        )))
        .insert_resource(Gravity(Vec2::ZERO));
    }
}
