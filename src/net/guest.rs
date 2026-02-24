use std::collections::{HashMap, VecDeque};

use avian2d::prelude::*;
use bevy::prelude::*;

use super::proto::{self};
use super::{GuestChannels, HostAllPaused, LocalGuestId, NetworkRole};
use crate::ball::{BallTag, BallTextureHandle};
use crate::camera::CameraTarget;
use crate::game_state::GameState;
use crate::health::{DamageKindMask, Health};
use crate::input::{MoveAction, PlayerTag};
use crate::save::CurrentMapPath;
use crate::simple_figure::{
    AnimationIndices, AnimationTimer, GameLayer, SimpleFigureTag, SimpleFigureTextureAtlasHandle,
};
use crate::tiled::{SuppressObjectSpawn, TilemapSpawnEvent};
use crate::PIXELS_PER_METER;

pub struct GuestPlugin;

impl Plugin for GuestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            guest_send_input
                .run_if(is_guest)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            guest_apply_updates
                .run_if(is_guest)
                .run_if(not(in_state(GameState::MainMenu))),
        )
        .add_systems(
            Update,
            guest_interpolate
                .after(guest_apply_updates)
                .run_if(is_guest)
                .run_if(not(in_state(GameState::MainMenu))),
        )
        .add_systems(
            OnEnter(GameState::Paused),
            guest_send_pause_state.run_if(is_guest),
        )
        .add_systems(
            OnEnter(GameState::Playing),
            guest_send_pause_state.run_if(is_guest),
        )
        .add_systems(
            Update,
            guest_apply_pending_snapshot.run_if(resource_exists::<PendingSnapshot>),
        );
    }
}

/// Per-entity interpolation state for smooth rendering between server updates.
///
/// Uses a timeline buffer: server positions are placed on a timeline spaced
/// by `SERVER_TICK_DURATION`. A playback cursor advances with real time and
/// the rendered position is linearly interpolated between the two surrounding
/// timeline entries. If the buffer grows too large, old entries are discarded
/// to stay current.
#[derive(Component)]
pub struct NetInterpolation {
    /// Timeline of positions. Entry 0 is at time `base_time`.
    /// Each subsequent entry is `SERVER_TICK_DURATION` later.
    timeline: VecDeque<Vec3>,
    /// The time of `timeline[0]`.
    base_time: f32,
    /// Current playback cursor (absolute time).
    cursor: f32,
}

/// One server fixed-update tick (Bevy default: 64 Hz).
const SERVER_TICK_DURATION: f32 = 1.0 / 64.0;

impl NetInterpolation {
    fn new(pos: Vec3) -> Self {
        Self {
            timeline: VecDeque::from([pos]),
            base_time: 0.0,
            cursor: 0.0,
        }
    }

    /// Enqueue a new server position onto the timeline.
    fn push(&mut self, new_pos: Vec3) {
        let was_starved = self.timeline.len() < 2;
        self.timeline.push_back(new_pos);

        // On first real update (going from 1 to 2+ entries), reset cursor
        // so it interpolates across the first segment from the beginning.
        if was_starved && self.timeline.len() >= 2 {
            self.cursor = self.base_time;
        }
    }

    /// Advance cursor by `dt` and return the interpolated position.
    fn step(&mut self, dt: f32) -> Vec3 {
        // Cap advancement at one tick to prevent traversing multiple
        // segments in a single frame (which causes visible jumps).
        // The tick sync system adjusts Time<Virtual> to keep the guest's
        // update rate aligned with the host, so this cap doesn't cause drift.
        self.cursor += dt.min(SERVER_TICK_DURATION);

        // Compute position FIRST, then trim consumed segments.
        let pos = self.current_pos();

        // Trim fully consumed segments (cursor has moved past them).
        // Keep at least 2 entries so we always have a segment to interpolate.
        while self.timeline.len() > 2
            && self.cursor >= self.base_time + SERVER_TICK_DURATION
        {
            self.timeline.pop_front();
            self.base_time += SERVER_TICK_DURATION;
        }

        pos
    }

