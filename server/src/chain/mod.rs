pub mod relay;
mod listener;

pub use relay::relay_tx;

use aptos_sdk::rest_client::{Client, aptos_api_types::Transaction};
use once_cell::sync::Lazy;

/// Singleton REST client built from `APTOS_NODE_URL`.
static APTOS: Lazy<Client> = Lazy::new(|| {
    let url = std::env::var("APTOS_NODE_URL").expect("APTOS_NODE_URL env");
    Client::new(url::Url::parse(&url).expect("valid APTOS_NODE_URL"))
});

pub async fn tx_status(hash: &str) -> anyhow::Result<Transaction> {
    Ok(
        APTOS
            .get_transaction_by_hash(hash.parse()?)
            .await?
            .into_inner(),
    )
}