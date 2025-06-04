use crate::ws::index::ws_index;
use actix_web::web;

/// Mount the WebSocket endpoint
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/ws/", web::get().to(ws_index));
}
