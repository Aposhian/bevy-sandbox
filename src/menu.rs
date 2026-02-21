use bevy::app::AppExit;
use bevy::prelude::*;

use crate::game_state::GameState;
use crate::save::{LoadGameRequest, SaveDir, SaveGameRequest, SaveIndex};

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Paused), spawn_menu)
            .add_systems(OnExit(GameState::Paused), despawn_menu)
            .add_systems(
                Update,
                button_interactions.run_if(in_state(GameState::Paused)),
            )
            .add_systems(
                Update,
                menu_actions.run_if(in_state(GameState::Paused)),
            );
    }
}

#[derive(Component)]
struct MenuRoot;

#[derive(Component)]
enum MenuAction {
    Resume,
    ShowSave,
    ShowLoad,
    Exit,
    SaveToSlot(usize),
    LoadFromSlot(usize),
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
    spawn_button_under(commands, panel, "Save Game", MenuAction::ShowSave);
    spawn_button_under(commands, panel, "Load Game", MenuAction::ShowLoad);
    spawn_button_under(commands, panel, "Exit Game", MenuAction::Exit);
}

fn spawn_slot_panel_under(
    commands: &mut Commands,
    parent: Entity,
    is_save: bool,
    index: &SaveIndex,
) {
    let panel = commands
        .spawn((
            MenuPanel,
            panel_node(),
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    commands.entity(parent).add_child(panel);

    let title_text = if is_save { "Save Game" } else { "Load Game" };
    let title = commands
        .spawn((
            Text::new(title_text),
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

    for slot in 0..5 {
        let label = if let Some(info) = index.find_slot(slot) {
            let secs = info.timestamp_secs;
            format!("Slot {} - {}", slot + 1, format_timestamp(secs))
        } else {
            format!("Slot {} - Empty", slot + 1)
        };

        let action = if is_save {
            MenuAction::SaveToSlot(slot)
        } else {
            MenuAction::LoadFromSlot(slot)
        };

        spawn_button_under(commands, panel, &label, action);
    }

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
) {
    for (interaction, action) in interaction_query.iter() {
        if *interaction != Interaction::Pressed {
            continue;
        }

        match action {
            MenuAction::Resume => {
                next_state.set(GameState::Playing);
            }
            MenuAction::ShowSave => {
                rebuild_with_slots(&mut commands, &menu_root, &panels, true, &save_dir);
            }
            MenuAction::ShowLoad => {
                rebuild_with_slots(&mut commands, &menu_root, &panels, false, &save_dir);
            }
            MenuAction::Exit => {
                exit.write(AppExit::Success);
            }
            MenuAction::SaveToSlot(slot) => {
                save_requests.write(SaveGameRequest { slot: *slot });
                next_state.set(GameState::Playing);
            }
            MenuAction::LoadFromSlot(slot) => {
                load_requests.write(LoadGameRequest { slot: *slot });
            }
            MenuAction::Back => {
                rebuild_with_main(&mut commands, &menu_root, &panels);
            }
        }
    }
}

fn rebuild_with_slots(
    commands: &mut Commands,
    menu_root: &Query<Entity, With<MenuRoot>>,
    panels: &Query<Entity, With<MenuPanel>>,
    is_save: bool,
    save_dir: &SaveDir,
) {
    for entity in panels.iter() {
        commands.entity(entity).despawn();
    }

    let index = SaveIndex::load(&save_dir.0);

    if let Some(root) = menu_root.iter().next() {
        spawn_slot_panel_under(commands, root, is_save, &index);
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

fn despawn_menu(mut commands: Commands, menu_root: Query<Entity, With<MenuRoot>>) {
    for entity in menu_root.iter() {
        commands.entity(entity).despawn();
    }
}
