//! Headless testing infrastructure for the bevy-sandbox.
//!
//! Provides [`HeadlessPlugins`] (a window-less plugin set) and [`TestApp`]
//! (a convenience wrapper around [`App`]) so integration tests can exercise
//! game systems without a GPU or display server.

use bevy::app::{PluginGroupBuilder, SubApp};
use bevy::image::TextureAtlasPlugin;
use bevy::input::InputPlugin;
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::state::app::StatesPlugin;
use bevy::window::{ExitCondition, WindowPlugin};

use bevy::ecs::error::DefaultErrorHandler;

use crate::SandboxPlugins;
use crate::game_state::GameState;
use crate::net::{
    ConnectedGuests, GuestIdCounter, HostChannels, HostTick, NetworkRole, PauseVotes,
};

/// Minimal set of Bevy plugins that lets [`SandboxPlugins`] initialise without
/// opening a window or creating a renderer.
pub struct HeadlessPlugins;

/// Inserts an empty `RenderApp` sub-app so plugins like `bevy_ecs_tilemap`
/// that unconditionally call `app.sub_app_mut(RenderApp)` don't panic.
struct FakeRenderAppPlugin;

impl Plugin for FakeRenderAppPlugin {
    fn build(&self, app: &mut App) {
        app.insert_sub_app(RenderApp, SubApp::new());
    }
}

impl PluginGroup for HeadlessPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(bevy::app::TaskPoolPlugin::default())
            .add(bevy::time::TimePlugin)
            .add(bevy::asset::AssetPlugin::default())
            .add(ImagePlugin::default_nearest())
            .add(TextureAtlasPlugin)
            .add(StatesPlugin)
            .add(WindowPlugin {
                primary_window: None,
                exit_condition: ExitCondition::DontExit,
                ..default()
            })
            .add(InputPlugin)
            .add(FakeRenderAppPlugin)
    }
}

/// Test harness wrapping a headless [`App`] with convenience methods.
pub struct TestApp {
    pub app: App,
}

impl TestApp {
    /// Create a new headless app with [`HeadlessPlugins`] + [`SandboxPlugins`].
    /// The app starts in [`GameState::MainMenu`].
    pub fn new() -> Self {
        let mut app = App::new();
        // Use warn instead of panic for missing resources — some render-dependent
        // systems (e.g. bevy_ecs_tilemap) will fail validation headlessly, which
        // is expected and harmless.
        app.insert_resource(DefaultErrorHandler(bevy::ecs::error::warn));
        app.add_plugins(HeadlessPlugins);
        app.add_plugins(SandboxPlugins);
        // Run one update to let startup systems execute.
        app.update();
        TestApp { app }
    }

    /// Run a single frame.
    pub fn tick(&mut self) {
        self.app.update();
    }

    /// Run `n` frames.
    pub fn tick_n(&mut self, n: usize) {
        for _ in 0..n {
            self.app.update();
        }
    }

    /// Read the current [`GameState`].
    pub fn game_state(&self) -> GameState {
        *self.app.world().resource::<State<GameState>>().get()
    }

    /// Transition to [`GameState::Playing`] and spawn the default tilemap.
    pub fn start_game(&mut self) {
        self.app
            .world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        self.app
            .world_mut()
            .write_message(crate::tiled::TilemapSpawnEvent {
                path: "assets/example.tmx".to_string(),
                objects_enabled: true,
            });
        self.tick();
    }

    /// Transition to [`GameState::Playing`] without spawning a tilemap.
    pub fn start_game_no_map(&mut self) {
        self.app
            .world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);
        self.tick();
    }

    /// Simulate pressing a key by writing a [`KeyboardInput`] event.
    pub fn press_key(&mut self, key: KeyCode) {
        self.app
            .world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: key,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: bevy::input::ButtonState::Pressed,
                text: None,
                window: Entity::PLACEHOLDER,
                repeat: false,
            });
    }

    /// Simulate releasing a key by writing a [`KeyboardInput`] event.
    pub fn release_key(&mut self, key: KeyCode) {
        self.app
            .world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: key,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: bevy::input::ButtonState::Released,
                text: None,
                window: Entity::PLACEHOLDER,
                repeat: false,
            });
    }

    /// Simulate pressing a mouse button by writing a [`MouseButtonInput`] event.
    pub fn press_mouse(&mut self, button: MouseButton) {
        self.app
            .world_mut()
            .write_message(bevy::input::mouse::MouseButtonInput {
                button,
                state: bevy::input::ButtonState::Pressed,
                window: Entity::PLACEHOLDER,
            });
    }

    /// Simulate releasing a mouse button by writing a [`MouseButtonInput`] event.
    pub fn release_mouse(&mut self, button: MouseButton) {
        self.app
            .world_mut()
            .write_message(bevy::input::mouse::MouseButtonInput {
                button,
                state: bevy::input::ButtonState::Released,
                window: Entity::PLACEHOLDER,
            });
    }

    /// Count entities that have component `T`.
    pub fn count<T: Component>(&mut self) -> usize {
        self.app
            .world_mut()
            .query::<&T>()
            .iter(self.app.world())
            .count()
    }

    /// Check whether a resource of type `T` exists.
    pub fn has_resource<T: Resource>(&self) -> bool {
        self.app.world().get_resource::<T>().is_some()
    }

    /// Get a reference to a resource.
    pub fn resource<T: Resource>(&self) -> &T {
        self.app.world().resource::<T>()
    }

    /// Configure this app as a network host (without starting a real gRPC server).
    /// Inserts `NetworkRole::Host`, `HostChannels`, `ConnectedGuests`, etc.
    /// Returns the `HostChannels` default so tests can send events through the
    /// crossbeam senders.
    pub fn setup_host_mode(&mut self) -> HostChannels {
        let channels = HostChannels::default();
        let channels_clone = HostChannels {
            join_rx: channels.join_rx.clone(),
            join_tx: channels.join_tx.clone(),
            input_rx: channels.input_rx.clone(),
            input_tx: channels.input_tx.clone(),
            leave_rx: channels.leave_rx.clone(),
            leave_tx: channels.leave_tx.clone(),
        };
        self.app
            .world_mut()
            .insert_resource(NetworkRole::Host { port: 0 });
        self.app.world_mut().insert_resource(channels);
        self.app
            .world_mut()
            .insert_resource(ConnectedGuests::default());
        self.app
            .world_mut()
            .insert_resource(GuestIdCounter::default());
        self.app
            .world_mut()
            .insert_resource(PauseVotes::default());
        channels_clone
    }

    /// Read the current [`HostTick`] value.
    pub fn host_tick(&self) -> u64 {
        self.app.world().resource::<HostTick>().0
    }
}
