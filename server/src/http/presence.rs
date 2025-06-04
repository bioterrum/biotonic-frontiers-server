// src/http/presence.rs
//! Very thin presence API backed by Redis session keys.

use actix_web::{get, web, HttpResponse, Responder};
use redis::{AsyncCommands, Client as RedisClient};
use uuid::Uuid;

#[get("/presence/online/{player_id}")]
pub async fn online(path: web::Path<Uuid>, redis: web::Data<RedisClient>) -> impl Responder {
    let pid = path.into_inner();
    let key = format!("session:{pid}");
    let mut conn = match redis.get_multiplexed_async_connection().await {
        Ok(c) => c,
        Err(_) => return HttpResponse::InternalServerError().body("redis down"),
    };

    match conn.exists(&key).await {
        Ok(true) => HttpResponse::Ok().json(serde_json::json!({ "online": true })),
        Ok(false) => HttpResponse::Ok().json(serde_json::json!({ "online": false })),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(online);
}
