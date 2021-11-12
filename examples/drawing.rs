use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

fn main() {
    App::build()
        .insert_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_startup_system(setup.system())
        .run();
}

fn setup(mut commands: Commands) {
    let mut builder = GeometryBuilder::new();

    builder.add(&shapes::Line(Vec2::ZERO, Vec2::X * 100.0))
        .add(&shapes::Line(Vec2::X * 100.0, Vec2::new(100.0, 100.0)));

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(builder
        .build(
            ShapeColors::new(Color::BLACK),
            DrawMode::Stroke(StrokeOptions::default().with_line_width(10.0)),
            Transform::default(),
        )
    );
}