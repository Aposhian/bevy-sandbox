use bevy::prelude::*;

use crate::game_state::GameState;
use crate::input::PlayerTag;
use crate::net::GuestTag;
use crate::pathfinding::GoalPosition;
use crate::simple_figure::SimpleFigureTag;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, zombie_follow.run_if(in_state(GameState::Playing)));
    }
}

#[derive(Resource)]
struct ReplanTimer(Timer);

fn setup(mut commands: Commands) {
    commands.insert_resource(ReplanTimer(Timer::from_seconds(0.5, TimerMode::Repeating)));
}

fn zombie_follow(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<ReplanTimer>,
    player: Query<&Transform, With<PlayerTag>>,
    zombies: Query<Entity, (Without<PlayerTag>, Without<GuestTag>, With<SimpleFigureTag>)>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        if let Some(player_transform) = player.iter().next() {
            let player_pos = player_transform.translation.truncate();
            for entity in zombies.iter() {
                debug!("Resetting zombie goal");
                commands.entity(entity).insert(GoalPosition {
                    position: player_pos,
                });
            }
        }
    }
}
