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

const THETA_STEPS: i32 = 4;

const MAX_TOI: f32 = 4.0;

fn compute_path_to_goal(
    mut commands: Commands,
    rapier_config: Res<RapierConfiguration>,
    query: Query<(Entity, &GoalPosition), Or<(Added<GoalPosition>, Changed<GoalPosition>)>>,
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

        info!("Inserting path");
        commands.entity(entity)
            .insert(Path {
                points: vec![
                        Vec2::ZERO,
                        rapier_config.scale * Vec2::from(goal.translation)
                    ]
            });
    }
}

fn draw_paths(
    mut commands: Commands,
    query: Query<(Entity, &Path), Or<(Added<Path>, Changed<Path>)>>
) {
    for (entity, path) in query.iter() {
        info!("Draw path");
        let mut builder = GeometryBuilder::new();

        for points in path.points.windows(2) {
            if let [point1, point2] = points {
                info!("adding points");
                builder.add(&shapes::Line(point1.clone(), point2.clone()));
            }
        }

        let geometry_entity = commands.spawn_bundle(builder
            .build(
                ShapeColors::new(Color::RED),
                DrawMode::Stroke(StrokeOptions::default().with_line_width(2.0)),
                Transform::default()
            )
        ).id();

        commands.entity(entity)
            .push_children(&vec![geometry_entity]);
    }
}