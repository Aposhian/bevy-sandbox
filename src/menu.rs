use bevy::app::AppExit;
use bevy::prelude::*;

use crate::game_state::GameState;
use crate::save::{LoadGameRequest, SaveDir, SaveGameRequest, SaveIndex, SaveTrigger};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Paused), spawn_menu)
            .add_systems(OnExit(GameState::Paused), despawn_menu)
            .add_systems(
                Update,
                (button_interactions, menu_actions, join_input_system)
                    .run_if(in_state(GameState::Paused)),
            );
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
enum MenuAction {
    Resume,
    QuickSave,
    ShowLoad,
    HostGame,
    ShowJoin,
    JoinGame,
    Exit,
    LoadFile(String),
    Back,
}

#[derive(Component)]
struct MenuPanel;

const NORMAL_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const HOVERED_BUTTON: Color = Color::srgb(0.35, 0.35, 0.35);
const PRESSED_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);

fn spawn_menu(mut commands: Commands) {
    let root = commands
        .spawn((
            MenuRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            ZIndex(100),
        ))
        .id();

    spawn_main_panel_under(&mut commands, root);
}

fn panel_node() -> Node {
    Node {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        padding: UiRect::all(Val::Px(20.0)),
        row_gap: Val::Px(10.0),
        border_radius: BorderRadius::all(Val::Px(8.0)),
        ..default()
    }
}

