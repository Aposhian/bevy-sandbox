use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use nalgebra::Isometry2;

pub struct BallPlugin;

impl Plugin for BallPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_event::<BallSpawnEvent>()
            .add_system(spawn.system());
    }
}

#[derive(Bundle)]
pub struct BallBundle {
    #[bundle]
    rigid_body_bundle: RigidBodyBundle,
    position_sync: RigidBodyPositionSync,
    #[bundle]
    collider_bundle: ColliderBundle,
}

impl Default for BallBundle {
    fn default() -> Self {
        BallBundle {
            rigid_body_bundle: Default::default(),
            position_sync: RigidBodyPositionSync::Discrete,
            collider_bundle: ColliderBundle::default()
        }
    }
}

pub struct BallSpawnEvent {
    pub position: Isometry2<f32>
}

impl Default for BallSpawnEvent {
    fn default() -> Self {
        BallSpawnEvent {
            position: Isometry2::identity()
        }
    }
}

/// Spawn entities in response to spawn events
fn spawn(
    mut commands: Commands,
    mut spawn_events: EventReader<BallSpawnEvent>
) {
    for spawn_event in spawn_events.iter() {
        commands.spawn_bundle(BallBundle {
            rigid_body_bundle: RigidBodyBundle {
                mass_properties: RigidBodyMassPropsFlags::ROTATION_LOCKED.into(),
                forces: RigidBodyForces {
                    gravity_scale: 0.0,
                    ..Default::default()
                },
                velocity: RigidBodyVelocity {
                    linvel: [2.0, 0.0].into(),
                    ..Default::default()
                },
                position: spawn_event.position.into(),
                ..Default::default()
            },
            collider_bundle: ColliderBundle {
                shape: ColliderShape::ball(0.1),
                mass_properties: ColliderMassProps::Density(0.001),
                material: ColliderMaterial {
                    restitution: 1.0,
                    restitution_combine_rule: CoefficientCombineRule::Average,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(ColliderDebugRender::with_id(2));
    }
}