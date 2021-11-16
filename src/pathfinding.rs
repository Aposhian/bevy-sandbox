use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::na::Isometry2;
use bevy_prototype_lyon::prelude::*;
use pathfinding::prelude::astar;
use std::f32::consts::TAU;
use bevy::math::Mat2;
use std::ops::Add;
use std::ops::Sub;

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
    points: Vec<Vec2>
}

pub struct PathVisualization(Entity);

const MAX_TOI: f32 = 0.1;

const THETA_STEPS: u8 = 4;

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
        Vec2::new(self.0 as f32 / GRID_SCALE as f32, self.1 as f32 / GRID_SCALE as f32)
    }
}

fn compute_path_to_goal(
    mut commands: Commands,
    query: Query<(Entity, &RigidBodyPosition, &ColliderShape, &GoalPosition), Or<(Added<GoalPosition>, Changed<GoalPosition>)>>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery
) {
    for (
        entity,
        RigidBodyPosition { position: start, .. },
        shape,
        GoalPosition { position: goal }
    ) in query.iter() {
        let start_grid = GridPoint::from(Vec2::from(start.translation));
        let goal_grid = GridPoint::from(Vec2::from(goal.translation));
        info!("start_grid: {:?}, goal_grid: {:?}", start_grid, goal_grid);
        let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

        let result = astar(
            &start_grid,
            |position| {
                let query_pipeline = &query_pipeline;
                let collider_set = &collider_set;
                (0..THETA_STEPS).map(move |theta_step| {
                        let position = position.clone();
                        let theta: f32 =  theta_step as f32 * (TAU / THETA_STEPS as f32);
                        let vec_position: Vec2 = position.into();
                        let direction: Vec2 = Mat2::from_angle(theta) * Vec2::X;
                        let direction = direction.normalize_or_zero();

                        let toi = match query_pipeline.cast_shape(
                            collider_set,
                            &vec_position.into(),
                            &direction.into(),
                            &**shape,
                            MAX_TOI,
                            InteractionGroups::new(0b0100, 0b0100),
                            Some(&|handle| {
                                handle != entity.handle()
                            })
                        ) {
                            Some((_, toi)) => toi.toi,
                            None => MAX_TOI
                        };
                        let next = GridPoint::from(toi * direction);
                        (position + next, position.distance(next))
                    })
                    .filter(|(next, _)| *next != *position) // not sure if this is necessary
                    .collect::<Vec<(GridPoint, i32)>>().into_iter()
            },
            |position| {
                position.distance(goal_grid)
            },
            |position| *position == goal_grid
        );

        if let Some((path, _)) = result {
            commands.entity(entity)
                .insert(Path {
                    points: path.iter().map(|&point| { point.into() }).collect()
                });
        } else {
            warn!("no path found");
        }
    }
}

fn draw_paths(
    mut commands: Commands,
    rc: Res<RapierConfiguration>,
    query: Query<(Entity, &Path), Or<(Added<Path>, Changed<Path>)>>,
    viz_query: Query<(Entity, &PathVisualization)>
) {
    for (entity, path) in query.iter() {
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
        ).id();

        if let Ok((_,PathVisualization(entity))) = viz_query.get(entity) {
            commands.entity(*entity).despawn();
        }

        commands.entity(entity)
            .insert(PathVisualization(line_entity));
    }
}