fn spawn_main_panel_under(commands: &mut Commands, parent: Entity) {
    let panel = commands
        .spawn((
            MenuPanel,
            panel_node(),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    commands.entity(parent).add_child(panel);

    // Title
    let title = commands
        .spawn((
            Text::new("PAUSED"),
            TextFont {
                font_size: 32.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(title);

    spawn_button_under(commands, panel, "Resume", MenuAction::Resume);
    spawn_button_under(commands, panel, "Save Game", MenuAction::QuickSave);
    spawn_button_under(commands, panel, "Load Game", MenuAction::ShowLoad);
    spawn_button_under(commands, panel, "Host Game", MenuAction::HostGame);
    spawn_button_under(commands, panel, "Join Game", MenuAction::ShowJoin);
    spawn_button_under(commands, panel, "Exit Game", MenuAction::Exit);
}

fn spawn_load_panel_under(commands: &mut Commands, parent: Entity, index: &SaveIndex) {
    let panel = commands
        .spawn((
            MenuPanel,
            panel_node(),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    commands.entity(parent).add_child(panel);

    let title = commands
        .spawn((
            Text::new("Load Game"),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(title);

    if index.slots.is_empty() {
        let empty = commands
            .spawn((
                Text::new("No saves found"),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                Node {
                    margin: UiRect::bottom(Val::Px(10.0)),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel).add_child(empty);
    } else {
        // Slots are already sorted newest-first by SaveIndex
        for info in &index.slots {
            let trigger = SaveTrigger::from_proto(info.trigger);
            let label = format!(
                "{} - {}",
                trigger.label(),
                format_timestamp(info.timestamp_secs)
            );
            spawn_button_under(
                commands,
                panel,
                &label,
                MenuAction::LoadFile(info.filename.clone()),
            );
        }
    }

    spawn_button_under(commands, panel, "Back", MenuAction::Back);
}

/// Marker for the text input field in the join panel.
#[derive(Component)]
struct JoinAddrInput;

fn spawn_join_panel_under(commands: &mut Commands, parent: Entity) {
    let panel = commands
        .spawn((
            MenuPanel,
            panel_node(),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    commands.entity(parent).add_child(panel);

    let title = commands
        .spawn((
            Text::new("Join Game"),
            TextFont {
                font_size: 28.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(10.0)),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(title);

    let hint = commands
        .spawn((
            Text::new("Enter host address (e.g. 127.0.0.1:5555)"),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.7, 0.7)),
            Node {
                margin: UiRect::bottom(Val::Px(5.0)),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(hint);

    // Text input field (editable via keyboard in join_input_system)
    let input_bg = commands
        .spawn((
            Node {
                width: Val::Px(250.0),
                height: Val::Px(35.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
        ))
        .id();
    commands.entity(panel).add_child(input_bg);

    let input_text = commands
        .spawn((
            JoinAddrInput,
            Text::new("127.0.0.1:5555".to_string()),
            TextFont {
                font_size: 16.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(input_bg).add_child(input_text);

    spawn_button_under(
        commands,
        panel,
        "Connect",
        MenuAction::JoinGame, // actual addr read from input at press time
    );
    spawn_button_under(commands, panel, "Back", MenuAction::Back);
}

fn format_timestamp(secs: u64) -> String {
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    let days = secs / 86400;
    format!("Day {days} {hours:02}:{minutes:02}:{seconds:02}")
}

fn spawn_button_under(commands: &mut Commands, parent: Entity, text: &str, action: MenuAction) {
    let btn = commands
        .spawn((
            action,
            Button,
            Node {
                width: Val::Px(250.0),
                height: Val::Px(45.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(NORMAL_BUTTON),
        ))
        .id();
    commands.entity(parent).add_child(btn);

    let label = commands
        .spawn((
            Text::new(text.to_string()),
            TextFont {
                font_size: 18.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ))
        .id();
    commands.entity(btn).add_child(label);
}

fn button_interactions(
    mut query: Query<(&Interaction, &mut BackgroundColor), (Changed<Interaction>, With<Button>)>,
) {
    for (interaction, mut bg) in query.iter_mut() {
        *bg = match *interaction {
            Interaction::Pressed => PRESSED_BUTTON.into(),
            Interaction::Hovered => HOVERED_BUTTON.into(),
            Interaction::None => NORMAL_BUTTON.into(),
        };
    }
}

fn menu_actions(
    mut commands: Commands,
    interaction_query: Query<(&Interaction, &MenuAction), (Changed<Interaction>, With<Button>)>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
    mut save_requests: MessageWriter<SaveGameRequest>,
    mut load_requests: MessageWriter<LoadGameRequest>,
    save_dir: Res<SaveDir>,
    menu_root: Query<Entity, With<MenuRoot>>,
    panels: Query<Entity, With<MenuPanel>>,
    join_input: Query<&Text, With<JoinAddrInput>>,
) {
    for (interaction, action) in interaction_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            MenuAction::Resume => {
                next_state.set(GameState::Playing);
            }
            MenuAction::QuickSave => {
                save_requests.write(SaveGameRequest {
                    trigger: SaveTrigger::User,
                });
                next_state.set(GameState::Playing);
            }
            MenuAction::ShowLoad => {
                rebuild_with_load(&mut commands, &menu_root, &panels, &save_dir);
            }
            MenuAction::HostGame => {
                commands.queue(|world: &mut World| {
                    crate::net::host::start_hosting(world, 5555);
                });
                info!("Hosting game on port 5555");
                next_state.set(GameState::Playing);
            }
            MenuAction::ShowJoin => {
                rebuild_with_join(&mut commands, &menu_root, &panels);
            }
            MenuAction::JoinGame => {
                // Read the actual address from the text input
                let addr = join_input
                    .iter()
                    .next()
                    .map(|t| t.0.clone())
                    .unwrap_or_else(|| "127.0.0.1:5555".to_string());
                let addr_clone = addr.clone();
                commands.queue(move |world: &mut World| {
                    crate::net::guest::start_guest_connection(world, addr_clone);
                });
                info!("Joining game at {addr}");
                next_state.set(GameState::Playing);
            }
            MenuAction::Exit => {
                exit.write(AppExit::Success);
            }
            MenuAction::LoadFile(filename) => {
                load_requests.write(LoadGameRequest {
                    filename: filename.clone(),
                });
            }
            MenuAction::Back => {
                rebuild_with_main(&mut commands, &menu_root, &panels);
            }
        }
    }
}

fn rebuild_with_load(
    commands: &mut Commands,
    menu_root: &Query<Entity, With<MenuRoot>>,
    panels: &Query<Entity, With<MenuPanel>>,
    save_dir: &SaveDir,
) {
    for entity in panels.iter() {
        commands.entity(entity).despawn();
    }

    let index = SaveIndex::load(&save_dir.0);

    if let Some(root) = menu_root.iter().next() {
        spawn_load_panel_under(commands, root, &index);
    }
}

fn rebuild_with_join(
    commands: &mut Commands,
    menu_root: &Query<Entity, With<MenuRoot>>,
    panels: &Query<Entity, With<MenuPanel>>,
) {
    for entity in panels.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(root) = menu_root.iter().next() {
        spawn_join_panel_under(commands, root);
    }
}

fn rebuild_with_main(
    commands: &mut Commands,
    menu_root: &Query<Entity, With<MenuRoot>>,
    panels: &Query<Entity, With<MenuPanel>>,
) {
    for entity in panels.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(root) = menu_root.iter().next() {
        spawn_main_panel_under(commands, root);
    }
}

fn join_input_system(
    mut char_events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Text, With<JoinAddrInput>>,
) {
    let Ok(mut text) = query.single_mut() else {
        return;
    };

    for event in char_events.read() {
        if event.state != bevy::input::ButtonState::Pressed {
            continue;
        }
        match event.key_code {
            KeyCode::Backspace => {
                text.0.pop();
            }
            _ => {
                // Map key codes to characters for address input
                let ch = key_to_char(event.key_code, keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight));
                if let Some(c) = ch {
                    text.0.push(c);
                }
            }
        }
    }
}

fn key_to_char(key: KeyCode, _shift: bool) -> Option<char> {
    match key {
        KeyCode::Digit0 | KeyCode::Numpad0 => Some('0'),
        KeyCode::Digit1 | KeyCode::Numpad1 => Some('1'),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some('2'),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some('3'),
        KeyCode::Digit4 | KeyCode::Numpad4 => Some('4'),
        KeyCode::Digit5 | KeyCode::Numpad5 => Some('5'),
        KeyCode::Digit6 | KeyCode::Numpad6 => Some('6'),
        KeyCode::Digit7 | KeyCode::Numpad7 => Some('7'),
        KeyCode::Digit8 | KeyCode::Numpad8 => Some('8'),
        KeyCode::Digit9 | KeyCode::Numpad9 => Some('9'),
        KeyCode::Period | KeyCode::NumpadDecimal => Some('.'),
        KeyCode::Semicolon => Some(':'), // Shift+; = : on US layout
        _ => None,
    }
}

fn despawn_menu(mut commands: Commands, menu_root: Query<Entity, With<MenuRoot>>) {
    for entity in menu_root.iter() {
        commands.entity(entity).despawn();
    }
}
