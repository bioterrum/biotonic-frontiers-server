use super::APTOS;
use sqlx::PgPool;
use tokio::time::{sleep, Duration};

use serde_json::Value;

/// Continuously poll the full-node, capture every transaction whose write-set
/// mentions `collection`, and persist the raw on-chain payload so that the rest
/// of the backend can consume it asynchronously.
pub async fn run(db: PgPool, collection: &str) -> anyhow::Result<()> {
    let mut last_seen_version = 0u64;

    loop {
        match APTOS.get_transactions(None, Some(100)).await {
            Ok(resp) => {
                for tx in resp.into_inner() {
                    // `version()` is still the stable helper.
                    let version = tx.version().unwrap_or_default();
                    if version <= last_seen_version {
                        continue;
                    }

                    // Serialise once so we can (1) inspect for a match and
                    // (2) insert the exact JSON blob into Postgres.
                    let tx_json: Value = serde_json::to_value(&tx)?;
                    let tx_str = tx_json.to_string();

                    if tx_str.contains(collection) {
                        // The REST representation exposes `hash` as a field,
                        // not a method, in the latest SDK.
                        let hash = tx_json
                            .get("hash")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_owned();

                        sqlx::query!(
                            r#"
                            INSERT INTO chain_events (version, hash, payload)
                            VALUES ($1, $2, $3)
                            ON CONFLICT (version) DO NOTHING
                            "#,
                            version as i64,
                            hash,
                            tx_json
                        )
                        .execute(&db)
                        .await
                        .ok();
                    }

                    last_seen_version = version;
                }
            }
            Err(e) => log::warn!("aptos-listener: {e:?}"),
        }

        // ~0.5 req/s is gentle enough for public full-nodes.
        sleep(Duration::from_secs(2)).await;
    }
}
