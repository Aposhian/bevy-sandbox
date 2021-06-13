use bevy::prelude::*;
mod utils;
use std::f32::consts::PI;

struct SpriteSheetConfig {
    path: &'static str,
    tile_size: Vec2,
    columns: usize,
    rows: usize,
    scale_factor: f32
}

const TIE_MAN_SPRITESHEET: SpriteSheetConfig = SpriteSheetConfig {
    path: "spritesheet_32x32.png",
    tile_size: Vec2::new(0.0,0.0),
    columns: 3,
    rows: 4,
    scale_factor: 3.0
};

#[derive(Default)]
struct MoveAction {
    velocity: Vec2
}

enum MoveAnimationSet {
    UP,
    DOWN,
    RIGHT,
    LEFT,
    STATIONARY
}

#[derive(Clone)]
pub struct AnimationEffect {
    frames: std::iter::Cycle<u32>,
    flip_x: bool
}

const QUADRANT_BOUNDS : [f32; 3] = [
    Bounds {
        lower: 0.0,
        upper: PI/4.0
    },
    Bounds {
        lower: PI/4.0,
        upper: 3.0*PI/4.0
    },
    Bounds {
        lower: 3.0*PI/4.0,
        upper: PI
    }
];

impl From<&MoveAction> for MoveAnimationSet {
    fn from(value: &MoveAction) -> Self {
        let angle = value.velocity.angle_between(Vec2::splat(0.0));

        match value.velocity.max_element() {
            0.0 | -0.0 => MoveAnimationSet::STATIONARY,
            _ => match angle.which_bounds(QUADRANT_BOUNDS) {
                0 => MoveAnimationSet::RIGHT,
                1 if value.velocity.y > 0.0 => MoveAnimationSet::UP,
                1 if value.velocity.y <= 0.0 => MoveAnimationSet::DOWN,
                2 | _ => MoveAnimationSet::LEFT
            }
        }
    }
}

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(auto_input_system.system())
        .add_system(keyboard_input_system.system())
        .add_system(tie_man_animation_control_system.system())
        .add_system(animate_system.system())
        .add_system(movement_system.system())
        .run();
}

pub struct KeyboardInputBinding {
    pub enabled: bool,
    pub speed: f32
}

pub struct AutoInput {
    pub enabled: bool,
    pub velocity: Vec2
}

impl Default for KeyboardInputBinding {
    fn default() -> Self {
        Self {
            enabled: true,
            speed: 1.0
        }
    }
}

impl Default for AutoInput {
    fn default() -> Self {
        Self {
            enabled: true,
            velocity: Vec2::splat(0.0)
        }
    }
}

pub struct TieManTag;

const TIE_MAN_RIGHT : AnimationEffect = AnimationEffect {
    frames: (9_u32..11).cycle(),
    flip_x: false
};

const TIE_MAN_LEFT : AnimationEffect = AnimationEffect {
    frames: (9_u32..11).cycle(),
    flip_x: true
};

const TIE_MAN_UP : AnimationEffect = AnimationEffect {
    frames: (6_u32..9).cycle(),
    flip_x: false
};

const TIE_MAN_DOWN : AnimationEffect = AnimationEffect {
    frames: (0_u32..3).cycle(),
    flip_x: false
};

fn tie_man_animation_control_system(
    mut query: Query<(&TieManTag, &MoveAction, &mut AnimationEffect)>
) {
    for (tag, move_action, mut animation) in query.iter_mut() {
        animation = match MoveAnimationSet::from(move_action) {
            MoveAnimationSet::RIGHT => TIE_MAN_RIGHT,
            MoveAnimationSet::LEFT => TIE_MAN_LEFT,
            MoveAnimationSet::DOWN | MoveAnimationSet::STATIONARY => TIE_MAN_DOWN,
            MoveAnimationSet::UP => TIE_MAN_UP
        };
    }
}

// Runs sprite animations based on Animation components
fn animate_system(
    time: Res<Time>,
    mut query: Query<(&mut Timer, &mut TextureAtlasSprite, &AnimationEffect)>,
) {
    for (mut timer, mut sprite, animation) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            sprite.index = animation.frames.next();
            sprite.flip_x = animation.flip_x;
        }
    }
}

// Moves sprite transforms based on SpriteControl
fn movement_system(
    time: Res<Time>,
    mut query: Query<(&MoveAction, &mut Transform)>,
) {
    for (move_action, mut transform) in query.iter_mut() {
        transform.translation += time.delta_seconds() * move_action.velocity;
    }
}

fn keyboard_input_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut MoveAction, &KeyboardInputBinding)>,
) {
    for (mut move_action, keyboard_input_binding) in query.iter_mut() {
        move_action.velocity = Vec2::splat(0.0);
        if keyboard_input.pressed(KeyCode::Left) {
            move_action.velocity.x -= keyboard_input_binding.speed;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            move_action.velocity.x += keyboard_input_binding.speed;
        }
        if keyboard_input.pressed(KeyCode::Up) {
            move_action.velocity.y += keyboard_input_binding.speed;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            move_action.velocity.y -= keyboard_input_binding.speed;
        }
    }
}

impl From<&AutoInput> for MoveAction {
    fn from(value: &AutoInput) -> Self {
        MoveAction {
            velocity: value.velocity
        }
    }
}

fn auto_input_system(
    mut query: Query<(&mut MoveAction, &AutoInput)>,
) {
    for (mut move_action, auto_input) in query.iter_mut() {
        move_action = auto_input.into();
    }
}

fn get_texture_atlas(asset_server: &Res<AssetServer>, sprite_sheet: &SpriteSheetConfig) -> TextureAtlas {
    let texture_handle = asset_server.load(sprite_sheet.path);
    TextureAtlas::from_grid(texture_handle, sprite_sheet.tile_size, sprite_sheet.columns, sprite_sheet.rows);
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_atlas = get_texture_atlas(&asset_server, &TIE_MAN_SPRITESHEET);
    let texture_atlas_handle =  texture_atlases.add(texture_atlas);
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(TIE_MAN_SPRITESHEET.scale_factor)),
            ..Default::default()
        })
        .insert(AutoInput {
            velocity: Vec2::new(20.0,0.0),
            ..Default::default()
        })
        .insert(MoveAction::default())
        .insert(TieManTag)
        .insert(TIE_MAN_DOWN.clone())
        .insert(Timer::from_seconds(0.1, true));

    commands
        .spawn_bundle(SpriteSheetBundle {
            texture_atlas: texture_atlas_handle.clone(),
            transform: Transform::from_scale(Vec3::splat(TIE_MAN_SPRITESHEET.scale_factor)),
            ..Default::default()
        })
        .insert(KeyboardInputBinding {
            speed: 100.0,
            ..Default::default()
        })
        .insert(MoveAction::default())
        .insert(TieManTag)
        .insert(TIE_MAN_DOWN.clone())
        .insert(Timer::from_seconds(0.1, true));
}