    /// Current interpolated position without advancing time.
    fn current_pos(&self) -> Vec3 {
        if self.timeline.len() < 2 {
            return *self.timeline.back().unwrap_or(&Vec3::ZERO);
        }

        // Find which segment the cursor is in and interpolate within it.
        let end_time =
            self.base_time + (self.timeline.len() - 1) as f32 * SERVER_TICK_DURATION;
        let clamped = self.cursor.clamp(self.base_time, end_time);

        let local = clamped - self.base_time;
        let seg = (local / SERVER_TICK_DURATION) as usize;
        let seg = seg.min(self.timeline.len() - 2);
        let t = (local - seg as f32 * SERVER_TICK_DURATION) / SERVER_TICK_DURATION;

        self.timeline[seg].lerp(self.timeline[seg + 1], t)
    }
}

fn is_guest(role: Res<NetworkRole>) -> bool {
    matches!(*role, NetworkRole::Guest { .. })
}

/// Maps host entity IDs to local ECS entities.
#[derive(Resource, Default)]
pub struct EntityMap(pub HashMap<u64, Entity>);

/// Pending snapshot to be applied by a Bevy system (needs MessageWriter access).
#[derive(Resource)]
struct PendingSnapshot {
    snapshot: proto::WorldSnapshot,
    guest_entity_id: u64,
}

/// Connect to the host, send JoinRequest, apply initial snapshot.
pub fn start_guest_connection(world: &mut World, addr: String) {
    info!("Connecting to host at {addr}...");

    let (update_tx, update_rx) = crossbeam_channel::unbounded();
    let (input_tx, input_rx) = tokio::sync::mpsc::channel::<proto::GuestInput>(64);

    // We need to connect and join synchronously enough to get the snapshot,
    // but we want the ongoing streaming to be async. Use a oneshot to get initial data.
    let (init_tx, init_rx) = std::sync::mpsc::channel();

    let addr_clone = addr.clone();
    let update_tx_clone = update_tx.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for guest");

        rt.block_on(async move {
            let endpoint = format!("http://{addr_clone}");
            let mut client =
                match proto::game_session_client::GameSessionClient::connect(endpoint).await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Failed to connect to host: {e}");
                        let _ = init_tx.send(Err(format!("Connection failed: {e}")));
                        return;
                    }
                };

            // Join
            let join_response = match client
                .join(proto::JoinRequest {
                    player_name: "Guest".to_string(),
                })
                .await
            {
                Ok(resp) => resp.into_inner(),
                Err(e) => {
                    error!("Join failed: {e}");
                    let _ = init_tx.send(Err(format!("Join failed: {e}")));
                    return;
                }
            };

            let guest_id = join_response.guest_id;
            let guest_entity_id = join_response.guest_entity_id;
            let snapshot = join_response.snapshot;

            let _ = init_tx.send(Ok((guest_id, guest_entity_id, snapshot)));

            // Start streaming updates from host
            let update_stream = client
                .stream_updates(proto::StreamRequest { guest_id })
                .await;

            // Start sending input to host — bridge the tokio mpsc receiver
            // (Bevy writes to input_tx, we forward from input_rx to gRPC)
            let input_stream = tokio_stream::wrappers::ReceiverStream::new(input_rx);
            let mut client_for_input = client.clone();
            tokio::spawn(async move {
                if let Err(e) = client_for_input.send_input(input_stream).await {
                    error!("SendInput stream error: {e}");
                }
            });

            // Read world updates and forward to Bevy
            match update_stream {
                Ok(response) => {
                    let mut stream = response.into_inner();
                    loop {
                        match stream.message().await {
                            Ok(Some(update)) => {
                                let _ = update_tx_clone.send(update);
                            }
                            Ok(None) => {
                                info!("Host stream ended");
                                break;
                            }
                            Err(e) => {
                                error!("Update stream error: {e}");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to start update stream: {e}");
                }
            }
        });
    });

    // Wait for initial join response (blocking, but only at connection time)
    match init_rx.recv() {
        Ok(Ok((guest_id, guest_entity_id, snapshot))) => {
            info!("Joined as guest {guest_id}, entity_id={guest_entity_id}");

            // Store the input sender in a way the guest input system can use
            // Don't store update_tx — only the background thread holds the sender.
            // When the host disconnects, the sender drops and update_rx sees Disconnected.
            drop(update_tx);
            let guest_channels = GuestChannels {
                update_rx,
                input_tx,
            };

            world.insert_resource(guest_channels);
            world.insert_resource(LocalGuestId {
                guest_id,
                entity_id: guest_entity_id,
            });
            world.insert_resource(EntityMap::default());
            world.insert_resource(NetworkRole::Guest { addr });

            // Queue snapshot for processing by a Bevy system
            // (needs MessageWriter<TilemapSpawnEvent> which isn't available from &mut World)
            if let Some(snapshot) = snapshot {
                world.insert_resource(PendingSnapshot {
                    snapshot,
                    guest_entity_id,
                });
            }
        }
        Ok(Err(e)) => {
            error!("Failed to join: {e}");
        }
        Err(_) => {
            error!("Connection thread died");
        }
    }
}

