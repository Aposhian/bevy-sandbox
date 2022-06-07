use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use nalgebra::Isometry2;

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
    #[bundle]
    rigid_body_bundle: RigidBodyBundle,
    position_sync: RigidBodyPositionSync,
    #[bundle]
    collider_bundle: ColliderBundle,
    health: Health,
    #[bundle]
    sprite_bundle: SpriteBundle,
}

impl Default for BallBundle {
    fn default() -> Self {
        BallBundle {
            tag: BallTag,
            rigid_body_bundle: Default::default(),
            collision_damage: CollisionDamage { damage: 1 },
            position_sync: RigidBodyPositionSync::Discrete,
            collider_bundle: ColliderBundle {
                shape: ColliderShape::ball(0.1).into(),
                flags: ColliderFlags {
                    collision_groups: InteractionGroups::new(0b0011, 0b0011),
                    active_events: ActiveEvents::CONTACT_EVENTS,
                    ..Default::default()
                }
                .into(),
                mass_properties: ColliderMassProps::Density(0.001).into(),
                material: ColliderMaterial {
                    restitution: 1.0,
                    restitution_combine_rule: CoefficientCombineRule::Average,
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            },
            health: Health::from_max(1),
            sprite_bundle: SpriteBundle::default(),
        }
    }
}

pub struct BallSpawnEvent {
    pub position: Isometry2<f32>,
    pub velocity: Vec2,
}

impl Default for BallSpawnEvent {
    fn default() -> Self {
        BallSpawnEvent {
            position: Isometry2::identity(),
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
            rigid_body_bundle: RigidBodyBundle {
                mass_properties: RigidBodyMassPropsFlags::ROTATION_LOCKED.into(),
                forces: RigidBodyForces {
                    gravity_scale: 0.0,
                    ..Default::default()
                }
                .into(),
                velocity: RigidBodyVelocity {
                    linvel: spawn_event.velocity.into(),
                    ..Default::default()
                }
                .into(),
                position: spawn_event.position.into(),
                ..Default::default()
            },
            sprite_bundle: SpriteBundle {
                texture: texture_handle.0.clone(),
                transform: Transform::from_scale(Vec3::splat(1.0))
                    * Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
                ..Default::default()
            },
            ..Default::default()
        });
    }
}
