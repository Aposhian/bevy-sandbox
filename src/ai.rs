use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::input::PlayerTag;
use crate::simple_figure::SimpleFigureTag;
use crate::pathfinding::GoalPosition;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system(zombie_follow.system());
    }
}

fn zombie_follow(
    mut commands: Commands,
    player: Query<&RigidBodyPosition, With<PlayerTag>>,
    zombies: Query<Entity, (Without<PlayerTag>, With<SimpleFigureTag>)>
) {
    if let Some(player_position) = player.iter().next() {
        for entity in zombies.iter() {
            commands.entity(entity)
                .insert(GoalPosition {
                    position: player_position.position
                });
        }
    }
}
