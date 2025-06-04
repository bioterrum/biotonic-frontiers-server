//! One async task per live game.
//! ✔ resume after disconnect            (NEW: loads snapshot if found)
//! ✔ grace-period auto-forfeit
//! ✔ persistent per-turn snapshot in Redis

use crate::{
    config::settings,
    db::elo_repo,
    game::{
        logic, scoring,
        snapshot::Snapshot,
        types::{ResourcePool, TurnAction, Unit},
    },
    protocol::{ClientMsg, ServerMsg},
};
use dashmap::DashMap;
use once_cell::sync::Lazy;
use redis::{AsyncCommands, Client as RedisClient};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{
    sync::mpsc,
    task::JoinHandle,
    time::{sleep, Duration, Instant},
};
use uuid::Uuid;

/// In-memory map of active sessions: game_id → sender
static SESSIONS: Lazy<DashMap<Uuid, mpsc::Sender<ClientMsg>>> = Lazy::new(DashMap::new);

#[derive(Debug)]
pub enum DispatchErr {
    ChannelClosed,
}

pub async fn dispatch(db: PgPool, redis: RedisClient, msg: ClientMsg) -> Result<(), DispatchErr> {
    let game_id = match &msg {
        ClientMsg::Ready { game_id, .. }
        | ClientMsg::Turn { game_id, .. }
        | ClientMsg::Resume { game_id, .. }
        | ClientMsg::Disconnected { game_id, .. } => *game_id,
    };

    // Fast path - already running
    if let Some(tx) = SESSIONS.get(&game_id) {
        return tx.send(msg).await.map_err(|_| DispatchErr::ChannelClosed);
    }

    // Spawn new actor
    let (tx, mut rx) = mpsc::channel::<ClientMsg>(64);
    tx.send(msg).await.map_err(|_| DispatchErr::ChannelClosed)?;
    SESSIONS.insert(game_id, tx.clone());

    // --- shared handles ----------------------------------------------------
    let snap_key = format!("game:{game_id}:snap");
    let redis_client = Arc::new(redis.clone());
    let db_pool = db.clone();

    tokio::spawn(async move {
        //--------------------------------------------------------------------
        //   ❶  State initialisation  – possibly restored from Redis
        //--------------------------------------------------------------------
        let mut turn = 0_u32;
        let mut p1: Option<Uuid> = None;
        let mut p2: Option<Uuid> = None;
        let mut ready_p1 = false;
        let mut ready_p2 = false;
        let mut dc_since_p1 = None::<Instant>;
        let mut dc_since_p2 = None::<Instant>;

        let mut pool_p1 = ResourcePool {
            energy: 5,
            biomass: 5,
            gene_seeds: 2,
        };
        let mut pool_p2 = pool_p1.clone();
        let mut units_p1: Vec<Unit> = Vec::new();
        let mut units_p2: Vec<Unit> = Vec::new();

        let mut pending_p1 = None::<(u32, Vec<TurnAction>)>;
        let mut pending_p2 = None::<(u32, Vec<TurnAction>)>;
        let mut last_turn_result = None::<ServerMsg>;

        // ---- NEW: snapshot restore ---------------------------------------
        if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
            if let Ok(Some(json)) = conn.get::<_, Option<String>>(&snap_key).await {
                if let Ok(snap) = serde_json::from_str::<Snapshot>(&json) {
                    turn = snap.turn;
                    p1 = snap.p1;
                    p2 = snap.p2;
                    ready_p1 = snap.ready_p1;
                    ready_p2 = snap.ready_p2;
                    pool_p1 = snap.pool_p1;
                    pool_p2 = snap.pool_p2;
                    units_p1 = snap.units_p1;
                    units_p2 = snap.units_p2;
                    pending_p1 = snap.pending_p1;
                    pending_p2 = snap.pending_p2;
                    last_turn_result = snap.last_turn_result;
                    log::info!("Session {game_id} restored from snapshot (turn {turn})");
                }
            }
        }
        //--------------------------------------------------------------------

        // Mark row InProgress (idempotent)
        let _ = sqlx::query!(
            "UPDATE games SET state = 'InProgress' WHERE id = $1",
            game_id
        )
        .execute(&db_pool)
        .await;

        // helper: publish via Redis
        let redis_pub = redis_client.clone();
        let publish = move |pid: Uuid, msg: ServerMsg| -> JoinHandle<()> {
            let rc = redis_pub.clone();
            tokio::spawn(async move {
                if let Ok(mut c) = rc.get_multiplexed_async_connection().await {
                    let _: () = c
                        .publish(
                            format!("player:{pid}:events"),
                            serde_json::to_string(&msg).unwrap(),
                        )
                        .await
                        .unwrap_or(());
                }
            })
        };

        //--------------------------------------------------------------------
        //                         ❷  Main loop
        //--------------------------------------------------------------------
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    match msg {
                        // ------- Connect / Reconnect -----------------------
                        ClientMsg::Ready   { player_id, .. }
                        | ClientMsg::Resume{ player_id, .. } => {
                            if p1.is_none()                           { p1 = Some(player_id); }
                            else if p2.is_none() && p1 != Some(player_id) { p2 = Some(player_id); }

                            if Some(player_id) == p1 { ready_p1 = true; dc_since_p1 = None; }
                            if Some(player_id) == p2 { ready_p2 = true; dc_since_p2 = None; }

                            // On explicit Resume, replay the last turn so the UI is up-to-date
                            if matches!(msg, ClientMsg::Resume{..}) {
                                if let Some(tr) = &last_turn_result {
                                    publish(player_id, tr.clone()).await.ok();
                                }
                            }

                            // If both ready, (re)announce GameStart
                            if ready_p1 && ready_p2 {
                                let gs = ServerMsg::GameStart { game_id, turn };
                                publish(p1.unwrap(), gs.clone()).await.ok();
                                publish(p2.unwrap(), gs).await.ok();
                                if let Some(tr) = &last_turn_result {
                                    publish(p1.unwrap(), tr.clone()).await.ok();
                                    publish(p2.unwrap(), tr.clone()).await.ok();
                                }
                            }
                        }

                        // ------- Disconnect notice -------------------------
                        ClientMsg::Disconnected{ player_id, .. } => {
                            if Some(player_id) == p1 { ready_p1 = false; dc_since_p1 = Some(Instant::now()); }
                            if Some(player_id) == p2 { ready_p2 = false; dc_since_p2 = Some(Instant::now()); }
                        }

                        // ------- Player turn -------------------------------
                        ClientMsg::Turn{ player_id, turn: t, actions, .. } => {
                            if Some(player_id) == p1 { pending_p1 = Some((t, actions.clone())); }
                            if Some(player_id) == p2 { pending_p2 = Some((t, actions.clone())); }

                            if let (Some((ta,a1)), Some((tb,a2))) = (&pending_p1,&pending_p2) {
                                if ta == tb {
                                    let result = logic::resolve_turn(
                                        a1.clone(), a2.clone(),
                                        &mut pool_p1, &mut pool_p2,
                                        &mut units_p1,&mut units_p2,
                                    );
                                    let tr = ServerMsg::TurnResult{ game_id, turn:*ta, result };
                                    last_turn_result = Some(tr.clone());
                                    publish(p1.unwrap(), tr.clone()).await.ok();
                                    publish(p2.unwrap(), tr.clone()).await.ok();

                                    pending_p1 = None;
                                    pending_p2 = None;
                                    turn += 1;

                                    // save snapshot
                                    if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
                                        let snap = Snapshot {
                                            turn, p1, p2, ready_p1, ready_p2,
                                            pool_p1: pool_p1.clone(), pool_p2: pool_p2.clone(),
                                            units_p1: units_p1.clone(), units_p2: units_p2.clone(),
                                            pending_p1: pending_p1.clone(), pending_p2: pending_p2.clone(),
                                            last_turn_result: last_turn_result.clone(),
                                        };
                                        let _: () = conn
                                            .set_ex(&snap_key, serde_json::to_string(&snap).unwrap(), settings().disconnect_grace)
                                            .await
                                            .unwrap_or(());
                                    }

                                    if turn >= settings().max_turns {
                                        finish_game(&db_pool,&publish,game_id,&units_p1,&units_p2,p1,p2,&snap_key).await;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }

                // ------- Grace-period watch -------------------------------
                _ = sleep(Duration::from_secs(5)) => {
                    let grace = Duration::from_secs(settings().disconnect_grace);
                    if dc_since_p1.zip(p2).filter(|(t,_)| t.elapsed() >= grace).is_some() {
                        finish_forfeit(&db_pool,&publish,game_id,p2.unwrap(),p1.unwrap(),&snap_key).await;
                        break;
                    }
                    if dc_since_p2.zip(p1).filter(|(t,_)| t.elapsed() >= grace).is_some() {
                        finish_forfeit(&db_pool,&publish,game_id,p1.unwrap(),p2.unwrap(),&snap_key).await;
                        break;
                    }
                }
            }
        }

        // final cleanup
        SESSIONS.remove(&game_id);
    });

    Ok(())
}

