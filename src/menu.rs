use bevy::app::AppExit;
use bevy::prelude::*;

use crate::ball::BallTag;
use crate::game_state::GameState;
use crate::net::{ConnectedGuests, GuestTag, NetworkRole};
use crate::save::{LoadGameRequest, SaveDir, SaveGameRequest, SaveIndex, SaveTrigger};
use crate::simple_figure::SimpleFigureTag;
use crate::tiled::{TiledMapComponent, TilemapSpawnEvent, WallTag};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::MainMenu), spawn_main_menu)
            .add_systems(OnExit(GameState::MainMenu), despawn_menu)
            .add_systems(OnEnter(GameState::Paused), spawn_pause_menu)
            .add_systems(OnExit(GameState::Paused), despawn_menu)
            .add_systems(
                Update,
                (
                    button_interactions.run_if(in_menu),
                    menu_actions.run_if(in_menu),
                    join_input_system.run_if(in_menu),
                ),
            );
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
enum MenuAction {
    // Main menu actions
    StartGame,
    MainMenuShowJoin,
    MainMenuJoin,
    // Pause menu actions
    Resume,
    QuickSave,
    ShowLoad,
    HostGame,
    StopHosting,
    Disconnect,
    QuitToMainMenu,
    QuitToDesktop,
    LoadFile(String),
    Back,
}

#[derive(Component)]
struct MenuPanel;

fn in_menu(state: Res<State<GameState>>) -> bool {
    matches!(state.get(), GameState::Paused | GameState::MainMenu)
}

const NORMAL_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const HOVERED_BUTTON: Color = Color::srgb(0.35, 0.35, 0.35);
const PRESSED_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);

// ---------------------------------------------------------------------------
// Main Menu
// ---------------------------------------------------------------------------

fn spawn_main_menu(mut commands: Commands) {
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
            BackgroundColor(Color::srgb(0.05, 0.05, 0.1)),
            ZIndex(100),
        ))
        .id();

    spawn_main_menu_panel(&mut commands, root);
}

