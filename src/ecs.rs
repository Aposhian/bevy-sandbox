use bevy::prelude::*;
use std::ops::{Deref, DerefMut};

use crate::game_state::GameState;

pub struct DespawnPlugin;

impl Plugin for DespawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DespawnEvent>()
            .add_systems(Last, despawn.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Message)]
pub struct DespawnEvent(pub Entity);

/// Essentially the same as Children, but without relative
/// Transforms.
#[derive(Component)]
pub struct BondedEntities(pub Vec<Entity>);

impl Deref for BondedEntities {
    type Target = Vec<Entity>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for BondedEntities {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

fn despawn(
    mut commands: Commands,
    q: Query<&BondedEntities>,
    mut ev: MessageReader<DespawnEvent>,
) {
    for DespawnEvent(entity) in ev.read() {
        if let Ok(BondedEntities(bonded_entities)) = q.get(*entity) {
            for bonded_entity in bonded_entities {
                commands.entity(*bonded_entity).despawn();
            }
        }
        commands.entity(*entity).despawn();
    }
}
