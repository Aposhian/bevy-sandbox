use std::sync::Arc;

use avian2d::prelude::*;
use bevy::prelude::*;
use tokio::sync::{mpsc, Mutex};
use tonic::{Request, Response, Status};

use crate::ball::{BallSpawnEvent, BallTag};
use crate::game_state::GameState;
use crate::health::Health;
use crate::input::{MoveAction, PlayerTag};
use crate::simple_figure::SimpleFigureTag;
use crate::PIXELS_PER_METER;

use super::proto::game_session_server::{GameSession, GameSessionServer};
use super::proto::{self};
use super::{
    GuestIdCounter, GuestInputEvent, GuestTag, HostChannels, HostTick,
    JoinEvent, JoinResponseData, LeaveEvent, NetworkRole,
};

pub struct HostPlugin;

impl Plugin for HostPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (host_tick_increment, host_broadcast)
                .chain()
                .run_if(is_host)
                .run_if(in_state(GameState::Playing)),
        )
        .add_systems(
            Update,
            (host_handle_joins, host_handle_leaves, host_receive_input)
                .run_if(is_host)
                .run_if(in_state(GameState::Playing)),
        );
    }
}

fn is_host(role: Res<NetworkRole>) -> bool {
    matches!(*role, NetworkRole::Host { .. })
}

/// Shared state for the gRPC service running in the background tokio runtime.
struct GameSessionService {
    join_tx: crossbeam_channel::Sender<JoinEvent>,
    input_tx: crossbeam_channel::Sender<GuestInputEvent>,
    leave_tx: crossbeam_channel::Sender<LeaveEvent>,
    /// Shared list of (guest_id, sender) for broadcasting world updates.
    update_senders: Arc<Mutex<Vec<(u32, mpsc::Sender<proto::WorldUpdate>)>>>,
}

#[tonic::async_trait]
impl GameSession for GameSessionService {
    async fn join(
        &self,
        request: Request<proto::JoinRequest>,
    ) -> Result<Response<proto::JoinResponse>, Status> {
        let req = request.into_inner();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();

        self.join_tx
            .send(JoinEvent {
                player_name: req.player_name,
                response_tx,
            })
            .map_err(|_| Status::internal("Host channel closed"))?;

        let response_data = response_rx
            .await
            .map_err(|_| Status::internal("Host failed to process join"))?;

        Ok(Response::new(proto::JoinResponse {
            guest_id: response_data.guest_id,
            snapshot: Some(response_data.snapshot),
        }))
    }

    async fn leave(
        &self,
        request: Request<proto::LeaveRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let req = request.into_inner();
        self.leave_tx
            .send(LeaveEvent {
                guest_id: req.guest_id,
            })
            .map_err(|_| Status::internal("Host channel closed"))?;

        // Remove the update sender for this guest
        let mut senders = self.update_senders.lock().await;
        senders.retain(|(id, _)| *id != req.guest_id);

        Ok(Response::new(proto::Empty {}))
    }

    async fn send_input(
        &self,
        request: Request<tonic::Streaming<proto::GuestInput>>,
    ) -> Result<Response<proto::Empty>, Status> {
        let mut stream = request.into_inner();

        while let Some(input) = stream
            .message()
            .await
            .map_err(|e| Status::internal(format!("Stream error: {e}")))?
        {
            let move_dir = input
                .move_direction
                .map(|v| Vec2::new(v.x, v.y))
                .unwrap_or_default();
            let shoot_dir = input.shoot_direction.map(|v| Vec2::new(v.x, v.y));

            let _ = self.input_tx.send(GuestInputEvent {
                guest_id: input.guest_id,
                move_direction: move_dir,
                shoot_direction: shoot_dir,
                client_tick: input.client_tick,
            });
        }

        Ok(Response::new(proto::Empty {}))
    }

    type StreamUpdatesStream =
        tokio_stream::wrappers::ReceiverStream<Result<proto::WorldUpdate, Status>>;

