use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

pub mod guest;
pub mod host;
pub mod sync;

pub mod proto {
    tonic::include_proto!("network");
}

/// Network role for this game instance.
#[derive(Resource, Clone, Debug)]
pub enum NetworkRole {
    Offline,
    Host { port: u16 },
    Guest { addr: String },
}

impl Default for NetworkRole {
    fn default() -> Self {
        NetworkRole::Offline
    }
}

/// Marks an entity as controlled by a remote guest.
#[derive(Component)]
pub struct GuestTag(pub u32);

/// Tracks connected guests: guest_id → entity.
#[derive(Resource, Default)]
pub struct ConnectedGuests(pub HashMap<u32, Entity>);

/// Authoritative tick counter on the host, incremented each FixedUpdate.
#[derive(Resource, Default)]
pub struct HostTick(pub u64);

/// Next guest ID counter, shared with the async gRPC server.
#[derive(Resource, Clone)]
pub struct GuestIdCounter(pub Arc<AtomicU32>);

impl Default for GuestIdCounter {
    fn default() -> Self {
        GuestIdCounter(Arc::new(AtomicU32::new(1)))
    }
}

impl GuestIdCounter {
    pub fn next(&self) -> u32 {
        self.0.fetch_add(1, Ordering::Relaxed)
    }
}

// --- Channel types for Bevy ↔ async bridge ---

/// A join request from a guest wanting to connect.
pub struct JoinEvent {
    pub player_name: String,
    pub response_tx: tokio::sync::oneshot::Sender<JoinResponseData>,
}

/// Data sent back to the guest after join is processed by Bevy.
pub struct JoinResponseData {
    pub guest_id: u32,
    pub guest_entity_id: u64,
    pub snapshot: proto::WorldSnapshot,
}

/// Input received from a guest.
pub struct GuestInputEvent {
    pub guest_id: u32,
    pub move_direction: Vec2,
    pub shoot_direction: Option<Vec2>,
    pub client_tick: u64,
}

/// A leave notification from a guest.
pub struct LeaveEvent {
    pub guest_id: u32,
}

/// Channels from the gRPC server to Bevy (host side).
#[derive(Resource)]
pub struct HostChannels {
    pub join_rx: Receiver<JoinEvent>,
    pub join_tx: Sender<JoinEvent>,
    pub input_rx: Receiver<GuestInputEvent>,
    pub input_tx: Sender<GuestInputEvent>,
    pub leave_rx: Receiver<LeaveEvent>,
    pub leave_tx: Sender<LeaveEvent>,
}

impl Default for HostChannels {
    fn default() -> Self {
        let (join_tx, join_rx) = crossbeam_channel::unbounded();
        let (input_tx, input_rx) = crossbeam_channel::unbounded();
        let (leave_tx, leave_rx) = crossbeam_channel::unbounded();
        HostChannels {
            join_rx,
            join_tx,
            input_rx,
            input_tx,
            leave_rx,
            leave_tx,
        }
    }
}

/// Per-guest broadcast sender for world updates (host side).
/// The gRPC StreamUpdates handler holds the corresponding receiver.
#[derive(Resource, Default)]
pub struct GuestUpdateSenders {
    pub senders: Vec<(u32, tokio::sync::mpsc::Sender<proto::WorldUpdate>)>,
}

/// Channels from the gRPC client to Bevy (guest side).
#[derive(Resource)]
pub struct GuestChannels {
    pub update_rx: Receiver<proto::WorldUpdate>,
    pub update_tx: Sender<proto::WorldUpdate>,
    pub input_tx: tokio::sync::mpsc::Sender<proto::GuestInput>,
}

/// The guest's assigned ID and entity ID from the host.
#[derive(Resource)]
pub struct LocalGuestId {
    pub guest_id: u32,
    /// The host-side Entity bits for this guest's character.
    pub entity_id: u64,
}

pub struct NetworkPlugin;

impl Plugin for NetworkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NetworkRole>()
            .init_resource::<HostTick>()
            .init_resource::<GuestIdCounter>()
            .add_plugins(host::HostPlugin)
            .add_plugins(guest::GuestPlugin)
            .add_plugins(sync::SyncPlugin);
    }
}
