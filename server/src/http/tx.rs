// server/src/http/tx.rs
// -----------------------------------------------------------------------------
// HTTP → Aptos “Gas Station” bridge
// -----------------------------------------------------------------------------
// Exposes POST /api/tx/sponsored for the Unity client. It receives a raw
// BCS‑encoded `TransactionPayload` hex string, forwards it to `chain::relay_tx`,
// and returns `{ "tx_hash": "0x…" }` on success.
//
// Aligns with Sprint‑9 acceptance criteria (#4 in sprint_9_05-29‑2025.odt) that
// “POST /api/tx/sponsored signs & submits raw tx; unit test asserts payer =
// relayer; tx confirmed on chain.” fileciteturn1file7
// -----------------------------------------------------------------------------

use actix_web::{post, web, HttpResponse};
use serde::{Deserialize, Serialize};

use crate::chain::relay::{relay_tx, RawPayload};

/// JSON body expected from client
#[derive(Debug, Deserialize)]
pub struct SponsoredTxRequest {
    /// Hex string (with or without 0x) of BCS‑encoded `TransactionPayload`.
    pub payload_hex: String,
}

/// JSON response on success
#[derive(Debug, Serialize)]
pub struct SponsoredTxResponse {
    /// On‑chain transaction hash ("0x…")
    pub tx_hash: String,
}

/// POST /api/tx/sponsored
#[post("/api/tx/sponsored")]
async fn sponsored_tx(
    req: web::Json<SponsoredTxRequest>,
) -> Result<HttpResponse, actix_web::Error> {
    // Delegate to relay module (fee‑payer signing + submission)
    match relay_tx(RawPayload {
        payload_hex: req.payload_hex.clone(),
    })
    .await
    {
        Ok(hash) => Ok(HttpResponse::Created().json(SponsoredTxResponse { tx_hash: hash })),
        Err(e) => {
            tracing::error!(?e, "relay_tx failed");
            Err(actix_web::error::ErrorInternalServerError(e))
        }
    }
}

/// Hook for `main.rs` → `.configure(tx::config)`
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(sponsored_tx);
}