    async fn stream_updates(
        &self,
        request: Request<proto::StreamRequest>,
    ) -> Result<Response<Self::StreamUpdatesStream>, Status> {
        let guest_id = request.into_inner().guest_id;

        // Create a channel for this guest's updates
        let (raw_tx, mut raw_rx) = mpsc::channel::<proto::WorldUpdate>(64);
        let (result_tx, result_rx) = mpsc::channel::<Result<proto::WorldUpdate, Status>>(64);

        // Register this sender so the Bevy broadcast system can push updates
        {
            let mut senders = self.update_senders.lock().await;
            senders.push((guest_id, raw_tx));
        }

        // Bridge: convert WorldUpdate → Result<WorldUpdate, Status>
        tokio::spawn(async move {
            while let Some(update) = raw_rx.recv().await {
                if result_tx.send(Ok(update)).await.is_err() {
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(result_rx)))
    }

    async fn request_resync(
        &self,
        _request: Request<proto::StreamRequest>,
    ) -> Result<Response<proto::WorldSnapshot>, Status> {
        // For now, return an empty snapshot. The Bevy system will handle proper resync.
        // TODO: implement proper resync via channel to Bevy
        Err(Status::unimplemented("Resync not yet implemented"))
    }
}

/// Resource holding the Arc<Mutex<Vec>> of update senders, shared with the gRPC service.
#[derive(Resource, Clone)]
pub struct HostUpdateSenders(pub Arc<Mutex<Vec<(u32, mpsc::Sender<proto::WorldUpdate>)>>>);

/// Starts hosting: spawns the gRPC server and inserts necessary resources.
pub fn start_hosting(world: &mut World, port: u16) {
    let channels = HostChannels::default();
    let guest_id_counter = GuestIdCounter::default();

    let service = GameSessionService {
        join_tx: channels.join_tx.clone(),
        input_tx: channels.input_tx.clone(),
        leave_tx: channels.leave_tx.clone(),
        update_senders: Arc::new(Mutex::new(Vec::new())),
    };

    let update_senders = HostUpdateSenders(service.update_senders.clone());

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime for host");

        rt.block_on(async move {
            let addr = format!("0.0.0.0:{port}").parse().unwrap();
            info!("Host gRPC server listening on {addr}");

            tonic::transport::Server::builder()
                .add_service(GameSessionServer::new(service))
                .serve(addr)
                .await
                .expect("gRPC server failed");
        });
    });

    world.insert_resource(channels);
    world.insert_resource(guest_id_counter);
    world.insert_resource(update_senders);
    world.insert_resource(NetworkRole::Host { port });
}

fn host_tick_increment(mut tick: ResMut<HostTick>) {
    tick.0 += 1;
}

fn host_broadcast(
    tick: Res<HostTick>,
    update_senders: Option<Res<HostUpdateSenders>>,
    player_query: Query<
        (Entity, &Transform, &LinearVelocity, Option<&Health>),
        (With<SimpleFigureTag>, With<PlayerTag>, Without<GuestTag>),
    >,
    npc_query: Query<
        (Entity, &Transform, &LinearVelocity, &Health),
        (
            With<SimpleFigureTag>,
            Without<PlayerTag>,
            Without<GuestTag>,
        ),
    >,
    guest_query: Query<
        (Entity, &Transform, &LinearVelocity, &GuestTag, Option<&Health>),
        With<SimpleFigureTag>,
    >,
    ball_query: Query<(Entity, &Transform, &LinearVelocity, &Health), With<BallTag>>,
) {
    let Some(update_senders) = update_senders else {
        return;
    };

    let mut entities = Vec::new();

    // Host player
    for (entity, tf, vel, health) in player_query.iter() {
        entities.push(proto::EntityState {
            entity_id: entity.to_bits(),
            position: Some(proto::Vec2 {
                x: tf.translation.x,
                y: tf.translation.y,
            }),
            velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
            health_max: health.map(|h| h.max).unwrap_or(0),
            health_current: health.map(|h| h.current).unwrap_or(0),
            kind: proto::EntityKind::Player.into(),
        });
    }

    // NPCs
    for (entity, tf, vel, health) in npc_query.iter() {
        entities.push(proto::EntityState {
            entity_id: entity.to_bits(),
            position: Some(proto::Vec2 {
                x: tf.translation.x,
                y: tf.translation.y,
            }),
            velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
            health_max: health.max,
            health_current: health.current,
            kind: proto::EntityKind::Npc.into(),
        });
    }

    // Guest characters
    for (entity, tf, vel, guest_tag, health) in guest_query.iter() {
        entities.push(proto::EntityState {
            entity_id: entity.to_bits(),
            position: Some(proto::Vec2 {
                x: tf.translation.x,
                y: tf.translation.y,
            }),
            velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
            health_max: health.map(|h| h.max).unwrap_or(0),
            health_current: health.map(|h| h.current).unwrap_or(0),
            kind: proto::EntityKind::Guest.into(),
        });
        let _ = guest_tag; // used in query filter
    }

    // Balls
    for (entity, tf, vel, health) in ball_query.iter() {
        entities.push(proto::EntityState {
            entity_id: entity.to_bits(),
            position: Some(proto::Vec2 {
                x: tf.translation.x,
                y: tf.translation.y,
            }),
            velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
            health_max: health.max,
            health_current: health.current,
            kind: proto::EntityKind::Ball.into(),
        });
    }

    let timestamp_us = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;

    let update = proto::WorldUpdate {
        host_tick: tick.0,
        timestamp_us,
        entities,
        despawned: Vec::new(), // TODO: track despawned entities
    };

    // Send to all connected guests (non-blocking)
    // Since we can't block Bevy, use try_lock + try_send
    {
        let senders = update_senders.0.clone();
        if let Ok(guard) = senders.try_lock() {
            for (_guest_id, sender) in guard.iter() {
                let _ = sender.try_send(update.clone());
            }
        };
    }
}

