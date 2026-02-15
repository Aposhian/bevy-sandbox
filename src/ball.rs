use avian2d::prelude::*;
use bevy::prelude::*;

use crate::health::CollisionDamage;
use crate::health::Health;
use crate::simple_figure::GameLayer;
use crate::PIXELS_PER_METER;

pub struct BallPlugin;

/// Resource for holding ball texture
#[derive(Resource)]
pub struct BallTextureHandle(Handle<Image>);

impl FromWorld for BallTextureHandle {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let image = asset_server.load("spritesheets/baseball.png");
        BallTextureHandle(image)
    }
}

impl Plugin for BallPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<BallSpawnEvent>()
            .init_resource::<BallTextureHandle>()
            .add_systems(Update, spawn);
    }
}

#[derive(Component)]
pub struct BallTag;

#[derive(Debug, Message)]
pub struct BallSpawnEvent {
    pub position: Vec2,
    pub velocity: Vec2,
}

impl Default for BallSpawnEvent {
    fn default() -> Self {
        BallSpawnEvent {
            position: Vec2::ZERO,
            velocity: Vec2::ZERO,
        }
    }
}

/// Spawn entities in response to spawn events
fn spawn(
    mut commands: Commands,
    mut spawn_events: MessageReader<BallSpawnEvent>,
    texture_handle: Res<BallTextureHandle>,
) {
    for spawn_event in spawn_events.read() {
        commands.spawn((
            BallTag,
            CollisionDamage { damage: 1 },
            Health::from_max(1),
            Sprite::from_image(texture_handle.0.clone()),
            Transform::from_translation(Vec3::new(
                spawn_event.position.x,
                spawn_event.position.y,
                2.0,
            )),
            // Physics
            RigidBody::Dynamic,
            Collider::circle(0.1 * PIXELS_PER_METER),
            CollisionLayers::new(
                LayerMask::from([GameLayer::Character, GameLayer::Ball]),
                LayerMask::from([GameLayer::Character, GameLayer::Ball]),
            ),
            CollisionEventsEnabled,
            Restitution::new(1.0),
            ColliderDensity(0.001),
            LockedAxes::ROTATION_LOCKED,
            LinearVelocity(spawn_event.velocity),
        ));
    }
}