fn spawn_main_menu_panel(commands: &mut Commands, parent: Entity) {
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
            Text::new("BEVY SANDBOX"),
            TextFont {
                font_size: 40.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Node {
                margin: UiRect::bottom(Val::Px(20.0)),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(title);

    spawn_button_under(commands, panel, "Start Game", MenuAction::StartGame);
    spawn_button_under(commands, panel, "Load Game", MenuAction::ShowLoad);
    spawn_button_under(commands, panel, "Join Game", MenuAction::MainMenuShowJoin);
    spawn_button_under(commands, panel, "Quit to Desktop", MenuAction::QuitToDesktop);
}

// ---------------------------------------------------------------------------
// Pause Menu
// ---------------------------------------------------------------------------

fn spawn_pause_menu(
    mut commands: Commands,
    role: Res<NetworkRole>,
    connected_guests: Res<ConnectedGuests>,
) {
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

    spawn_pause_panel_under(&mut commands, root, &role, &connected_guests);
}

fn spawn_pause_panel_under(
    commands: &mut Commands,
    parent: Entity,
    role: &NetworkRole,
    connected_guests: &ConnectedGuests,
) {
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

    match role {
        NetworkRole::Guest { addr } => {
            // Guest view
            spawn_button_under(commands, panel, "Disconnect", MenuAction::Disconnect);

            // Connection info
            spawn_info_section(
                commands,
                panel,
                &format!("Connected to {addr}"),
                &[],
            );
        }
        NetworkRole::Host { port } => {
            // Host view: save/load + stop hosting
            spawn_button_under(commands, panel, "Save Game", MenuAction::QuickSave);
            spawn_button_under(commands, panel, "Load Game", MenuAction::ShowLoad);
            spawn_button_under(commands, panel, "Stop Hosting", MenuAction::StopHosting);

            // Connected guests info
            let guest_ids: Vec<String> = connected_guests
                .0
                .keys()
                .map(|id| format!("Guest {id}"))
                .collect();
            let guest_strs: Vec<&str> = guest_ids.iter().map(|s| s.as_str()).collect();
            spawn_info_section(
                commands,
                panel,
                &format!("Hosting on 0.0.0.0:{port}"),
                &guest_strs,
            );
        }
        NetworkRole::Offline => {
            // Offline: full menu
            spawn_button_under(commands, panel, "Save Game", MenuAction::QuickSave);
            spawn_button_under(commands, panel, "Load Game", MenuAction::ShowLoad);
            spawn_button_under(commands, panel, "Host Game", MenuAction::HostGame);
        }
    }

    spawn_button_under(commands, panel, "Quit to Main Menu", MenuAction::QuitToMainMenu);
    spawn_button_under(commands, panel, "Quit to Desktop", MenuAction::QuitToDesktop);
}

// ---------------------------------------------------------------------------
// Info section (connected guests panel)
// ---------------------------------------------------------------------------

fn spawn_info_section(commands: &mut Commands, parent: Entity, header: &str, items: &[&str]) {
    let section = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(8.0)),
                margin: UiRect::top(Val::Px(10.0)),
                row_gap: Val::Px(4.0),
                border_radius: BorderRadius::all(Val::Px(4.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
        ))
        .id();
    commands.entity(parent).add_child(section);

    let header_text = commands
        .spawn((
            Text::new(header),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextColor(Color::srgb(0.7, 0.9, 0.7)),
        ))
        .id();
    commands.entity(section).add_child(header_text);

    if items.is_empty() {
        let empty = commands
            .spawn((
                Text::new("No guests connected"),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                TextColor(Color::srgb(0.5, 0.5, 0.5)),
            ))
            .id();
        commands.entity(section).add_child(empty);
    } else {
        for item in items {
            let label = commands
                .spawn((
                    Text::new(item.to_string()),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.8, 0.8, 0.8)),
                ))
                .id();
            commands.entity(section).add_child(label);
        }
    }
}

// ---------------------------------------------------------------------------
// Sub-panels (load, join)
// ---------------------------------------------------------------------------

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

