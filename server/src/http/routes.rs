use crate::http;
use actix_web::web;

/// Mount every HTTP sub-module under `/api`.
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(http::auth::init_routes)
            .configure(http::matchmaking::init_routes)
            .configure(http::items::init_routes)
            .configure(http::inventory::init_routes)
            .configure(http::shop::init_routes)
            .configure(http::trades::init_routes)
            .configure(http::factions::init_routes)
            .configure(http::land::init_routes)
            .configure(http::structures::init_routes)
            .configure(http::leaderboard::init_routes)
            .configure(http::games::init_routes)
            .configure(http::presence::init_routes)
            .configure(http::chat::init_routes)
            .configure(http::health::init_routes)
            .configure(http::land::init_routes)
            .configure(http::aptos::init_routes)
            .configure(http::tx::init_routes)
    );
}