/// Decide winner: returns Some(player_id) if exactly one side has units remaining.
fn decide_winner(u1: &[Unit], u2: &[Unit], p1: Option<Uuid>, p2: Option<Uuid>) -> Option<Uuid> {
    match (u1.is_empty(), u2.is_empty()) {
        (false, true) => p1,
        (true, false) => p2,
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
async fn finish_game(
    db: &PgPool,
    publish: &impl Fn(Uuid, ServerMsg) -> JoinHandle<()>,
    gid: Uuid,
    u1: &[Unit],
    u2: &[Unit],
    p1: Option<Uuid>,
    p2: Option<Uuid>,
    snap_key: &str,
) {
    let winner = decide_winner(u1, u2, p1, p2);
    apply_elo_and_persist(db, gid, winner, p1, p2).await;
    if let (Some(a), Some(b)) = (p1, p2) {
        publish(
            a,
            ServerMsg::GameOver {
                game_id: gid,
                winner,
            },
        )
        .await
        .ok();
        publish(
            b,
            ServerMsg::GameOver {
                game_id: gid,
                winner,
            },
        )
        .await
        .ok();
    }
    let _: () = redis_cleanup(db, snap_key).await;
}

async fn finish_forfeit(
    db: &PgPool,
    publish: &impl Fn(Uuid, ServerMsg) -> JoinHandle<()>,
    gid: Uuid,
    winner: Uuid,
    loser: Uuid,
    snap_key: &str,
) {
    apply_elo_and_persist(db, gid, Some(winner), Some(winner), Some(loser)).await;
    publish(
        winner,
        ServerMsg::GameOver {
            game_id: gid,
            winner: Some(winner),
        },
    )
    .await
    .ok();
    publish(
        loser,
        ServerMsg::GameOver {
            game_id: gid,
            winner: Some(winner),
        },
    )
    .await
    .ok();
    let _: () = redis_cleanup(db, snap_key).await;
}

async fn redis_cleanup(_db: &PgPool, key: &str) {
    if let Ok(redis_url) = std::env::var("REDIS_URL") {
        if let Ok(client) = RedisClient::open(redis_url.as_str()) {
            if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
                let _: () = conn.del(key).await.unwrap_or(());
            }
        }
    }
}

async fn apply_elo_and_persist(
    db: &PgPool,
    gid: Uuid,
    winner: Option<Uuid>,
    p1_opt: Option<Uuid>,
    p2_opt: Option<Uuid>,
) {
    if let (Some(p1), Some(p2)) = (p1_opt, p2_opt) {
        let (r1, r2) = sqlx::query_as::<_, (i32, i32)>(
            "SELECT p1.elo_rating, p2.elo_rating FROM players p1, players p2 WHERE p1.id=$1 AND p2.id=$2"
        )
        .bind(p1)
        .bind(p2)
        .fetch_one(db)
        .await
        .unwrap();

        let flag = match winner {
            Some(id) if id == p1 => 1,
            Some(id) if id == p2 => 2,
            _ => 0,
        };
        let (d1, d2) = scoring::elo_delta(r1, r2, flag, 32.0);
        let _ = elo_repo::apply_delta(db, p1, d1).await;
        let _ = elo_repo::apply_delta(db, p2, d2).await;

        let _ = sqlx::query!(
            "UPDATE games SET state='Finished', winner_id=$1, player1_elo_delta=$2, player2_elo_delta=$3 WHERE id=$4",
            winner, d1, d2, gid
        )
        .execute(db)
        .await;
    }
}
