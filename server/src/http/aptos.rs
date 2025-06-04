use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;

use crate::chain::relay_tx;

#[derive(Deserialize)]
struct SponsoredReq {
    payload_hex: String,
}

/// POST /api/tx/sponsored  { payload_hex }
#[post("/tx/sponsored")]
async fn sponsored(web::Json(req): web::Json<SponsoredReq>) -> impl Responder {
    match relay_tx(crate::chain::relay::RawPayload { payload_hex: req.payload_hex }).await {
        Ok(hash) => HttpResponse::Ok().json(serde_json::json!({"hash": hash})),
        Err(e) => {
            log::warn!("sponsored‑tx error: {e:?}");
            HttpResponse::InternalServerError().body("relay failed")
        }
    }
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(sponsored);
}