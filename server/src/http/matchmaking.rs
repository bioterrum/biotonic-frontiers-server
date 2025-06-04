use actix_web::{post, web, HttpResponse, Responder};
use chrono::Utc;
use redis::{AsyncCommands, Client as RedisClient};
use serde::Deserialize;
use uuid::Uuid;

/// Body for both join & leave.
#[derive(Deserialize)]
pub struct QueueRequest {
    /// Player’s UUID (client already knows this)
    pub player_id: Uuid,
    /// Current Elo rating (used for pairing score)
    pub elo_rating: i32,
}

/// POST /api/matchmaking/join
#[post("/matchmaking/join")]
async fn join_queue(
    info: web::Json<QueueRequest>,
    redis: web::Data<RedisClient>,
) -> impl Responder {
    let mut conn = match redis.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("Redis unavailable"),
    };

    // Score = Elo + tiny time component, so older entrants with same Elo get priority
    let score = info.elo_rating as f64 + (Utc::now().timestamp_millis() as f64) * 1e-6;
    let _: () = conn
        .zadd("mm:queue", info.player_id.to_string(), score)
        .await
        .unwrap_or(());

    HttpResponse::Ok().json(serde_json::json!({ "status": "queued" }))
}

/// POST /api/matchmaking/leave
#[post("/matchmaking/leave")]
async fn leave_queue(
    info: web::Json<QueueRequest>,
    redis: web::Data<RedisClient>,
) -> impl Responder {
    let mut conn = match redis.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("Redis unavailable"),
    };

    let _: () = conn
        .zrem("mm:queue", info.player_id.to_string())
        .await
        .unwrap_or(());

    HttpResponse::Ok().body("Left matchmaking queue")
}

/// Mount
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(join_queue).service(leave_queue);
}
