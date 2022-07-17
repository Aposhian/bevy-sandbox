use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::health::CollisionDamage;
use crate::health::Health;

pub struct BallPlugin;

/// Resource for holding texture atlas
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
        app.add_event::<BallSpawnEvent>()
            .init_resource::<BallTextureHandle>()
            .add_system(spawn);
    }
}

#[derive(Component)]
pub struct BallTag;

#[derive(Bundle)]
pub struct BallBundle {
    tag: BallTag,
    collision_damage: CollisionDamage,
    rigid_body: RigidBody,
    collider: Collider,
    collision_groups: CollisionGroups,
    active_events: ActiveEvents,
    locked_axes: LockedAxes,
    gravity_scale: GravityScale,
    restitution: Restitution,
    velocity: Velocity,
    health: Health,
    #[bundle]
    sprite_bundle: SpriteBundle,
}

impl Default for BallBundle {
    fn default() -> Self {
        BallBundle {
            tag: BallTag,
            rigid_body: Default::default(),
            collider: Collider::ball(0.1),
            collision_groups: CollisionGroups::new(0b0011, 0b0011),
            active_events: ActiveEvents::COLLISION_EVENTS,
            locked_axes: LockedAxes::ROTATION_LOCKED,
            gravity_scale: GravityScale(0.0),
            velocity: Default::default(),
            restitution: Restitution {
                coefficient: 1.0,
                combine_rule: CoefficientCombineRule::Average,
            },
            collision_damage: CollisionDamage { damage: 1 },
            health: Health::from_max(1),
            sprite_bundle: SpriteBundle::default(),
        }
    }
}

pub struct BallSpawnEvent {
    pub transform: Transform,
    pub velocity: Vec2,
}

impl Default for BallSpawnEvent {
    fn default() -> Self {
        BallSpawnEvent {
            transform: Transform::identity(),
            velocity: Vec2::ZERO,
        }
    }
}

/// Spawn entities in response to spawn events
fn spawn(
    mut commands: Commands,
    mut spawn_events: EventReader<BallSpawnEvent>,
    texture_handle: Res<BallTextureHandle>,
) {
    for spawn_event in spawn_events.iter() {
        commands.spawn_bundle(BallBundle {
            velocity: Velocity {
                linvel: spawn_event.velocity.into(),
                ..Default::default()
            },
            sprite_bundle: SpriteBundle {
                texture: texture_handle.0.clone(),
                transform: spawn_event.transform,
                ..Default::default()
            },
            ..Default::default()
        });
    }
}
