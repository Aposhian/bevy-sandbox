use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::na::Isometry2;
use bevy_prototype_lyon::prelude::*;
use pathfinding::prelude::astar;
use std::f32::consts::TAU;
use bevy::math::Mat2;
use std::ops::Add;

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

const MAX_TOI: f32 = 100.0;

const TOI_SCALE: i32 = 100;

const THETA_STEPS: u8 = 8;

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct GridPoint(i32, i32);

impl Add for GridPoint {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0, self.1 + other.1)
    }
}

impl From<Vec2> for GridPoint {
    fn from(value: Vec2) -> Self {
        let rounded = value.round();
        GridPoint(rounded.x as i32, rounded.y as i32)
    }
}

impl Into<Vec2> for GridPoint {
    fn into(self) -> Vec2 {
        Vec2::new(self.0 as f32, self.1 as f32)
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
        let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

        let result = astar(
            &start_grid,
            |position| {
                let query_pipeline = &query_pipeline;
                let collider_set = &collider_set;
                (0..THETA_STEPS).map(move |theta_step| {
                        let theta: f32 =  theta_step as f32 * (TAU / THETA_STEPS as f32);
                        let vec_position: Vec2 = position.clone().into();
                        let direction: Vec2 = Mat2::from_angle(theta) * vec_position;
                        let direction = direction.normalize_or_zero();

                        // unwrap since we are setting max_toi
                        let (_, toi) = query_pipeline.cast_shape(
                            collider_set,
                            &start.translation.into(),
                            &direction.into(),
                            &**shape,
                            MAX_TOI,
                            InteractionGroups::new(0b0100, 0b0100),
                            Some(&|handle| {
                                handle != entity.handle()
                            })
                        ).unwrap();
                        (position.clone() + GridPoint::from(toi.toi * direction), (toi.toi * TOI_SCALE as f32) as i32)
                    }).collect::<Vec<(GridPoint, i32)>>().into_iter()
            },
            |position| {
                TOI_SCALE * ((position.0 - goal_grid.0).pow(2) + (position.1 - goal_grid.1).pow(2))
            },
            |position| *position == goal_grid
        );

        if let Some((path, _)) = result {
            info!("inserting path");

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
    query: Query<(Entity, &Path), Or<(Added<Path>, Changed<Path>)>>
) {
    for (entity, path) in query.iter() {
        info!("Draw path");
        let mut builder = GeometryBuilder::new();

        for points in path.points.windows(2) {
            if let [point1, point2] = points {
                info!("adding points");
                builder.add(&shapes::Line(*point1 * rc.scale, *point2 * rc.scale));
            }
        }

        commands.spawn_bundle(builder
            .build(
                ShapeColors::new(Color::RED),
                DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                Transform::from_translation(Vec3::new(0.0, 0.0, 10.0))
            )
        );
    }
}