fn guest_apply_pending_snapshot(
    mut commands: Commands,
    pending: Res<PendingSnapshot>,
    mut map_path: ResMut<CurrentMapPath>,
    mut tilemap_spawn: MessageWriter<TilemapSpawnEvent>,
    mut entity_map: Option<ResMut<EntityMap>>,
    // Despawn existing gameplay entities before loading
    figures: Query<Entity, With<SimpleFigureTag>>,
    balls: Query<Entity, With<BallTag>>,
    atlas_handle: Res<SimpleFigureTextureAtlasHandle>,
    ball_texture: Res<BallTextureHandle>,
) {
    let snapshot = &pending.snapshot;
    let guest_entity_id = pending.guest_entity_id;

    info!(
        "Applying snapshot: {} entities, map: {}",
        snapshot.entities.len(),
        snapshot.map_path
    );

    // Despawn existing gameplay entities
    for entity in figures.iter() {
        commands.entity(entity).despawn();
    }
    for entity in balls.iter() {
        commands.entity(entity).despawn();
    }

    // Load the tilemap (objects_enabled: false — entities come from snapshot only)
    map_path.0 = snapshot.map_path.clone();
    commands.insert_resource(SuppressObjectSpawn);
    tilemap_spawn.write(TilemapSpawnEvent {
        path: snapshot.map_path.clone(),
        objects_enabled: false,
    });

    // Initialize entity map
    let mut map = HashMap::new();

    // Spawn entities from snapshot
    for entity_state in &snapshot.entities {
        let pos = entity_state
            .position
            .as_ref()
            .map(|p| Vec2::new(p.x, p.y))
            .unwrap_or_default();
        let vel = entity_state
            .velocity
            .as_ref()
            .map(|v| Vec2::new(v.x, v.y))
            .unwrap_or_default();

        let kind = proto::EntityKind::try_from(entity_state.kind).unwrap_or(proto::EntityKind::Npc);

        let local_entity = match kind {
            proto::EntityKind::Player
            | proto::EntityKind::Npc
            | proto::EntityKind::Guest
            | proto::EntityKind::Unspecified => {
                let spawn_pos = Vec3::new(pos.x, pos.y, 2.0);
                let mut ecmds = commands.spawn((
                    SimpleFigureTag,
                    Sprite::from_atlas_image(
                        atlas_handle.texture.clone(),
                        TextureAtlas {
                            layout: atlas_handle.layout.clone(),
                            index: 0,
                        },
                    ),
                    Transform::from_translation(spawn_pos),
                    AnimationIndices { first: 0, last: 2 },
                    AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
                    RigidBody::Kinematic,
                    Collider::capsule(0.18 * PIXELS_PER_METER, 0.6 * PIXELS_PER_METER),
                    CollisionLayers::new(
                        LayerMask::from([GameLayer::Character]),
                        LayerMask::from([GameLayer::Character, GameLayer::Wall, GameLayer::Ball]),
                    ),
                    LockedAxes::ROTATION_LOCKED,
                    LinearVelocity(vel),
                    NetInterpolation::new(spawn_pos),
                ));

                // This guest's own entity gets PlayerTag + CameraTarget
                if entity_state.entity_id == guest_entity_id {
                    ecmds.insert((PlayerTag, CameraTarget));
                }

                if entity_state.health_max > 0 {
                    ecmds.insert(Health {
                        max: entity_state.health_max,
                        current: entity_state.health_current,
                        vulnerable_to: DamageKindMask::NONE,
                    });
                }

                ecmds.id()
            }
            proto::EntityKind::Ball => {
                let spawn_pos = Vec3::new(pos.x, pos.y, 2.0);
                commands
                    .spawn((
                        BallTag,
                        Sprite::from_image(ball_texture.0.clone()),
                        Transform::from_translation(spawn_pos),
                        RigidBody::Kinematic,
                        Collider::circle(0.1 * PIXELS_PER_METER),
                        CollisionLayers::new(
                            LayerMask::from([GameLayer::Ball]),
                            LayerMask::from([
                                GameLayer::Character,
                                GameLayer::Ball,
                                GameLayer::Wall,
                            ]),
                        ),
                        LockedAxes::ROTATION_LOCKED,
                        LinearVelocity(vel),
                        NetInterpolation::new(spawn_pos),
                    ))
                    .id()
            }
        };

        map.insert(entity_state.entity_id, local_entity);
    }

    // Update entity map
    if let Some(ref mut entity_map) = entity_map {
        entity_map.0 = map;
    } else {
        commands.insert_resource(EntityMap(map));
    }

    // Remove the SuppressObjectSpawn after this frame
    commands.remove_resource::<SuppressObjectSpawn>();

    // Remove the pending snapshot resource
    commands.remove_resource::<PendingSnapshot>();
}

