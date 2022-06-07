use bevy::math::Mat2;
use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;
use bevy_rapier2d::na::Isometry2;
use bevy_rapier2d::prelude::*;
use pathfinding::prelude::astar;
use std::f32::consts::TAU;
use std::ops::Add;
use std::ops::Sub;

use crate::ecs::BondedEntities;
use crate::ecs::DespawnEvent;
use crate::input::PlayerTag;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(compute_path_to_goal);
        // .add_system(draw_paths);
    }
}

#[derive(Component)]
pub struct GoalPosition {
    pub position: Isometry2<f32>,
}

impl Default for GoalPosition {
    fn default() -> Self {
        GoalPosition {
            position: Isometry::identity(),
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

impl Into<Vec2> for GridPoint {
    fn into(self) -> Vec2 {
        Vec2::new(
            self.0 as f32 / GRID_SCALE as f32,
            self.1 as f32 / GRID_SCALE as f32,
        )
    }
}

const MAX_TOI: f32 = 1.0; // seconds

const INFLATION_LAYER: f32 = 0.2; // m

fn compute_path_to_goal(
    mut commands: Commands,
    player: Query<Entity, With<PlayerTag>>,
    query: Query<
        (
            Entity,
            &RigidBodyPositionComponent,
            &ColliderShapeComponent,
            &GoalPosition,
        ),
        Or<(Added<GoalPosition>, Changed<GoalPosition>)>,
    >,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery,
) {
    let player_entity = player.iter().next();

    for (entity, start_position, shape, GoalPosition { position: goal }) in query.iter() {
        let start_grid = GridPoint::from(Vec2::from(start_position.position.translation));
        let goal_grid = GridPoint::from(Vec2::from(goal.translation));
        let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

        let result = astar(
            &start_grid,
            |position| {
                let query_pipeline = &query_pipeline;
                let collider_set = &collider_set;
                (0..THETA_STEPS)
                    .map(move |theta_step| {
                        let position = position.clone();
                        let theta: f32 = theta_step as f32 * (TAU / THETA_STEPS as f32);
                        let vec_position: Vec2 = position.into();
                        let direction: Vec2 = Mat2::from_angle(theta) * Vec2::X;
                        let direction = direction.normalize_or_zero();

                        let inflated_shape = match shape.shape_type() {
                            ShapeType::Cuboid => {
                                let cuboid = shape.as_cuboid().unwrap();
                                ColliderShape::cuboid(
                                    cuboid.half_extents[0] + INFLATION_LAYER,
                                    cuboid.half_extents[1] + INFLATION_LAYER,
                                )
                            }
                            _ => ColliderShape::cuboid(INFLATION_LAYER, INFLATION_LAYER),
                        };

                        let toi = match query_pipeline.cast_shape(
                            collider_set,
                            &vec_position.into(),
                            &direction.into(),
                            &*inflated_shape,
                            MAX_TOI,
                            InteractionGroups::new(0b0100, 0b0100),
                            Some(&|handle| {
                                handle != entity.handle()
                                    && match player_entity {
                                        Some(player) => handle != player.handle(),
                                        None => true,
                                    }
                            }),
                        ) {
                            Some((_, toi)) => toi.toi,
                            None => MAX_TOI,
                        };
                        let next = position + GridPoint::from(toi * direction);
                        let min_x = std::cmp::min(position.0, next.0);
                        let max_x = std::cmp::max(position.0, next.0);
                        let min_y = std::cmp::min(position.1, next.1);
                        let max_y = std::cmp::max(position.1, next.1);
                        Iterator::zip(min_x..=max_x, min_y..=max_y).map(move |(x, y)| {
                            let p = GridPoint(x, y);
                            (p, position.distance(p))
                        })
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
                points: path.iter().map(|&point| point.into()).collect(),
            });
        } else {
            warn!("no path found");
        }
    }
}

fn draw_paths(
    mut commands: Commands,
    rc: Res<RapierConfiguration>,
    mut path_query: Query<
        (Entity, &Path, Option<&mut BondedEntities>),
        Or<(Added<Path>, Changed<Path>)>,
    >,
    mut despawn: EventWriter<DespawnEvent>,
) {
    for (path_entity, path, bonded_entities) in path_query.iter_mut() {
        info!("Draw path");
        let mut builder = GeometryBuilder::new();

        for points in path.points.windows(2) {
            if let [point1, point2] = points {
                builder = builder.add(&shapes::Line(*point1 * rc.scale, *point2 * rc.scale));
            }
        }

        let line_entity = commands
            .spawn_bundle(builder.build(
                DrawMode::Stroke(StrokeMode {
                    options: StrokeOptions::default().with_line_width(2.0),
                    color: Color::RED,
                }),
                Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
            ))
            .id();

        if let Some(mut bonded_entities) = bonded_entities {
            for entity in bonded_entities.iter() {
                despawn.send(DespawnEvent(*entity));
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
