use bevy::prelude::*;
use bevy_rapier2d::prelude::*;
use bevy_rapier2d::na::Isometry2;
use pathfinding::prelude::astar;

pub struct GoalPosition {
    position: Isometry2<f32>
}

fn compute_path_to_goal(
    query: Query<(&RigidBodyPosition, &ColliderShape, &GoalPosition)>,
    query_pipeline: Res<QueryPipeline>,
    collider_query: QueryPipelineColliderComponentsQuery
) {
    for (RigidBodyPosition { position, .. }, shape, GoalPosition { position: goal }) in query.iter() {
        let collider_set = QueryPipelineColliderComponentsSet(&collider_query);

        let shape_vel = Vec2::new(0.1, 0.4).into();
        let max_toi = 4.0;
    
        if let Some((_, toi)) = query_pipeline.cast_shape(
            &collider_set,
            &position,
            &shape_vel,
            &shape,
            max_toi,
            InteractionGroups::all(),
            None
        ) {
        }
    }
}