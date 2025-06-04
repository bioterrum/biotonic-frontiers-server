//! Faction chat: history + send

use actix_web::{get, post, web, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::protocol::ServerMsg;

//////////////////////////////////////////////////
// DTOs
//////////////////////////////////////////////////

#[derive(Deserialize)]
pub struct SendReq {
    pub faction_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
}

#[derive(Serialize)]
pub struct ChatMsgRow {
    pub sender_id: Uuid,
    pub content: String,
    pub ts: DateTime<Utc>,
}

//////////////////////////////////////////////////
// POST /api/chat/faction/send
//////////////////////////////////////////////////
#[post("/chat/faction/send")]
pub async fn send(
    info: web::Json<SendReq>,
    db: web::Data<PgPool>,
    redis: web::Data<RedisClient>,
) -> impl Responder {
    if info.content.trim().is_empty() || info.content.len() > 500 {
        return HttpResponse::BadRequest().body("content length 1–500 required");
    }

    // Make sure sender is a member
    let member: Option<bool> = sqlx::query_scalar!(
        "SELECT EXISTS(
             SELECT 1 FROM faction_members
              WHERE faction_id = $1 AND player_id = $2)",
        info.faction_id,
        info.sender_id
    )
    .fetch_one(&**db)
    .await
    .unwrap_or(Some(false));

    if member != Some(true) {
        return HttpResponse::Unauthorized().body("not in faction");
    }

    // Persist message
    let _ = sqlx::query!(
        "INSERT INTO chat_messages (faction_id, sender_id, content)
              VALUES ($1,$2,$3)",
        info.faction_id,
        info.sender_id,
        info.content
    )
    .execute(&**db)
    .await;

    // Publish via Redis
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let evt = ServerMsg::FactionChat {
            faction_id: info.faction_id,
            sender_id: info.sender_id,
            content: info.content.clone(),
            ts: Utc::now(),
        };
        let _: () = conn
            .publish(
                format!("faction:{}:chat", info.faction_id),
                serde_json::to_string(&evt).unwrap(),
            )
            .await
            .unwrap_or(());
    }

    HttpResponse::Ok().body("sent")
}

//////////////////////////////////////////////////
// GET /api/chat/faction/history/{faction_id}?limit=50
//////////////////////////////////////////////////
#[derive(Deserialize)]
pub struct HistoryParams {
    #[serde(default = "default_limit")]
    pub limit: i64,
}
fn default_limit() -> i64 {
    50
}

#[get("/chat/faction/history/{faction_id}")]
pub async fn history(
    path: web::Path<Uuid>,
    web::Query(params): web::Query<HistoryParams>,
    db: web::Data<PgPool>,
) -> impl Responder {
    let fid = path.into_inner();
    let rows = sqlx::query_as!(
        ChatMsgRow,
        r#"
        SELECT sender_id, content, sent_at AS "ts!"
          FROM chat_messages
         WHERE faction_id = $1
         ORDER BY id DESC
         LIMIT $2
        "#,
        fid,
        params.limit
    )
    .fetch_all(&**db)
    .await
    .unwrap_or_default();

    HttpResponse::Ok().json(rows.into_iter().rev().collect::<Vec<_>>())
}

//////////////////////////////////////////////////
// Mount
//////////////////////////////////////////////////
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(send).service(history);
}
