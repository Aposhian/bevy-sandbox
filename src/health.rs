use avian2d::prelude::*;
use bevy::prelude::*;

use crate::ecs::DespawnEvent;

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (damage, health_despawner));
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
    mut despawn: MessageWriter<DespawnEvent>,
) {
    for (entity, health) in q.iter() {
        if health.current <= 0 {
            despawn.write(DespawnEvent(entity));
        }
    }
}

fn damage(
    damager_query: Query<&CollisionDamage>,
    mut health_query: Query<&mut Health>,
    collisions: Collisions,
) {
    for contacts in collisions.iter() {
        let Some(e1) = contacts.body1 else { continue; };
        let Some(e2) = contacts.body2 else { continue; };

        for (damager, damageable) in [(e1, e2), (e2, e1)] {
            if let Ok(CollisionDamage { damage }) = damager_query.get(damager) {
                if let Ok(mut health) = health_query.get_mut(damageable) {
                    health.current -= damage;
                }
            }
        }
    }
}
