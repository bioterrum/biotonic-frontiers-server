use actix_web::{middleware::Logger, web, App, HttpServer};
use biotonic_server::{cache, ecosystem, http, matchmaking, metrics, ws};
use redis::Client as RedisClient;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    // Configuration
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let redis_url = env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1/".into());
    let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".into());

    // Postgres pool
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create Postgres pool");

    // Redis client
    let redis_client = RedisClient::open(redis_url.as_str()).expect("Invalid REDIS_URL");

    // ───── Sprint 1: Warm in-memory caches ─────
    cache::warm_all(&db_pool).await;

    // Start the background matchmaking & ecosystem loops
    matchmaking::start(redis_client.clone(), db_pool.clone());
    ecosystem::simulation::start(db_pool.clone(), redis_client.clone());

    // Start HTTP + WS server
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .wrap(metrics::METRICS.clone())
            .app_data(web::Data::new(db_pool.clone()))
            .app_data(web::Data::new(redis_client.clone()))
            .configure(http::routes::init_routes)
            .configure(ws::routes::init_routes)
    })
    .bind(&server_addr)?
    .run()
    .await
}