fn host_handle_joins(
    mut commands: Commands,
    channels: Option<Res<HostChannels>>,
    guest_id_counter: ResMut<GuestIdCounter>,
    tick: Res<HostTick>,
    map_path: Res<crate::save::CurrentMapPath>,
    atlas_handle: Res<crate::simple_figure::SimpleFigureTextureAtlasHandle>,
    // Query all existing entities for the snapshot
    player_query: Query<
        (Entity, &Transform, &LinearVelocity, Option<&Health>),
        (With<SimpleFigureTag>, With<PlayerTag>),
    >,
    npc_query: Query<
        (Entity, &Transform, &LinearVelocity, &Health),
        (
            With<SimpleFigureTag>,
            Without<PlayerTag>,
            Without<GuestTag>,
        ),
    >,
    ball_query: Query<(Entity, &Transform, &LinearVelocity, &Health), With<BallTag>>,
    guest_figure_query: Query<
        (Entity, &Transform, &LinearVelocity, &GuestTag, Option<&Health>),
        With<SimpleFigureTag>,
    >,
) {
    let Some(channels) = channels else { return };

    while let Ok(join) = channels.join_rx.try_recv() {
        let guest_id = guest_id_counter.next();
        info!(
            "Guest '{}' joining with id {guest_id}",
            join.player_name
        );

        // Spawn a new SimpleFigure for the guest
        // Place near the host player or at origin
        let spawn_pos = player_query
            .iter()
            .next()
            .map(|(_, tf, _, _)| Vec2::new(tf.translation.x + 32.0, tf.translation.y))
            .unwrap_or(Vec2::ZERO);

        commands.spawn((
            SimpleFigureTag,
            GuestTag(guest_id),
            bevy::prelude::Sprite::from_atlas_image(
                atlas_handle.texture.clone(),
                bevy::prelude::TextureAtlas {
                    layout: atlas_handle.layout.clone(),
                    index: 0,
                },
            ),
            Transform::from_translation(Vec3::new(spawn_pos.x, spawn_pos.y, 2.0)),
            crate::simple_figure::AnimationIndices { first: 0, last: 2 },
            crate::simple_figure::AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
            RigidBody::Dynamic,
            Collider::capsule(0.18 * PIXELS_PER_METER, 0.6 * PIXELS_PER_METER),
            CollisionLayers::new(
                LayerMask::from([crate::simple_figure::GameLayer::Character]),
                LayerMask::from([
                    crate::simple_figure::GameLayer::Character,
                    crate::simple_figure::GameLayer::Wall,
                    crate::simple_figure::GameLayer::Ball,
                ]),
            ),
            CollisionEventsEnabled,
            LockedAxes::ROTATION_LOCKED,
            MoveAction::default(),
        ));

        // Build world snapshot
        let mut entities = Vec::new();

        for (entity, tf, vel, health) in player_query.iter() {
            entities.push(proto::EntityState {
                entity_id: entity.to_bits(),
                position: Some(proto::Vec2 {
                    x: tf.translation.x,
                    y: tf.translation.y,
                }),
                velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
                health_max: health.map(|h| h.max).unwrap_or(0),
                health_current: health.map(|h| h.current).unwrap_or(0),
                kind: proto::EntityKind::Player.into(),
            });
        }

        for (entity, tf, vel, health) in npc_query.iter() {
            entities.push(proto::EntityState {
                entity_id: entity.to_bits(),
                position: Some(proto::Vec2 {
                    x: tf.translation.x,
                    y: tf.translation.y,
                }),
                velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
                health_max: health.max,
                health_current: health.current,
                kind: proto::EntityKind::Npc.into(),
            });
        }

        for (entity, tf, vel, health) in ball_query.iter() {
            entities.push(proto::EntityState {
                entity_id: entity.to_bits(),
                position: Some(proto::Vec2 {
                    x: tf.translation.x,
                    y: tf.translation.y,
                }),
                velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
                health_max: health.max,
                health_current: health.current,
                kind: proto::EntityKind::Ball.into(),
            });
        }

        for (entity, tf, vel, guest_tag, health) in guest_figure_query.iter() {
            entities.push(proto::EntityState {
                entity_id: entity.to_bits(),
                position: Some(proto::Vec2 {
                    x: tf.translation.x,
                    y: tf.translation.y,
                }),
                velocity: Some(proto::Vec2 { x: vel.x, y: vel.y }),
                health_max: health.map(|h| h.max).unwrap_or(0),
                health_current: health.map(|h| h.current).unwrap_or(0),
                kind: proto::EntityKind::Guest.into(),
            });
            let _ = guest_tag;
        }

        // Add the newly spawned guest entity to the snapshot
        // (it won't be in queries yet since we just spawned it, so add manually)
        entities.push(proto::EntityState {
            entity_id: 0, // The guest will identify itself by guest_id, not entity_id
            position: Some(proto::Vec2 {
                x: spawn_pos.x,
                y: spawn_pos.y,
            }),
            velocity: Some(proto::Vec2 { x: 0.0, y: 0.0 }),
            health_max: 0,
            health_current: 0,
            kind: proto::EntityKind::Guest.into(),
        });

        let snapshot = proto::WorldSnapshot {
            host_tick: tick.0,
            map_path: map_path.0.clone(),
            entities,
        };

        let _ = join.response_tx.send(JoinResponseData {
            guest_id,
            snapshot,
        });
    }
}

