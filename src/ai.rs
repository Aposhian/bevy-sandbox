use bevy::prelude::*;
use bevy_rapier2d::prelude::*;

use crate::input::PlayerTag;
use crate::simple_figure::SimpleFigureTag;
use crate::pathfinding::GoalPosition;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_startup_system(setup.system())
            .add_system(zombie_follow.system());
    }
}

struct ReplanTimer(Timer);

fn setup(
    mut commands: Commands
) {
    commands.insert_resource(ReplanTimer(Timer::from_seconds(0.5, true)));
}

fn zombie_follow(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ReplanTimer>,
    player: Query<&RigidBodyPosition, With<PlayerTag>>,
    zombies: Query<Entity, (Without<PlayerTag>, With<SimpleFigureTag>)>
) {
    timer.0.tick(time.delta());
    if timer.0.finished() {
        if let Some(player_position) = player.iter().next() {
            for entity in zombies.iter() {
                info!("Resetting zombie goal");
                commands.entity(entity)
                    .insert(GoalPosition {
                        position: player_position.position
                    });
            }
        }
    }
}
