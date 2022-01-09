use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::na::Isometry2;
use bevy_prototype_lyon::prelude::*;
use pathfinding::prelude::astar;
use std::f32::consts::TAU;
use bevy::math::Mat2;
use std::ops::Add;
use std::ops::Sub;

use crate::ecs::BondedEntities;
use crate::ecs::DespawnEvent;
use crate::input::PlayerTag;
use crate::costmap::{COSTMAP_SIZE, SharedCostmap};

pub struct PathfindingPlugin;


impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system(compute_path_to_goal.system())
            .add_system(draw_paths.system());
    }
}

pub struct GoalPosition {
    pub position: Isometry2<f32>
}

impl Default for GoalPosition {
    fn default() -> Self {
        GoalPosition {
            position: Isometry::identity()
        }
    }
}
pub struct Path {
    pub points: Vec<Vec2>
}

fn euclidean_distance(a: (usize, usize), b: (usize, usize)) -> usize {
    let squared_euclidean_distance = (a.0 - b.0).pow(2) + (a.1 - b.0).pow(2);
    (squared_euclidean_distance as f32).sqrt() as usize
}

fn compute_path_to_goal(
    costmap: Res<SharedCostmap>,
    mut commands: Commands,
    player: Query<Entity, With<PlayerTag>>,
    query: Query<(Entity, &ColliderFlags, &RigidBodyPosition, &ColliderShape, &GoalPosition), Or<(Added<GoalPosition>, Changed<GoalPosition>)>>,
) {

    for (
        entity,
        ColliderFlags { collision_groups: ig, .. },
        RigidBodyPosition { position: start, .. },
        shape,
        GoalPosition { position: goal }
    ) in query.iter() {
        let start = costmap.to_row_column(start.translation.into());
        let goal = costmap.to_row_column(goal.translation.into());

        let result = astar(
            &start,
            |cell| {
                let previous_row = cell.0.saturating_sub(1);
                let previous_column = cell.1.saturating_sub(1);
                let next_row = std::cmp::min(cell.0 + 1, COSTMAP_SIZE - 1);
                let next_column = std::cmp::min(cell.1 + 1, COSTMAP_SIZE - 1);
                let mut v = Vec::new();
                for r in previous_row..=next_row {
                    for c in previous_column..=next_column {
                        if !costmap.data[r][c].interaction_groups.test(*ig) {
                            let p = (r,c);
                            v.push(((r,c), euclidean_distance(p, *cell)));
                        }
                    }
                }
                v
            },
            |cell| {
                euclidean_distance(*cell, goal)
            },
            |cell| *cell == goal
        );

        if let Some((path, _)) = result {
            commands.entity(entity)
                .insert(Path {
                    points: path.iter().map(|&point| { costmap.to_physics_position(point) }).collect()
                });
        } else {
            warn!("no path found");
        }
    }
}

fn draw_paths(
    mut commands: Commands,
    rc: Res<RapierConfiguration>,
    mut path_query: Query<(Entity, &Path, Option<&mut BondedEntities>), Or<(Added<Path>, Changed<Path>)>>,
    mut despawn: EventWriter<DespawnEvent>
) {
    for (path_entity, path, bonded_entities) in path_query.iter_mut() {
        info!("Draw path");
        let mut builder = GeometryBuilder::new();

        for points in path.points.windows(2) {
            if let [point1, point2] = points {
                builder.add(&shapes::Line(*point1 * rc.scale, *point2 * rc.scale));
            }
        }

        let line_entity = commands.spawn_bundle(builder
            .build(
                ShapeColors::new(Color::RED),
                DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))
            )
        )
        .id();

        if let Some(mut bonded_entities) = bonded_entities {
            for entity in bonded_entities.iter() {
                despawn.send(DespawnEvent(*entity));
            }
            bonded_entities.clear();
            bonded_entities.push(line_entity);
        } else {
            commands.entity(path_entity).insert(BondedEntities(vec![line_entity]));
        }
    }
}
