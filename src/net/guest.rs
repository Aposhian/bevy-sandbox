use std::collections::HashMap;

use avian2d::prelude::*;
use bevy::prelude::*;

use super::proto::{self};
use super::{GuestChannels, LocalGuestId, NetworkRole};
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
            (guest_send_input, guest_apply_updates)
                .run_if(is_guest)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            guest_apply_pending_snapshot.run_if(resource_exists::<PendingSnapshot>),
        );
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
                let mut ecmds = commands.spawn((
                    SimpleFigureTag,
                    Sprite::from_atlas_image(
                        atlas_handle.texture.clone(),
                        TextureAtlas {
                            layout: atlas_handle.layout.clone(),
                            index: 0,
                        },
                    ),
                    Transform::from_translation(Vec3::new(pos.x, pos.y, 2.0)),
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
            proto::EntityKind::Ball => commands
                .spawn((
                    BallTag,
                    Sprite::from_image(ball_texture.0.clone()),
                    Transform::from_translation(Vec3::new(pos.x, pos.y, 2.0)),
                    RigidBody::Kinematic,
                    Collider::circle(0.1 * PIXELS_PER_METER),
                    CollisionLayers::new(
                        LayerMask::from([GameLayer::Ball]),
                        LayerMask::from([GameLayer::Character, GameLayer::Ball, GameLayer::Wall]),
                    ),
                    LockedAxes::ROTATION_LOCKED,
                    LinearVelocity(vel),
                ))
                .id(),
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
        (&mut Transform, &mut LinearVelocity),
        Or<(With<SimpleFigureTag>, With<BallTag>)>,
    >,
    mut sync_state: Option<ResMut<super::sync::TickSyncState>>,
    player_query: Query<Entity, With<PlayerTag>>,
    mut next_state: ResMut<NextState<GameState>>,
    figures: Query<Entity, With<SimpleFigureTag>>,
    balls: Query<Entity, With<BallTag>>,
    maps: Query<Entity, With<crate::tiled::TiledMapComponent>>,
    walls: Query<Entity, With<crate::tiled::WallTag>>,
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

    // Drain all pending updates; accumulate despawns from every update
    // but use only the latest for entity positions.
    let mut all_despawned: Vec<u64> = Vec::new();
    let mut latest_update: Option<proto::WorldUpdate> = None;
    while let Ok(update) = channels.update_rx.try_recv() {
        all_despawned.extend_from_slice(&update.despawned);
        latest_update = Some(update);
    }

    // If no updates received, check if channel is disconnected (host dropped).
    // Since the guest doesn't hold the Sender, a disconnected receiver means
    // the background streaming thread (and thus the host connection) is gone.
    if latest_update.is_none() {
        // Peek with try_recv one more time to distinguish Empty from Disconnected
        match channels.update_rx.try_recv() {
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
            Err(crossbeam_channel::TryRecvError::Empty) => return,
            Ok(update) => {
                // Got a late update after all
                all_despawned.extend_from_slice(&update.despawned);
                latest_update = Some(update);
            }
        }
    }

    let Some(update) = latest_update else { return };

    // Update tick sync
    if let Some(ref mut sync) = sync_state {
        sync.last_host_tick = update.host_tick;
    }

    // Handle despawned entities (from ALL drained updates, not just the latest)
    for despawned_id in &all_despawned {
        if let Some(local_entity) = entity_map.0.remove(despawned_id) {
            if let Ok(mut entity_commands) = commands.get_entity(local_entity) {
                entity_commands.despawn();
            }
        }
    }

    // Update or spawn entities
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

        if let Some(&local_entity) = entity_map.0.get(&entity_state.entity_id) {
            // Update existing entity
            if let Ok((mut tf, mut lv)) = figure_query.get_mut(local_entity) {
                // Interpolate position for smoothness?
                let target = Vec3::new(pos.x, pos.y, tf.translation.z);
                tf.translation = target;
                lv.0 = vel;
            }
        } else {
            // Spawn new entity — no colliders or velocity on guest side
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
                    let mut entity_commands = commands.spawn((
                        SimpleFigureTag,
                        Sprite::from_atlas_image(
                            atlas_handle.texture.clone(),
                            TextureAtlas {
                                layout: atlas_handle.layout.clone(),
                                index: 0,
                            },
                        ),
                        Transform::from_translation(Vec3::new(pos.x, pos.y, 2.0)),
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
                proto::EntityKind::Ball => commands
                    .spawn((
                        BallTag,
                        Sprite::from_image(ball_texture.0.clone()),
                        Transform::from_translation(Vec3::new(pos.x, pos.y, 2.0)),
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
                    ))
                    .id(),
            };

            entity_map.0.insert(entity_state.entity_id, local_entity);
        }
    }
}
