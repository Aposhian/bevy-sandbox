use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::na::Isometry2;
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

const MAX_TOI: f32 = 100.0;

fn compute_path_to_goal(
    mut commands: Commands,
    query: Query<(Entity, &RigidBodyPosition, &ColliderShape, &GoalPosition), Or<(Added<GoalPosition>, Changed<GoalPosition>)>>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery
) {
    for (
        entity,
        RigidBodyPosition { position, .. },
        shape,
        GoalPosition { position: goal }
    ) in query.iter() {
        let collider_set = QueryPipelineColliderComponentsSet(&collider_query);
    
        let direction: Vec2 = goal.translation.into();

        if let Some((_, toi)) = query_pipeline.cast_shape(
            &collider_set,
            &position,
            &direction.into(),
            &**shape,
            MAX_TOI,
            InteractionGroups::all(),
            None
        ) {
            info!("inserting path: toi={}", toi.toi);
            commands.entity(entity)
                .insert(Path {
                    points: vec![
                            position.translation.into(),
                            goal.translation.into()
                        ]
                });
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