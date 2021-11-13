use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::na::Isometry2;
use pathfinding::prelude::astar;
use std::f32::consts::TAU;
use bevy::math::Mat2;
use bevy_prototype_lyon::prelude::*;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            .add_system(compute_path_to_goal.system())
            .add_system(draw_paths.system());
    }
}

pub struct GoalPosition {
    position: Isometry2<f32>
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

const THETA_STEPS: i32 = 4;

const MAX_TOI: f32 = 4.0;

fn compute_path_to_goal(
    mut commands: Commands,
    query: Query<(Entity, &GoalPosition), Added<GoalPosition>>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery
) {
    for (
        entity,
        // RigidBodyPosition { position, .. },
        // shape,
        GoalPosition { position: goal }
    ) in query.iter() {
        // let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

        // for theta_step in 0..THETA_STEPS {
        //     let transform = Mat2::from_angle(TAU / theta_step as f32);
        //     let direction: Vec2 = transform * Vec2::X;

        //     if let Some((_, toi)) = query_pipeline.cast_shape(
        //         &collider_set,
        //         &position,
        //         &direction.into(),
        //         &**shape,
        //         MAX_TOI,
        //         InteractionGroups::all(),
        //         None
        //     ) {

        //     }
        // }

        let path = commands.spawn()
            .insert(Path {
                points: vec![
                        Vec2::new(0.0, 100.0),
                        Vec2::new(100.0, 100.0),
                        Vec2::new(200.0, 200.0)
                    ]
            }).id();
        info!("Inserting path");
        commands.entity(entity)
            .push_children(&vec![path]);
    }
}

fn draw_paths(
    mut commands: Commands,
    query: Query<&Path, Added<Path>>
) {
    for path in query.iter() {
        info!("Draw path");
        let mut builder = GeometryBuilder::new();

        for points in path.points.windows(2) {
            if let [point1, point2] = points {
                info!("adding points");
                builder.add(&shapes::Line(point1.clone(), point2.clone()));
            }
        }

        commands.spawn_bundle(OrthographicCameraBundle::new_2d());
        commands.spawn_bundle(builder
            .build(
                ShapeColors::new(Color::BLACK),
                DrawMode::Stroke(StrokeOptions::default().with_line_width(10.0)),
                Transform::default()
            )
        );
    }
}