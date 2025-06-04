//! WebSocket endpoint with Redis event subscription.

use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::{handle, Message};
use futures::StreamExt;
use redis::{AsyncCommands, Client as RedisClient};
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::settings;
use crate::game::session::dispatch;
use crate::protocol::ClientMsg;

pub async fn ws_index(
    req: HttpRequest,
    body: web::Payload,
    db_pool: web::Data<PgPool>,
    redis: web::Data<RedisClient>,
) -> Result<HttpResponse, Error> {
    // 1 · player_id query param
    let pid_str = req
        .query_string()
        .split('&')
        .find_map(|kv| kv.strip_prefix("player_id="))
        .ok_or_else(|| actix_web::error::ErrorBadRequest("player_id missing"))?;
    let player_id =
        Uuid::parse_str(pid_str).map_err(|_| actix_web::error::ErrorBadRequest("bad UUID"))?;

    // 2 · handshake
    let (response, mut session, mut ws_stream) = handle(&req, body)?;

    // 3 · presence key
    {
        let mut conn = redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("redis"))?;
        let key = format!("session:{player_id}");
        let _: () = conn
            .set_ex(&key, "1", settings().presence_ttl)
            .await
            .unwrap_or(());
    }

    // 4 · find player’s faction (if any)
    let faction_id: Option<Uuid> = sqlx::query_scalar!(
        "SELECT faction_id FROM faction_members WHERE player_id = $1",
        player_id
    )
    .fetch_optional(db_pool.get_ref())
    .await
    .unwrap_or(None);

    // 5 · Redis subscribe
    let mut pubsub = redis
        .get_async_pubsub()
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("redis subscribe"))?;
    pubsub
        .subscribe(format!("player:{player_id}:events"))
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("redis subscribe"))?;
    if let Some(fid) = faction_id {
        pubsub
            .subscribe(format!("faction:{fid}:chat"))
            .await
            .map_err(|_| actix_web::error::ErrorInternalServerError("redis subscribe"))?;
    }

    let db = db_pool.get_ref().clone();
    let redis_client = redis.get_ref().clone();

    actix::spawn(async move {
        let mut redis_stream = pubsub.on_message();
        let mut current_game: Option<Uuid> = None;

        loop {
            tokio::select! {
                // client → server
                Some(frame) = ws_stream.next() => {
                    if let Ok(Message::Text(text)) = frame {
                        if let Ok(cmsg) = serde_json::from_str::<ClientMsg>(&text) {
                            match &cmsg {
                                ClientMsg::Ready { game_id, .. }
                                | ClientMsg::Resume { game_id, .. }
                                | ClientMsg::Turn  { game_id, .. } => current_game = Some(*game_id),
                                _ => {}
                            }
                            if let Err(e) = dispatch(db.clone(), redis_client.clone(), cmsg).await {
                                log::warn!("dispatch error: {e:?}");
                            }
                        }
                    }
                }
                // redis → client
                Some(msg) = redis_stream.next() => {
                    if let Ok(json) = msg.get_payload::<String>() {
                        if let Err(e) = session.text(json).await {
                            log::warn!("WS send failed for {player_id}: {e:?}");
                            break;
                        }
                    }
                }
                else => break,
            }
        }

        // On disconnect …
        if let Ok(mut conn) = redis_client.get_multiplexed_async_connection().await {
            let _: () = conn.del(format!("session:{player_id}")).await.unwrap_or(());
        }
        if let Some(gid) = current_game {
            let _ = dispatch(
                db.clone(),
                redis_client.clone(),
                ClientMsg::Disconnected {
                    game_id: gid,
                    player_id,
                },
            )
            .await;
        }
        log::info!("WS closed for player {player_id}");
    });

    Ok(response)
}
