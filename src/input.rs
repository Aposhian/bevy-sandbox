use bevy::prelude::*;

pub mod components {
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

    impl From<&AutoInput> for MoveAction {
        fn from(value: &AutoInput) -> Self {
            MoveAction {
                velocity: value.velocity
            }
        }
    }    
}

pub mod systems {
    fn keyboard(
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
    
    fn auto(
        mut query: Query<(&mut MoveAction, &AutoInput)>,
    ) {
        for (mut move_action, auto_input) in query.iter_mut() {
            *move_action = auto_input.into();
        }
    }
}