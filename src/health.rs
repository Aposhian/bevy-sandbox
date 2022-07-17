use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::ecs::DespawnEvent;

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(damage).add_system(health_despawner);
    }
}

#[derive(Component)]
pub struct Health {
    pub max: i32,
    pub current: i32,
}

impl Health {
    pub fn from_max(max: i32) -> Self {
        Health { max, current: max }
    }
}

#[derive(Component)]
pub struct CollisionDamage {
    pub damage: i32,
}

fn health_despawner(
    q: Query<(Entity, &Health), Changed<Health>>,
    mut despawn: EventWriter<DespawnEvent>,
) {
    for (entity, health) in q.iter() {
        if health.current <= 0 {
            despawn.send(DespawnEvent(entity));
        }
    }
}

fn damage(
    damager_query: Query<&CollisionDamage>,
    mut health_query: Query<&mut Health>,
    mut contact_events: EventReader<CollisionEvent>,
) {
    for contact_event in contact_events.iter() {
        if let CollisionEvent::Started(c1, c2, _) = contact_event {
            for (damager, damageable) in [(c1, c2), (c2, c1)] {
                if let Ok(CollisionDamage { damage }) = damager_query.get(*damager) {
                    if let Ok(mut health) = health_query.get_mut(*damageable) {
                        health.current -= damage;
                    }
                }
            }
        }
    }
}
