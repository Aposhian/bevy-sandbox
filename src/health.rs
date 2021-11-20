use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::ecs::DespawnEvent;

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system(damage.system())
            .add_system(health_despawner.system());
    }
}

pub struct Health {
    pub max: i32,
    pub current: i32
}

impl Health {
    pub fn from_max(max: i32) -> Self {
        Health {
            max,
            current: max
        }
    }
}

pub struct CollisionDamage {
    pub damage: i32
}

fn health_despawner(
    q: Query<(Entity, &Health), Changed<Health>>,
    mut despawn: EventWriter<DespawnEvent>
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
    mut contact_events: EventReader<ContactEvent>,
) {
    for contact_event in contact_events.iter() {
        if let ContactEvent::Started(c1, c2) = contact_event {
            for (damager, damageable) in [(c1, c2), (c2, c1)] {
                if let Ok(CollisionDamage { damage }) = damager_query.get(damager.entity()) {
                    if let Ok(mut health) = health_query.get_mut(damageable.entity()) {
                        health.current -= damage;
                    }
                }
            }
        }
    }
}