fn guest_send_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    guest_channels: Option<Res<GuestChannels>>,
    local_guest: Option<Res<LocalGuestId>>,
    state: Res<State<GameState>>,
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
    player_query: Query<&GlobalTransform, With<PlayerTag>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    let Some(channels) = guest_channels else {
        return;
    };
    let Some(local_guest) = local_guest else {
        return;
    };

    let mut desired_velocity = Vec2::ZERO;

    if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
        desired_velocity.y += 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
        desired_velocity.y -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
        desired_velocity.x -= 1.0;
    }
    if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
        desired_velocity.x += 1.0;
    }

    if desired_velocity.length_squared() != 0.0 {
        desired_velocity = desired_velocity.normalize();
    }

    // Shooting direction
    let shoot_direction = if buttons.just_pressed(MouseButton::Left) {
        // Calculate direction from player to mouse cursor
        let window = window_query.single();
        let camera = camera_query.single();
        let player_tf = player_query.iter().next();

        if let (Ok(window), Ok((camera, camera_tf)), Some(player_tf)) = (window, camera, player_tf)
        {
            window.cursor_position().and_then(|cursor_pos| {
                camera
                    .viewport_to_world_2d(camera_tf, cursor_pos)
                    .ok()
                    .map(|cursor_world_pos| {
                        let player_pos = player_tf.translation().xy();
                        (cursor_world_pos - player_pos).normalize_or_zero()
                    })
            })
        } else {
            None
        }
    } else {
        None
    };

    let input = proto::GuestInput {
        guest_id: local_guest.guest_id,
        move_direction: Some(proto::Vec2 {
            x: desired_velocity.x,
            y: desired_velocity.y,
        }),
        shoot_direction: shoot_direction.map(|d| proto::Vec2 { x: d.x, y: d.y }),
        client_tick: 0, // TODO: use local tick counter
        paused: matches!(state.get(), GameState::Paused),
    };

    let _ = channels.input_tx.try_send(input);
}

/// Sends a single input message with the current pause state on state transitions.
/// This ensures the host learns about pause/unpause even though `guest_send_input`
/// only runs in `GameState::Playing`.
fn guest_send_pause_state(
    guest_channels: Option<Res<GuestChannels>>,
    local_guest: Option<Res<LocalGuestId>>,
    state: Res<State<GameState>>,
) {
    let Some(channels) = guest_channels else {
        return;
    };
    let Some(local_guest) = local_guest else {
        return;
    };

    let input = proto::GuestInput {
        guest_id: local_guest.guest_id,
        move_direction: Some(proto::Vec2 { x: 0.0, y: 0.0 }),
        shoot_direction: None,
        client_tick: 0,
        paused: matches!(state.get(), GameState::Paused),
    };

    let _ = channels.input_tx.try_send(input);
}