fn spawn_join_panel_under(commands: &mut Commands, parent: Entity, action: MenuAction) {
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

    spawn_button_under(commands, panel, "Connect", action);
    spawn_button_under(commands, panel, "Back", MenuAction::Back);
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

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

fn despawn_menu(mut commands: Commands, menu_root: Query<Entity, With<MenuRoot>>) {
    for entity in menu_root.iter() {
        commands.entity(entity).despawn();
    }
}

// ---------------------------------------------------------------------------
// Interaction + actions
// ---------------------------------------------------------------------------

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
    state: Res<State<GameState>>,
    mut exit: MessageWriter<AppExit>,
    mut save_requests: MessageWriter<SaveGameRequest>,
    mut load_requests: MessageWriter<LoadGameRequest>,
    mut tilemap_spawn: MessageWriter<TilemapSpawnEvent>,
    save_dir: Res<SaveDir>,
    role: Res<NetworkRole>,
    connected_guests: Res<ConnectedGuests>,
    menu_root: Query<Entity, With<MenuRoot>>,
    panels: Query<Entity, With<MenuPanel>>,
    join_input: Query<&Text, With<JoinAddrInput>>,
    gameplay_entities: Query<Entity, Or<(With<SimpleFigureTag>, With<BallTag>, With<TiledMapComponent>, With<WallTag>)>>,
    guest_entities: Query<Entity, With<GuestTag>>,
) {
    for (interaction, action) in interaction_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            // --- Main menu actions ---
            MenuAction::StartGame => {
                tilemap_spawn.write(TilemapSpawnEvent {
                    path: "assets/example.tmx".to_string(),
                    objects_enabled: true,
                });
                next_state.set(GameState::Playing);
            }
            MenuAction::MainMenuShowJoin => {
                rebuild_with_join(&mut commands, &menu_root, &panels, MenuAction::MainMenuJoin);
            }
            MenuAction::MainMenuJoin => {
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

            // --- Pause menu actions ---
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
            MenuAction::StopHosting => {
                // Despawn guest entities
                for entity in guest_entities.iter() {
                    commands.entity(entity).despawn();
                }
                // Stop hosting (clears resources, sets Offline)
                commands.queue(|world: &mut World| {
                    crate::net::host::stop_hosting(world);
                });
                // Rebuild pause menu to reflect new state
                rebuild_with_pause(
                    &mut commands,
                    &menu_root,
                    &panels,
                    &NetworkRole::Offline,
                    &connected_guests,
                );
            }
            MenuAction::Disconnect => {
                // Remove guest resources and despawn guest-created entities
                commands.remove_resource::<crate::net::GuestChannels>();
                commands.remove_resource::<crate::net::LocalGuestId>();
                commands.remove_resource::<crate::net::guest::EntityMap>();
                commands.insert_resource(NetworkRole::Offline);
                for entity in gameplay_entities.iter() {
                    commands.entity(entity).despawn();
                }
                info!("Disconnected from host");
                next_state.set(GameState::Playing);
            }
            MenuAction::QuitToMainMenu => {
                // Clean up networking
                match *role {
                    NetworkRole::Host { .. } => {
                        for entity in guest_entities.iter() {
                            commands.entity(entity).despawn();
                        }
                        commands.queue(|world: &mut World| {
                            crate::net::host::stop_hosting(world);
                        });
                    }
                    NetworkRole::Guest { .. } => {
                        commands.remove_resource::<crate::net::GuestChannels>();
                        commands.remove_resource::<crate::net::LocalGuestId>();
                        commands.remove_resource::<crate::net::guest::EntityMap>();
                        commands.insert_resource(NetworkRole::Offline);
                    }
                    NetworkRole::Offline => {}
                }
                // Despawn all gameplay entities
                for entity in gameplay_entities.iter() {
                    commands.entity(entity).despawn();
                }
                next_state.set(GameState::MainMenu);
            }
            MenuAction::QuitToDesktop => {
                exit.write(AppExit::Success);
            }
            MenuAction::LoadFile(filename) => {
                load_requests.write(LoadGameRequest {
                    filename: filename.clone(),
                });
                // If on main menu, transition to playing after load
                if *state.get() == GameState::MainMenu {
                    next_state.set(GameState::Playing);
                }
            }
            MenuAction::Back => {
                match state.get() {
                    GameState::MainMenu => {
                        rebuild_with_main_menu(&mut commands, &menu_root, &panels);
                    }
                    _ => {
                        rebuild_with_pause(
                            &mut commands,
                            &menu_root,
                            &panels,
                            &role,
                            &connected_guests,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Panel rebuilders
// ---------------------------------------------------------------------------

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
    connect_action: MenuAction,
) {
    for entity in panels.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(root) = menu_root.iter().next() {
        spawn_join_panel_under(commands, root, connect_action);
    }
}

fn rebuild_with_main_menu(
    commands: &mut Commands,
    menu_root: &Query<Entity, With<MenuRoot>>,
    panels: &Query<Entity, With<MenuPanel>>,
) {
    for entity in panels.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(root) = menu_root.iter().next() {
        spawn_main_menu_panel(commands, root);
    }
}

fn rebuild_with_pause(
    commands: &mut Commands,
    menu_root: &Query<Entity, With<MenuRoot>>,
    panels: &Query<Entity, With<MenuPanel>>,
    role: &NetworkRole,
    connected_guests: &ConnectedGuests,
) {
    for entity in panels.iter() {
        commands.entity(entity).despawn();
    }

    if let Some(root) = menu_root.iter().next() {
        spawn_pause_panel_under(commands, root, role, connected_guests);
    }
}

// ---------------------------------------------------------------------------
// Text input for join address
// ---------------------------------------------------------------------------

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
        KeyCode::Semicolon => Some(':'),
        _ => None,
    }
}
