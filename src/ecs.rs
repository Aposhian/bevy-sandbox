use bevy::prelude::*;
use std::ops::{Deref, DerefMut};

pub struct DespawnPlugin;

impl Plugin for DespawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DespawnEvent>()
            .add_system_to_stage(CoreStage::Last, despawn);
    }
}
pub struct DespawnEvent(pub Entity);

/// Essentially the same as Children, but without relative
/// Transforms.
#[derive(Component)]
pub struct BondedEntities(pub Vec<Entity>);

impl Deref for BondedEntities {
    type Target = Vec<Entity>;
    fn deref(&self) -> &Self::Target {
        return &self.0;
    }
}

impl DerefMut for BondedEntities {
    fn deref_mut(&mut self) -> &mut Self::Target {
        return &mut self.0;
    }
}

fn despawn(mut commands: Commands, q: Query<&BondedEntities>, mut ev: EventReader<DespawnEvent>) {
    for DespawnEvent(entity) in ev.iter() {
        if let Ok(BondedEntities(bonded_entities)) = q.get(*entity) {
            for bonded_entity in bonded_entities {
                commands.entity(*bonded_entity).despawn_recursive();
            }
        }
        commands.entity(*entity).despawn_recursive();
    }
}