fn guest_apply_updates(
    mut commands: Commands,
    guest_channels: Option<Res<GuestChannels>>,
    mut local_guest: Option<ResMut<LocalGuestId>>,
    mut entity_map: Option<ResMut<EntityMap>>,
    atlas_handle: Res<SimpleFigureTextureAtlasHandle>,
    ball_texture: Res<BallTextureHandle>,
    mut figure_query: Query<
        (&mut Transform, &mut LinearVelocity, &mut NetInterpolation),
        Or<(With<SimpleFigureTag>, With<BallTag>)>,
    >,
    mut sync_state: Option<ResMut<super::sync::TickSyncState>>,
    player_query: Query<Entity, With<PlayerTag>>,
    mut next_state: ResMut<NextState<GameState>>,
    figures: Query<Entity, With<SimpleFigureTag>>,
    balls: Query<Entity, With<BallTag>>,
    maps: Query<Entity, With<crate::tiled::TiledMapComponent>>,
    walls: Query<Entity, With<crate::tiled::WallTag>>,
    mut host_all_paused: ResMut<HostAllPaused>,
) {
    let Some(channels) = guest_channels else {
        return;
    };
    let Some(ref mut local_guest) = local_guest else {
        return;
    };
    let Some(ref mut entity_map) = entity_map else {
        return;
    };

    // Drain all pending updates into a vec. Each update is pushed into
    // per-entity interpolation timelines, keeping every position for smooth
    // interpolation. The timeline buffer handles overflow internally.
    let mut pending: Vec<proto::WorldUpdate> = Vec::new();
    while let Ok(update) = channels.update_rx.try_recv() {
        pending.push(update);
    }

    if pending.is_empty() {
        // Check for disconnect
        match channels.update_rx.try_recv() {
            Err(crossbeam_channel::TryRecvError::Empty) => return,
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
                warn!("Host disconnected, returning to main menu");
                commands.remove_resource::<GuestChannels>();
                commands.remove_resource::<LocalGuestId>();
                commands.remove_resource::<EntityMap>();
                commands.insert_resource(NetworkRole::Offline);
                for entity in figures.iter() {
                    commands.entity(entity).despawn();
                }
                for entity in balls.iter() {
                    commands.entity(entity).despawn();
                }
                for entity in maps.iter() {
                    commands.entity(entity).despawn();
                }
                for entity in walls.iter() {
                    commands.entity(entity).despawn();
                }
                next_state.set(GameState::MainMenu);
                return;
            }
            Ok(update) => {
                pending.push(update);
            }
        }
    }

    if pending.is_empty() {
        return;
    }

    // Use the latest update for metadata (pause state, tick sync, spawns).
    let latest = pending.last().unwrap();
    host_all_paused.0 = latest.all_paused;
    if let Some(ref mut sync) = sync_state {
        sync.last_host_tick = latest.host_tick;
    }

    // Process despawns from ALL updates so we never miss one
    for update in &pending {
        for despawned_id in &update.despawned {
            if let Some(local_entity) = entity_map.0.remove(despawned_id) {
                if let Ok(mut entity_commands) = commands.get_entity(local_entity) {
                    entity_commands.despawn();
                }
            }
        }
    }

    // Push every update's positions into per-entity interpolation timelines.
    // This gives the timeline buffer multiple entries to interpolate across
    // smoothly, rather than skipping to the latest and jerking.
    for update in &pending {
        for entity_state in &update.entities {
            let pos = entity_state
                .position
                .as_ref()
                .map(|p| Vec2::new(p.x, p.y))
                .unwrap_or_default();

            if let Some(&local_entity) = entity_map.0.get(&entity_state.entity_id) {
                if let Ok((tf, mut lv, mut interp)) = figure_query.get_mut(local_entity) {
                    let target = Vec3::new(pos.x, pos.y, tf.translation.z);
                    interp.push(target);

                    // Update velocity from the latest update only
                    if std::ptr::eq(update, pending.last().unwrap()) {
                        let vel = entity_state
                            .velocity
                            .as_ref()
                            .map(|v| Vec2::new(v.x, v.y))
                            .unwrap_or_default();
                        lv.0 = vel;
                    }
                }
            }
        }
    }

    // Spawn new entities from the latest update only
    let update = pending.last().unwrap();
    for entity_state in &update.entities {
        let pos = entity_state
            .position
            .as_ref()
            .map(|p| Vec2::new(p.x, p.y))
            .unwrap_or_default();
        let vel = entity_state
            .velocity
            .as_ref()
            .map(|v| Vec2::new(v.x, v.y))
            .unwrap_or_default();

        if entity_map.0.contains_key(&entity_state.entity_id) {
            continue; // Already handled in the per-update loop above
        }

        {
            // Spawn new entity
            let is_our_entity = entity_state.entity_id == local_guest.entity_id;

            // Remove PlayerTag from old entities before spawning (avoids borrow conflict)
            if is_our_entity {
                for old_player in player_query.iter() {
                    commands
                        .entity(old_player)
                        .remove::<(PlayerTag, CameraTarget)>();
                }
            }

            let kind =
                proto::EntityKind::try_from(entity_state.kind).unwrap_or(proto::EntityKind::Npc);

            let local_entity = match kind {
                proto::EntityKind::Player
                | proto::EntityKind::Npc
                | proto::EntityKind::Guest
                | proto::EntityKind::Unspecified => {
                    let spawn_pos = Vec3::new(pos.x, pos.y, 2.0);
                    let mut entity_commands = commands.spawn((
                        SimpleFigureTag,
                        Sprite::from_atlas_image(
                            atlas_handle.texture.clone(),
                            TextureAtlas {
                                layout: atlas_handle.layout.clone(),
                                index: 0,
                            },
                        ),
                        Transform::from_translation(spawn_pos),
                        AnimationIndices { first: 0, last: 2 },
                        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
                        RigidBody::Kinematic,
                        Collider::capsule(0.18 * PIXELS_PER_METER, 0.6 * PIXELS_PER_METER),
                        CollisionLayers::new(
                            LayerMask::from([GameLayer::Character]),
                            LayerMask::from([
                                GameLayer::Character,
                                GameLayer::Wall,
                                GameLayer::Ball,
                            ]),
                        ),
                        LockedAxes::ROTATION_LOCKED,
                        MoveAction::default(),
                        LinearVelocity(vel),
                        NetInterpolation::new(spawn_pos),
                    ));

                    if is_our_entity {
                        entity_commands.insert((PlayerTag, CameraTarget));
                    }

                    if entity_state.health_max > 0 {
                        entity_commands.insert(Health {
                            max: entity_state.health_max,
                            current: entity_state.health_current,
                            vulnerable_to: DamageKindMask::NONE,
                        });
                    }

                    entity_commands.id()
                }
                proto::EntityKind::Ball => {
                    let spawn_pos = Vec3::new(pos.x, pos.y, 2.0);
                    commands
                        .spawn((
                            BallTag,
                            Sprite::from_image(ball_texture.0.clone()),
                            Transform::from_translation(spawn_pos),
                            RigidBody::Kinematic,
                            Collider::circle(0.1 * PIXELS_PER_METER),
                            CollisionLayers::new(
                                LayerMask::from([GameLayer::Ball]),
                                LayerMask::from([
                                    GameLayer::Character,
                                    GameLayer::Ball,
                                    GameLayer::Wall,
                                ]),
                            ),
                            LockedAxes::ROTATION_LOCKED,
                            LinearVelocity(vel),
                            NetInterpolation::new(spawn_pos),
                        ))
                        .id()
                }
            };

            entity_map.0.insert(entity_state.entity_id, local_entity);
        }
    }
}

/// Smoothly interpolates entity positions between server snapshots each frame
/// by advancing the playback cursor through the buffered timeline.
fn guest_interpolate(
    time: Res<Time>,
    mut query: Query<
        (&mut Transform, &mut NetInterpolation),
        Or<(With<SimpleFigureTag>, With<BallTag>)>,
    >,
) {
    let dt = time.delta_secs();
    for (mut tf, mut interp) in query.iter_mut() {
        let pos = interp.step(dt);
        // Preserve the z coordinate (sprite layer)
        tf.translation = Vec3::new(pos.x, pos.y, tf.translation.z);
    }
}
