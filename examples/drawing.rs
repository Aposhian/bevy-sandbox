use bevy::prelude::*;
use bevy_prototype_lyon::prelude::*;

fn main() {
    App::new()
        .insert_resource(Msaa::default())
        .add_plugins(DefaultPlugins)
        .add_plugin(ShapePlugin)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands) {
    let builder = GeometryBuilder::new()
        .add(&shapes::Line(Vec2::ZERO, Vec2::X * 100.0))
        .add(&shapes::Line(Vec2::X * 100.0, Vec2::new(100.0, 100.0)));

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(builder.build(
        DrawMode::Stroke(StrokeMode {
            options: StrokeOptions::default().with_line_width(10.0),
            color: Color::BLACK
        }),
        Transform::default()
    ));
}
