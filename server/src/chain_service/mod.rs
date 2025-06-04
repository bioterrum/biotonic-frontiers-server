//! Async worker that tails Aptos events -> Postgres
use aptos_sdk::{move_types::account_address::AccountAddress, types::transaction::SignedTransaction};
use tokio_stream::StreamExt;
use sqlx::PgPool;

pub async fn run(pool: PgPool, node_url: &str) -> anyhow::Result<()> {
    let client = aptos_sdk::client::Client::new(node_url);
    let mut stream = client.stream_transactions(None /* start at latest */).await?;
    while let Some(tx) = stream.next().await {
        if let Ok(txn) = tx {
            sqlx::query!("INSERT INTO chain_events (version, hash, payload) VALUES ($1,$2,$3) ON CONFLICT DO NOTHING",
                txn.version, txn.hash.to_string(), serde_json::to_value(&txn)?).execute(&pool).await?;
        }
    }
    Ok(())
}