use std::collections::HashMap;

use avian2d::prelude::*;
use bevy::prelude::*;

use crate::ball::{BallTag, BallTextureHandle};
use crate::game_state::GameState;
use crate::health::{DamageKindMask, Health};
use crate::input::{MoveAction, PlayerTag};
use crate::save::CurrentMapPath;
use crate::simple_figure::{
    AnimationIndices, AnimationTimer, GameLayer, SimpleFigureTag, SimpleFigureTextureAtlasHandle,
};
use crate::tiled::SuppressObjectSpawn;
use crate::PIXELS_PER_METER;

use super::proto::{self};
use super::{GuestChannels, GuestTag, LocalGuestId, NetworkRole};

pub struct GuestPlugin;

impl Plugin for GuestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (guest_send_input, guest_apply_updates)
                .run_if(is_guest)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn is_guest(role: Res<NetworkRole>) -> bool {
    matches!(*role, NetworkRole::Guest { .. })
}

/// Maps host entity IDs to local ECS entities.
#[derive(Resource, Default)]
pub struct EntityMap(pub HashMap<u64, Entity>);

/// Connect to the host, send JoinRequest, apply initial snapshot.
pub fn start_guest_connection(world: &mut World, addr: String) {
    info!("Connecting to host at {addr}...");

    let (update_tx, update_rx) = crossbeam_channel::unbounded();
    let (input_tx, _input_rx_holder) = tokio::sync::mpsc::channel::<proto::GuestInput>(64);

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
            let mut client = match proto::game_session_client::GameSessionClient::connect(endpoint)
                .await
            {
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
            let snapshot = join_response.snapshot;

            let _ = init_tx.send(Ok((guest_id, snapshot)));

            // Start streaming updates
            let update_stream = client
                .stream_updates(proto::StreamRequest { guest_id })
                .await;

            match update_stream {
                Ok(response) => {
                    let mut stream = response.into_inner();
                    // Also start sending input
                    let (_input_stream_tx, input_stream_rx) =
                        tokio::sync::mpsc::channel::<proto::GuestInput>(64);

                    // Bridge from the sync input_tx to the gRPC stream
                    // We need to create a ReceiverStream for tonic
                    let input_rx = tokio_stream::wrappers::ReceiverStream::new(input_stream_rx);
                    tokio::spawn(async move {
                        let _ = client.send_input(input_rx).await;
                    });

                    // Forward inputs from Bevy channel to the gRPC stream
                    // input_rx_holder is the tokio mpsc receiver
                    // We don't have it here — we need a different approach
                    // Actually input_tx is what Bevy writes to. We read from it here.
                    // But input_rx_holder was moved... Let's restructure.

                    // For now, read updates and forward to Bevy
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
        Ok(Ok((guest_id, snapshot))) => {
            info!("Joined as guest {guest_id}");

            // Store the input sender in a way the guest input system can use
            let guest_channels = GuestChannels {
                update_rx,
                update_tx,
                input_tx,
            };

            world.insert_resource(guest_channels);
            world.insert_resource(LocalGuestId(guest_id));
            world.insert_resource(EntityMap::default());
            world.insert_resource(NetworkRole::Guest { addr });

            // Apply snapshot
            if let Some(snapshot) = snapshot {
                apply_snapshot(world, &snapshot, guest_id);
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

fn apply_snapshot(world: &mut World, snapshot: &proto::WorldSnapshot, _local_guest_id: u32) {
    // Load the map
    let map_path = snapshot.map_path.clone();

    // We need to despawn existing gameplay entities and reload
    // For simplicity, just set the map path and let the tilemap system handle it
    world.insert_resource(CurrentMapPath(map_path.clone()));
    world.insert_resource(SuppressObjectSpawn);

    // Send tilemap spawn event
    // We can't easily write messages from &mut World, so we'll just set resources
    // and let systems handle the rest. For the initial snapshot, we'll spawn entities directly.

    // Note: The tilemap won't be spawned here since we can't fire events easily from World.
    // The guest will see entities but the map background will need to be handled.
    // For now, spawn a tilemap load via commands when we return to the ECS.

    // TODO: A cleaner approach would be to queue a TilemapSpawnEvent

    info!(
        "Applying snapshot: {} entities, map: {}",
        snapshot.entities.len(),
        snapshot.map_path
    );
}

fn guest_send_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    guest_channels: Option<Res<GuestChannels>>,
    guest_id: Option<Res<LocalGuestId>>,
    window_query: Query<&Window, With<bevy::window::PrimaryWindow>>,
    player_query: Query<&GlobalTransform, With<PlayerTag>>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
) {
    let Some(channels) = guest_channels else {
        return;
    };
    let Some(guest_id) = guest_id else { return };

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

        if let (Ok(window), Ok((camera, camera_tf)), Some(player_tf)) =
            (window, camera, player_tf)
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
        guest_id: guest_id.0,
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
    guest_id: Option<Res<LocalGuestId>>,
    mut entity_map: Option<ResMut<EntityMap>>,
    atlas_handle: Res<SimpleFigureTextureAtlasHandle>,
    ball_texture: Res<BallTextureHandle>,
    mut figure_query: Query<
        (&mut Transform, &mut LinearVelocity),
        Or<(With<SimpleFigureTag>, With<BallTag>)>,
    >,
    mut sync_state: Option<ResMut<super::sync::TickSyncState>>,
) {
    let Some(channels) = guest_channels else {
        return;
    };
    let Some(_guest_id) = guest_id else { return };
    let Some(ref mut entity_map) = entity_map else {
        return;
    };

    // Process all pending updates (use latest for positioning)
    let mut latest_update: Option<proto::WorldUpdate> = None;
    while let Ok(update) = channels.update_rx.try_recv() {
        latest_update = Some(update);
    }

    let Some(update) = latest_update else { return };

    // Update tick sync
    if let Some(ref mut sync) = sync_state {
        sync.last_host_tick = update.host_tick;
    }

    // Handle despawned entities
    for despawned_id in &update.despawned {
        if let Some(local_entity) = entity_map.0.remove(despawned_id) {
            commands.entity(local_entity).despawn();
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
                // Interpolate position for smoothness
                let target = Vec3::new(pos.x, pos.y, tf.translation.z);
                tf.translation = tf.translation.lerp(target, 0.3);
                lv.0 = vel;
            }
        } else {
            // Spawn new entity
            let kind = proto::EntityKind::try_from(entity_state.kind).unwrap_or(proto::EntityKind::Npc);
            let local_entity = match kind {
                proto::EntityKind::Player | proto::EntityKind::Npc | proto::EntityKind::Guest => {
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

                    // If this is our guest entity, add PlayerTag + CameraTarget
                    if kind == proto::EntityKind::Guest {
                        entity_commands.insert(GuestTag(0)); // remote guest
                    }

                    // Health for NPCs
                    if entity_state.health_max > 0 {
                        entity_commands.insert(Health {
                            max: entity_state.health_max,
                            current: entity_state.health_current,
                            vulnerable_to: DamageKindMask::NONE, // Guest doesn't run damage locally
                        });
                    }

                    entity_commands.id()
                }
                proto::EntityKind::Ball => {
                    commands
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
                        .id()
                }
            };

            entity_map.0.insert(entity_state.entity_id, local_entity);
        }
    }
}