fn host_handle_leaves(
    mut commands: Commands,
    channels: Option<Res<HostChannels>>,
    guest_query: Query<(Entity, &GuestTag)>,
) {
    let Some(channels) = channels else { return };

    while let Ok(leave) = channels.leave_rx.try_recv() {
        info!("Guest {} leaving", leave.guest_id);
        for (entity, tag) in guest_query.iter() {
            if tag.0 == leave.guest_id {
                commands.entity(entity).despawn();
            }
        }
    }
}

fn host_receive_input(
    channels: Option<Res<HostChannels>>,
    mut guest_query: Query<(&GuestTag, &mut MoveAction)>,
    mut ball_spawn: MessageWriter<BallSpawnEvent>,
    guest_transform_query: Query<(&GuestTag, &Transform)>,
) {
    let Some(channels) = channels else { return };

    while let Ok(input) = channels.input_rx.try_recv() {
        for (tag, mut move_action) in guest_query.iter_mut() {
            if tag.0 == input.guest_id {
                move_action.desired_velocity = input.move_direction;
            }
        }

        // Handle shooting
        if let Some(shoot_dir) = input.shoot_direction {
            for (tag, tf) in guest_transform_query.iter() {
                if tag.0 == input.guest_id {
                    let pos = Vec2::new(tf.translation.x, tf.translation.y);
                    let dir = shoot_dir.normalize_or_zero();
                    ball_spawn.write(BallSpawnEvent {
                        position: pos + dir * PIXELS_PER_METER,
                        velocity: dir * 10.0 * PIXELS_PER_METER,
                    });
                }
            }
        }
    }
}
