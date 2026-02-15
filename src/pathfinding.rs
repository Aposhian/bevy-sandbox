use avian2d::prelude::*;
use bevy::math::Mat2;
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use pathfinding::prelude::astar;
use std::f32::consts::TAU;
use std::ops::Add;
use std::ops::Sub;

use crate::ecs::BondedEntities;
use crate::ecs::DespawnEvent;
use crate::input::PlayerTag;
use crate::simple_figure::GameLayer;
use crate::PIXELS_PER_METER;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (compute_path_to_goal, draw_paths));
    }
}

#[derive(Component)]
pub struct GoalPosition {
    pub position: Vec2,
}

impl Default for GoalPosition {
    fn default() -> Self {
        GoalPosition {
            position: Vec2::ZERO,
        }
    }
}

#[derive(Component)]
pub struct Path {
    pub points: Vec<Vec2>,
}

const THETA_STEPS: u8 = 8;

const GRID_SCALE: u8 = 10;

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
struct GridPoint(i32, i32);

impl GridPoint {
    fn norm(self) -> i32 {
        (self.squared_norm() as f32).sqrt() as i32
    }

    fn squared_norm(self) -> i32 {
        self.0.pow(2) + self.1.pow(2)
    }

    fn distance(self, other: Self) -> i32 {
        (self - other).norm()
    }
}

impl Add for GridPoint {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1)
    }
}

impl Sub for GridPoint {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0, self.1 - other.1)
    }
}

impl From<Vec2> for GridPoint {
    fn from(value: Vec2) -> Self {
        let rounded = (GRID_SCALE as f32 * value).round();
        GridPoint(rounded.x as i32, rounded.y as i32)
    }
}

impl From<GridPoint> for Vec2 {
    fn from(gp: GridPoint) -> Vec2 {
        Vec2::new(
            gp.0 as f32 / GRID_SCALE as f32,
            gp.1 as f32 / GRID_SCALE as f32,
        )
    }
}

const MAX_TOI: f32 = 1.0;

const INFLATION_LAYER: f32 = 0.2;

// Default character half-extents in meters
const COLLIDER_HALF_W: f32 = 0.18;
const COLLIDER_HALF_H: f32 = 0.40;

fn compute_path_to_goal(
    mut commands: Commands,
    player: Query<Entity, With<PlayerTag>>,
    query: Query<
        (Entity, &Transform, &GoalPosition),
        Or<(Added<GoalPosition>, Changed<GoalPosition>)>,
    >,
    spatial_query: SpatialQuery,
) {
    let player_entity = player.iter().next();

    for (entity, transform, GoalPosition { position: goal }) in query.iter() {
        // Convert pixel positions to physics-meter positions for pathfinding grid
        let start_pos = transform.translation.truncate() / PIXELS_PER_METER;
        let goal_pos = *goal / PIXELS_PER_METER;

        let start_grid = GridPoint::from(start_pos);
        let goal_grid = GridPoint::from(goal_pos);
        info!("start_grid: {:?}, goal_grid: {:?}", start_grid, goal_grid);

        let mut excluded = vec![entity];
        if let Some(player) = player_entity {
            excluded.push(player);
        }

        let filter = SpatialQueryFilter::from_mask(GameLayer::Wall)
            .with_excluded_entities(excluded);

        // Inflate the collider shape for pathfinding margin
        let inflated_shape = Collider::rectangle(
            (COLLIDER_HALF_W + INFLATION_LAYER) * 2.0 * PIXELS_PER_METER,
            (COLLIDER_HALF_H + INFLATION_LAYER) * 2.0 * PIXELS_PER_METER,
        );
        let config = ShapeCastConfig::from_max_distance(MAX_TOI * PIXELS_PER_METER);

        let result = astar(
            &start_grid,
            |position| {
                (0..THETA_STEPS)
                    .map(|theta_step| {
                        let position = *position;
                        let theta: f32 = theta_step as f32 * (TAU / THETA_STEPS as f32);
                        let vec_position: Vec2 = position.into();
                        let direction: Vec2 = Mat2::from_angle(theta) * Vec2::X;
                        let direction = direction.normalize_or_zero();

                        let origin = vec_position * PIXELS_PER_METER;
                        let dir = Dir2::new(direction).unwrap_or(Dir2::X);

                        let toi = match spatial_query.cast_shape(
                            &inflated_shape,
                            origin,
                            0.0,
                            dir,
                            &config,
                            &filter,
                        ) {
                            Some(hit) => hit.distance / PIXELS_PER_METER,
                            None => MAX_TOI,
                        };

                        let next = position + GridPoint::from(toi * direction);
                        let min_x = std::cmp::min(position.0, next.0);
                        let max_x = std::cmp::max(position.0, next.0);
                        let min_y = std::cmp::min(position.1, next.1);
                        let max_y = std::cmp::max(position.1, next.1);
                        Iterator::zip(min_x..=max_x, min_y..=max_y)
                            .map(move |(x, y)| {
                                let p = GridPoint(x, y);
                                (p, position.distance(p))
                            })
                            .collect::<Vec<_>>()
                    })
                    .flatten()
                    .filter(|(next, _)| *next != *position)
                    .collect::<Vec<(GridPoint, i32)>>()
                    .into_iter()
            },
            |position| position.distance(goal_grid),
            |position| *position == goal_grid,
        );

        if let Some((path, _)) = result {
            commands.entity(entity).insert(Path {
                points: path.iter().map(|&point| Vec2::from(point)).collect(),
            });
        } else {
            warn!("no path found");
        }
    }
}

fn draw_paths(
    mut commands: Commands,
    mut path_query: Query<
        (Entity, &Path, Option<&mut BondedEntities>),
        Or<(Added<Path>, Changed<Path>)>,
    >,
    mut despawn: MessageWriter<DespawnEvent>,
) {
    for (path_entity, path, bonded_entities) in path_query.iter_mut() {
        info!("Draw path");

        let mut shape_path = ShapePath::new();
        for points in path.points.windows(2) {
            if let [point1, point2] = points {
                shape_path = shape_path
                    .move_to(*point1 * PIXELS_PER_METER)
                    .line_to(*point2 * PIXELS_PER_METER);
            }
        }

        let line_entity = commands
            .spawn((
                ShapeBuilder::with(&shape_path)
                    .stroke((Color::srgb(1.0, 0.0, 0.0), 2.0))
                    .build(),
                Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
            ))
            .id();

        if let Some(mut bonded_entities) = bonded_entities {
            for entity in bonded_entities.iter() {
                despawn.write(DespawnEvent(*entity));
            }
            bonded_entities.clear();
            bonded_entities.push(line_entity);
        } else {
            commands
                .entity(path_entity)
                .insert(BondedEntities(vec![line_entity]));
        }
    }
}
