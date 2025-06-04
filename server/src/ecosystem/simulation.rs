//! Minute-tick heartbeat (placeholder for real biome logic).

use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client as RedisClient};
use serde::Serialize;
use sqlx::PgPool;
use tokio::time::{sleep, Duration};

#[derive(Serialize)]
struct WorldEvent {
    ts: DateTime<Utc>,
    kind: &'static str,
}

async fn tick(db: &PgPool, redis: &RedisClient) {
    // Cheap liveness query (replace with real biome math later)
    let _ = sqlx::query("SELECT 1").execute(db).await;

    let evt = WorldEvent {
        ts: Utc::now(),
        kind: "heartbeat",
    };
    if let Ok(mut conn) = redis.get_multiplexed_async_connection().await {
        let _: () = conn
            .publish("world:events", serde_json::to_string(&evt).unwrap())
            .await
            .unwrap_or(());
    }
}

pub async fn run(db: PgPool, redis: RedisClient) {
    loop {
        tick(&db, &redis).await;
        sleep(Duration::from_secs(60)).await;
    }
}

pub fn start(db: PgPool, redis: RedisClient) {
    tokio::spawn(run(db, redis));
}
