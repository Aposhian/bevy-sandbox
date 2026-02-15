use avian2d::prelude::*;
use bevy::prelude::*;

use crate::simple_figure::GameLayer;
use crate::PIXELS_PER_METER;

pub fn spawn(mut commands: Commands) {
    commands.spawn((
        RigidBody::Static,
        Collider::rectangle(2.0 * PIXELS_PER_METER, 2.0 * PIXELS_PER_METER),
        CollisionLayers::new(
            LayerMask::from([GameLayer::Wall]),
            LayerMask::from([GameLayer::Character, GameLayer::Ball, GameLayer::Wall]),
        ),
        Transform::from_translation(Vec3::new(
            3.0 * PIXELS_PER_METER,
            3.0 * PIXELS_PER_METER,
            0.0,
        )),
    ));
}
