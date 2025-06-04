use std::convert::TryFrom;

use aptos_crypto::{
    ed25519::Ed25519PrivateKey,
    PrivateKey
};
use aptos_sdk::{
    rest_client::Client,
    transaction_builder::TransactionFactory,
    types::{
        chain_id::ChainId,
        transaction::authenticator::AuthenticationKey,
        LocalAccount,
    },
};
use hex::FromHex;
use serde::Deserialize;
use url::Url;

/// JSON body accepted by the relay endpoint.
///
/// The front-end builds a `ScriptFunction` (or `EntryFunction`) off-chain,
/// BCS-encodes it, hex-encodes the bytes, then POSTs the string here.
#[derive(Debug, Deserialize)]
pub struct RawPayload {
    pub payload_hex: String,
}

/// Submit the user-supplied payload and return the resulting transaction hash.
pub async fn relay_tx(data: RawPayload) -> anyhow::Result<String> {
    /*────────── configuration & key handling ───────────────────────────────*/
    let rest_url = std::env::var("APTOS_NODE_URL")?;
    let key_hex = std::env::var("APTOS_RELAY_PRIVATE_KEY")?;
    let key_bytes = Vec::<u8>::from_hex(key_hex.trim_start_matches("0x"))?;
    let priv_key = Ed25519PrivateKey::try_from(key_bytes.as_slice())?;

    /*────────── build helper objects ───────────────────────────────────────*/
    let auth_key = AuthenticationKey::ed25519(&priv_key.public_key());
    let relayer_addr = auth_key.account_address();

    let client = Client::new(Url::parse(&rest_url)?);
    let seq = client
        .get_account(relayer_addr)
        .await?
        .into_inner()
        .sequence_number;

    let relayer = LocalAccount::new(relayer_addr, priv_key, seq);

    /*────────── transaction factory ────────────────────────────────────────*/
    let chain_id = {
        let info = client.get_ledger_information().await?.into_inner();
        ChainId::new(info.chain_id)
    };

    let factory = TransactionFactory::new(chain_id)
        .with_gas_unit_price(1)      // devnet is cheap
        .with_max_gas_amount(1_000); // safe upper-bound

    /*────────── decode, sign & submit ──────────────────────────────────────*/
    let payload_bytes = Vec::<u8>::from_hex(data.payload_hex.trim_start_matches("0x"))?;
    let script_fn = aptos_sdk::bcs::from_bytes(&payload_bytes)?; // still works for EntryFunction

    let tx = relayer.sign_with_transaction_builder(factory.payload(script_fn));
    let pending = client.submit(&tx).await?.into_inner();

    Ok(pending.hash.to_string())
}
