use bevy::prelude::*;

use crate::core::components;
use crate::core::bundles;

pub mod components {
    #[derive(Default)]
    pub struct TieManTag;
}

pub mod bundles {
    use super::components::*;

    pub struct TieManBundle {
        tag: TieManTag,
        action: MoveAction,
        animation_effect: AnimationEffect,

        #[bundle]
        sprite_sheet: SpriteSheetBundle,

        #[bundle]
        animation: AnimationBundle
    }
}

pub mod systems {
    use super::components::*;
    use super::bundles::*;

    const SPRITE_SHEET: SpriteSheetConfig = SpriteSheetConfig {
        path: "tie_man_32x32.png",
        tile_size: (32.0,32.0),
        columns: 3,
        rows: 4,
        scale_factor: 3.0
    };

    const SPEED: f32 = 100.0;

    pub fn player_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    ) {
        let texture_atlas = get_texture_atlas(&asset_server, &SPRITE_SHEET);
        let texture_atlas_handle =  texture_atlases.add(texture_atlas);
        commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    
        commands
            .spawn_bundle(TieManBundle {
                texture_atlas: texture_atlas_handle.clone(),
                transform: Transform::from_scale(Vec3::splat(tie_man_spritesheet.scale_factor)),
                action: MoveAction::default(),
                ..Default::default()
            })
            .insert(KeyboardInputBinding {
                speed: SPEED,
                ..Default::default()
            });
    }

    /// Converts [MoveAction] into [AnimationEffect] for entities with [TieManTag]
    pub fn animation_control(
        mut query: Query<(&TieManTag, &MoveAction, &mut AnimationEffect)>
    ) {
        for (_tag, move_action, mut animation) in query.iter_mut() {
            *animation = match MoveAnimationSet::from(move_action) {
                MoveAnimationSet::RIGHT => AnimationEffect {
                    frames: (9..11).cycle(),
                    flip_x: false
                },
                MoveAnimationSet::LEFT => AnimationEffect {
                    frames: (9..11).cycle(),
                    flip_x: true
                },
                MoveAnimationSet::DOWN | MoveAnimationSet::STATIONARY => AnimationEffect {
                    frames: (6..9).cycle(),
                    flip_x: false
                },
                MoveAnimationSet::UP => AnimationEffect {
                    frames: (0..3).cycle(),
                    flip_x: false
                }
            }
        }
    }
}

pub mod plugins {
    use super::systems::*;

    pub struct PlayerPlugin;

    impl Plugin for PlayerPlugin {
        fn build(&self, app: &mut AppBuilder) {
            app.add_startup_system(player_setup.system())
                .add_system(animation.system())
                .add_system(movement.system())
        }
    }
}