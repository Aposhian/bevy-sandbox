use bevy::prelude::*;
use std::f32::consts::PI;
use std::ops::Range;

pub mod setup {

    pub fn get_texture_atlas(asset_server: &Res<AssetServer>, sprite_sheet: &SpriteSheetConfig) -> TextureAtlas {
        let texture_handle = asset_server.load(sprite_sheet.path);
        TextureAtlas::from_grid(texture_handle, Vec2::from(sprite_sheet.tile_size), sprite_sheet.columns, sprite_sheet.rows)
    }
}

pub mod components {
    pub struct SpriteSheetConfig {
        path: &'static str,
        tile_size: (f32, f32),
        columns: usize,
        rows: usize,
        scale_factor: f32
    }

    /// Generic move command for all objects
    #[derive(Default)]
    pub struct MoveAction {
        velocity: Vec2
    }

    /// Animation cycle details
    #[derive(Clone)]
    pub struct AnimationEffect {
        frames: std::iter::Cycle<Range<u32>>,
        flip_x: bool
    }

    impl Default for AnimationEffect {
      fn default() -> Self {
          AnimationEffect {
              frames: (0..0).cycle(),
              flip_x: false
          }
      }
    }

    pub enum MoveAnimationSet {
        UP,
        DOWN,
        RIGHT,
        LEFT,
        STATIONARY
    }
    
    const RIGHT_QUADRANT_BOUNDS : Range<f32> = 0.0..PI/4.0;
    const VERTICAL_QUADRANT_BOUNDS : Range<f32> = RIGHT_QUADRANT_BOUNDS.end..3.0*PI/4.0;
    
    impl From<&MoveAction> for MoveAnimationSet {
        fn from(value: &MoveAction) -> Self {
            let angle = value.velocity.angle_between(Vec2::splat(0.0));
    
            match value.velocity.max_element() {
                0.0 | -0.0 => MoveAnimationSet::STATIONARY,
                _ => if RIGHT_QUADRANT_BOUNDS.contains(&angle) {
                        MoveAnimationSet::RIGHT
                    } else if VERTICAL_QUADRANT_BOUNDS.contains(&angle) {
                        if value.velocity.y > 0.0 {
                            MoveAnimationSet::UP
                        } else {
                            MoveAnimationSet::DOWN
                        }
                    } else {
                        MoveAnimationSet::LEFT
                    }
            }
        }
    }
}

pub mod bundles {
    use super::components::*;

    #[derive(Bundle)]
    pub struct AnimationBundle {
        animation: AnimationEffect;
        timer: Timer
    }

    impl Default for AnimationBundle {
        fn default() -> Self {
            AnimationBundle {
                timer: Timer::from_seconds(0.1, true),
                animation: AnimationEffect::default()
            }
        }
    }
}

pub mod systems {
    use super::components::*;

    /// Runs [TextureAtlasSprite] animations based on [AnimationEffect] component
    pub fn animation(
        time: Res<Time>,
        mut query: Query<(&mut Timer, &mut TextureAtlasSprite, &mut AnimationEffect)>,
    ) {
        for (mut timer, mut sprite, mut animation) in query.iter_mut() {
            timer.tick(time.delta());
            if timer.finished() {
                sprite.index = animation.frames.next().unwrap();
                sprite.flip_x = animation.flip_x;
            }
        }
    }

    /// Resolves [MoveAction] into [MoveEffect]
    pub fn collision(mut query: Query<(&MoveAction, &BoundingBox, &mut MoveEffect)>) {
        for (move_action, mut move_effect) in query.iter_mut() {
            
            transform.translation += time.delta_seconds() * move_action.velocity.extend(0.0);
        }
    }

    /// Moves sprite [Transform] based on [MoveEffect]
    pub fn movement(
        time: Res<Time>,
        mut query: Query<(&MoveEffect, &mut Transform)>,
    ) {
        for (move_action, mut transform) in query.iter_mut() {
            transform.translation += time.delta_seconds() * move_action.velocity.extend(0.0);
        }
    }
}

pub mod plugins {
    use super::systems::*;

    pub struct CorePlugin;

    impl Plugin for CorePlugin {
        fn build(&self, app: &mut AppBuilder) {
                .add_startup_system()
            app
                .add_system(animation.system())
                .add_system(movement.system())
        }
    }
}