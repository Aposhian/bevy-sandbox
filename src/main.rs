use bevy::prelude::*;

const SPRITE_SHEET: &str = "spritesheet_32x32.png";
const SPRITE_WIDTH: f32 = 32.0;
const SPRITE_HEIGHT: f32 = 32.0;
const SCALE_FACTOR: f32 = 3.0;
const NUM_COLUMNS: usize = 3;
const NUM_ROWS: usize = 4;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(animate_system.system())
        .add_system(movement_system.system())
        .add_system(keyboard_control_system.system())
        .add_system(computer_control_system.system())
        .run();
}

pub struct Animation {
    pub start_index: u32,
    pub num_frames: u32
}

#[derive(Default)]
pub struct SpriteControl {
    pub velocity: Vec3
}

pub struct KeyboardControl {
    pub enabled: bool,
    pub speed: f32
}

pub struct ComputerControl {
    pub enabled: bool,
    pub velocity: Vec3
}

impl Default for KeyboardControl {
    fn default() -> Self {
        Self {
            enabled: true,
            speed: 1.0
        }
    }
}

impl Default for ComputerControl {
    fn default() -> Self {
        Self {
            enabled: true,
            velocity: Vec3::new(0.0,0.0,0.0)
        }
    }
}

fn animate_system(
    time: Res<Time>,
    mut query: Query<(&mut Timer, &mut TextureAtlasSprite, &Animation)>,
) {
    for (mut timer, mut sprite, animation) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            sprite.index = animation.start_index + ((sprite.index + 1) % animation.num_frames) as u32;
        }
    }
}

fn movement_system(
    time: Res<Time>,
    mut query: Query<(&SpriteControl, &mut Transform, &mut TextureAtlasSprite)>,
) {
    for (sprite_control, mut transform, mut texture_atlas_sprite) in query.iter_mut() {
        transform.translation += time.delta_seconds() * sprite_control.velocity;
        if sprite_control.velocity.x != 0.0 {
            texture_atlas_sprite.flip_x = sprite_control.velocity.x < 0.0;
        }
    }
}

fn keyboard_control_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut SpriteControl, &KeyboardControl)>,
) {
    for (mut sprite_control, keyboard_control) in query.iter_mut() {
        sprite_control.velocity = Vec3::new(0.0,0.0,0.0);
        if keyboard_input.pressed(KeyCode::Left) {
            sprite_control.velocity.x -= keyboard_control.speed;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            sprite_control.velocity.x += keyboard_control.speed;
        }
        if keyboard_input.pressed(KeyCode::Up) {
            sprite_control.velocity.y += keyboard_control.speed;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            sprite_control.velocity.y -= keyboard_control.speed;
        }
    }
}

fn computer_control_system(
    mut query: Query<(&mut SpriteControl, &ComputerControl)>,
) {
    for (mut sprite_control, computer_control) in query.iter_mut() {
        sprite_control.velocity = computer_control.velocity;
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load(SPRITE_SHEET);
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(SPRITE_WIDTH, SPRITE_HEIGHT), NUM_COLUMNS, NUM_ROWS);
    let texture_atlas_handle =  texture_atlases.add(texture_atlas);
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(SCALE_FACTOR)),
            ..Default::default()
        })
        .insert(Animation {
            start_index: 0,
            num_frames: NUM_COLUMNS as u32
        })
        .insert(SpriteControl {
            ..Default::default()
        })
        .insert(ComputerControl {
            velocity: Vec3::new(20.0,0.0,0.0),
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, true));

    

    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(SCALE_FACTOR)),
            sprite: TextureAtlasSprite {
                index: 3*NUM_COLUMNS as u32,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Animation {
            start_index: 3*NUM_COLUMNS as u32,
            num_frames: NUM_COLUMNS as u32
        })
        .insert(SpriteControl {
            ..Default::default()
        })
        .insert(KeyboardControl {
            speed: 100.0,
            ..Default::default()
        })
        .insert(Timer::from_seconds(0.1, true));
}
