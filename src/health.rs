use std::collections::HashSet;

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::ecs::DespawnEvent;
use crate::game_state::GameState;

pub struct HealthPlugin;

impl Plugin for HealthPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (damage, health_despawner, tick_damage_cooldown)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

/// Category of damage, used to filter what an entity is vulnerable to.
#[derive(Clone, Copy, PartialEq, Debug)]
#[repr(u32)]
pub enum DamageKind {
    /// Damage from a projectile hitting this entity
    Projectile = 0,
    /// Damage from physical impact (e.g. bouncing off a surface)
    Impact = 1,
}

impl DamageKind {
    /// Returns a mask containing only this damage kind.
    pub const fn mask(self) -> DamageKindMask {
        DamageKindMask(1 << self as u32)
    }
}

/// Bitmask of [`DamageKind`] values. An entity only takes damage whose kind is in its mask.
#[derive(Clone, Copy, Debug)]
pub struct DamageKindMask(pub u32);

impl DamageKindMask {
    pub const NONE: Self = DamageKindMask(0);
    pub const ALL: Self = DamageKindMask(u32::MAX);

    pub fn contains(self, kind: DamageKind) -> bool {
        self.0 & (1 << kind as u32) != 0
    }
}

impl std::ops::BitOr for DamageKindMask {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        DamageKindMask(self.0 | rhs.0)
    }
}

#[derive(Component)]
pub struct Health {
    pub max: i32,
    pub current: i32,
    /// Which damage kinds actually affect this entity.
    pub vulnerable_to: DamageKindMask,
}

impl Health {
    pub fn new(max: i32, vulnerable_to: DamageKindMask) -> Self {
        Health {
            max,
            current: max,
            vulnerable_to,
        }
    }
}

/// Deals damage of a given kind to whatever entity this entity collides with.
#[derive(Component)]
pub struct CollisionDamage {
    pub damage: i32,
    pub kind: DamageKind,
}

/// Entity takes damage to itself whenever it collides with anything.
#[derive(Component)]
pub struct CollisionSelfDamage {
    pub damage: i32,
    pub kind: DamageKind,
}

/// Active damage cooldown - entity cannot take damage while this component exists.
/// Removed automatically when the timer expires.
#[derive(Component)]
pub struct DamageCooldown(Timer);

const DAMAGE_COOLDOWN_SECS: f32 = 1.0;

fn tick_damage_cooldown(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DamageCooldown)>,
) {
    for (entity, mut cooldown) in query.iter_mut() {
        cooldown.0.tick(time.delta());
        if cooldown.0.just_finished() {
            commands.entity(entity).remove::<DamageCooldown>();
        }
    }
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
    mut commands: Commands,
    damager_query: Query<&CollisionDamage>,
    self_damage_query: Query<&CollisionSelfDamage>,
    mut health_query: Query<(&mut Health, Option<&DamageCooldown>)>,
    collisions: Collisions,
) {
    let mut damaged_this_frame: HashSet<Entity> = HashSet::new();

    for contacts in collisions.iter() {
        let Some(e1) = contacts.body1 else {
            continue;
        };
        let Some(e2) = contacts.body2 else {
            continue;
        };

        // Directed damage: one entity's CollisionDamage applies to the other's Health
        for (damager, damageable) in [(e1, e2), (e2, e1)] {
            if damaged_this_frame.contains(&damageable) {
                continue;
            }
            let Ok(cd) = damager_query.get(damager) else {
                continue;
            };
            let Ok((mut health, cooldown)) = health_query.get_mut(damageable) else {
                continue;
            };
            if cooldown.is_some() || !health.vulnerable_to.contains(cd.kind) {
                continue;
            }
            health.current -= cd.damage;
            damaged_this_frame.insert(damageable);
            commands
                .entity(damageable)
                .insert(DamageCooldown(Timer::from_seconds(
                    DAMAGE_COOLDOWN_SECS,
                    TimerMode::Once,
                )));
        }

        // Self-damage: entity takes damage when it collides with anything
        for self_entity in [e1, e2] {
            if damaged_this_frame.contains(&self_entity) {
                continue;
            }
            let Ok(sd) = self_damage_query.get(self_entity) else {
                continue;
            };
            let Ok((mut health, cooldown)) = health_query.get_mut(self_entity) else {
                continue;
            };
            if cooldown.is_some() || !health.vulnerable_to.contains(sd.kind) {
                continue;
            }
            health.current -= sd.damage;
            damaged_this_frame.insert(self_entity);
            commands
                .entity(self_entity)
                .insert(DamageCooldown(Timer::from_seconds(
                    DAMAGE_COOLDOWN_SECS,
                    TimerMode::Once,
                )));
        }
    }
}
