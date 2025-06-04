//! Simple liveness / readiness probe

use actix_web::{get, web, HttpResponse, Responder};
use redis::{AsyncCommands, Client as RedisClient};
use sqlx::PgPool;

#[get("/healthz")]
pub async fn healthz(db: web::Data<PgPool>, redis: web::Data<RedisClient>) -> impl Responder {
    // Check Postgres
    if sqlx::query("SELECT 1").execute(&**db).await.is_err() {
        return HttpResponse::ServiceUnavailable().body("db");
    }

    // Check Redis
    let mut conn = match redis.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::ServiceUnavailable().body("redis"),
    };
    // Annotate ping return type so compiler can infer RV
    if conn.ping::<String>().await.is_err() {
        return HttpResponse::ServiceUnavailable().body("redis");
    }

    HttpResponse::Ok().body("ok")
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(healthz);
}
