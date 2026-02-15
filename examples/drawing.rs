use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(ShapePlugin)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let path = ShapePath::new()
        .move_to(Vec2::ZERO)
        .line_to(Vec2::X * 100.0)
        .move_to(Vec2::X * 100.0)
        .line_to(Vec2::new(100.0, 100.0))
        .build();

    commands.spawn(Camera2d);
    commands.spawn((
        ShapeBuilder::with(&path)
            .stroke((Color::BLACK, 10.0))
            .build(),
        Transform::default(),
    ));
}
