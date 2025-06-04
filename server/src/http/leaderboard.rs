// src/http/leaderboard.rs

use actix_web::{get, web, HttpResponse, Responder};
use redis::{AsyncCommands, Client as RedisClient};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct LeaderboardParams {
    /// Maximum number of entries to return.
    pub limit: i64,
}

#[get("/leaderboard")]
pub async fn leaderboard(
    db: web::Data<PgPool>,
    redis: web::Data<RedisClient>,
    web::Query(params): web::Query<LeaderboardParams>,
) -> impl Responder {
    // 1) Try to read from Redis cache
    let key = format!("leaderboard:{}", params.limit);
    let mut conn = match redis.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("Redis unavailable"),
    };
    if let Ok(cached) = conn.get::<_, String>(&key).await {
        return HttpResponse::Ok()
            .content_type("application/json")
            .body(cached);
    }

    // 2) Query the database
    let rows: Vec<(Uuid, String, i32)> = match sqlx::query_as::<_, (Uuid, String, i32)>(
        r#"
        SELECT p.id, p.nickname, p.elo_rating
          FROM players p
         ORDER BY p.elo_rating DESC, p.created_at
         LIMIT $1
        "#,
    )
    .bind(params.limit)
    .fetch_all(db.get_ref())
    .await
    {
        Ok(r) => r,
        Err(_) => return HttpResponse::InternalServerError().body("DB error"),
    };

    // 3) Serialize and cache the result (type-annotate to satisfy Redis)
    let body = match serde_json::to_string(&rows) {
        Ok(b) => b,
        Err(_) => return HttpResponse::InternalServerError().body("Serialization error"),
    };
    let _: () = conn.set_ex(&key, &body, 30).await.unwrap();

    // 4) Return JSON response
    HttpResponse::Ok().json(rows)
}

/// Mounts the leaderboard route under `/api`
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(leaderboard);
}
