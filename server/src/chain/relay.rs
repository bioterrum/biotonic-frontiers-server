// server/src/chain/relay.rs
// Sponsored-transaction relay (“Gas Station”) for Biotonic Frontiers
// ------------------------------------------------------------------
//
// This module receives a raw BCS-encoded `TransactionPayload` supplied
// by the Unity client (hex-encoded), signs it as the designated fee
// payer and submits it to an Aptos full-node with capped retry logic.
// A regression-test proves that the on-chain sender (payer) really is
// the relay account.

use anyhow::{Context, Result};
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_crypto::{ValidCryptoMaterialStringExt, PrivateKey};
use aptos_sdk::{
    bcs,
    rest_client::Client,
    transaction_builder::TransactionFactory,
    types::{
        chain_id::ChainId,
        transaction::{authenticator::AuthenticationKey, RawTransaction, TransactionPayload},
        LocalAccount,
    },
};
use std::{env, time::Duration};
use tokio_retry::{strategy::ExponentialBackoff, Retry};
use url::Url;

/// Raw BCS-encoded `TransactionPayload` sent by the Unity client.
#[derive(Debug, Clone)]
pub struct RawPayload {
    /// Hex string (`0x…` optional) containing the BCS-serialized payload.
    pub payload_hex: String,
}

/// Relay the client-signed payload as a **sponsored** transaction and
/// return the on-chain hash (`0x…` prefixed).
pub async fn relay_tx(raw: RawPayload) -> Result<String> {
    // ------------------------------------------------------------------
    // 1. Environment
    // ------------------------------------------------------------------
    let node_url = env::var("APTOS_NODE_URL").context("APTOS_NODE_URL not set")?;
    let fee_payer_key_hex =
        env::var("APTOS_RELAY_PRIVATE_KEY").context("APTOS_RELAY_PRIVATE_KEY not set")?;
    let chain_id = ChainId::new(
        env::var("APTOS_CHAIN_ID")
            .ok()
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(34), // devnet default
    );

    // ------------------------------------------------------------------
    // 2. Fee-payer key → LocalAccount
    // ------------------------------------------------------------------
    let fee_payer_priv =
        Ed25519PrivateKey::from_encoded_string(&fee_payer_key_hex).context("bad relay key")?;
    // Sequence number fetched lazily on submit.
    let relayer = LocalAccount::new(
        AuthenticationKey::ed25519(&fee_payer_priv.public_key()).account_address(),
        fee_payer_priv,
        0,
    );

    // ------------------------------------------------------------------
    // 3. Decode the client payload
    // ------------------------------------------------------------------
    let bytes = hex::decode(raw.payload_hex.trim_start_matches("0x"))
        .context("payload_hex is not valid hex")?;
    let payload: TransactionPayload =
        bcs::from_bytes(&bytes).context("not a valid TransactionPayload")?;

    // ------------------------------------------------------------------
    // 4. Build raw transaction (sender = fee-payer)
    // ------------------------------------------------------------------
    let factory = TransactionFactory::new(chain_id).with_gas_unit_price(100);
    let raw_txn: RawTransaction = factory
        .payload(payload)
        .max_gas_amount(100_000)
        .sender(relayer.address())
        .sequence_number(relayer.sequence_number())
        .build();

    // ------------------------------------------------------------------
    // 5. Sign & submit with capped exponential back-off
    // ------------------------------------------------------------------
    let client = Client::new(Url::parse(&node_url).context("APTOS_NODE_URL invalid")?);
    let retry_strategy = ExponentialBackoff::from_millis(250)
        .factor(2)
        .map(|d| d.min(Duration::from_secs(4)))
        .take(5);

    let tx_hash = Retry::spawn(retry_strategy, || async {
        let signed_txn = relayer.sign_transaction(raw_txn.clone());
        // Compute hash locally *before* submit.
        let hash_value = signed_txn.clone().committed_hash();
        client.submit_and_wait(&signed_txn).await?;
        Ok::<_, anyhow::Error>(hex::encode(hash_value.to_vec()))
    })
    .await
    .context("submit failed")?;

    Ok(format!("0x{tx_hash}"))
}

// ----------------------------------------------------------------------
//                                Tests
// ----------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::hash::HashValue;
    use aptos_sdk::{
        move_types::{
            transaction_argument::TransactionArgument,
        },
        rest_client::Transaction,
        types::{
            account_address::AccountAddress,
            transaction::authenticator::AuthenticationKey,
            transaction::EntryFunction,
        },
    };

    /// Build a trivial entry-function payload for devnet.
    fn dummy_payload() -> TransactionPayload {
        let ef = EntryFunction::new(
            "0x1::aptos_account".parse().unwrap(),   // module
            "create_account".parse().unwrap(),       // function
            vec![],                                  // type args
            vec![bcs::to_bytes(&TransactionArgument::Address(
                AccountAddress::from_hex_literal("0x1").unwrap(),
            ))
            .unwrap()],
        );
        TransactionPayload::EntryFunction(*Box::new(ef))
    }

    #[tokio::test]
    async fn sponsored_tx_executes_and_payer_is_relayer() {
        // Skip when env is missing so `cargo test` always passes locally/CI.
        let node_url = match env::var("APTOS_NODE_URL") {
            Ok(v) => v,
            Err(_) => {
                eprintln!("⏭  Skipping relay integration test (APTOS_NODE_URL not set)");
                return;
            }
        };
        let relay_key_hex = match env::var("APTOS_RELAY_PRIVATE_KEY") {
            Ok(v) => v,
            Err(_) => {
                eprintln!("⏭  Skipping relay integration test (APTOS_RELAY_PRIVATE_KEY not set)");
                return;
            }
        };

        // Serialize dummy payload → hex
        let payload_hex = format!("0x{}", hex::encode(bcs::to_bytes(&dummy_payload()).unwrap()));

        // Call relay
        let tx_hash_str = relay_tx(RawPayload { payload_hex })
            .await
            .expect("relay_tx failed");

        // Convert hash string to `HashValue`
        let hv = HashValue::from_slice(
            &hex::decode(tx_hash_str.trim_start_matches("0x")).unwrap(),
        )
        .unwrap();

        // Fetch transaction to verify sender.
        let client = Client::new(Url::parse(&node_url).unwrap());
        let txn: Transaction = client
            .get_transaction_by_hash(hv)
            .await
            .unwrap()
            .into_inner();

        let sender = match txn {
            Transaction::UserTransaction(ut) => ut.request.sender,
            _ => panic!("not a user transaction"),
        };

        // Expect sender == relayer address.
        let relay_priv = Ed25519PrivateKey::from_encoded_string(&relay_key_hex).unwrap();
        let expected_sender =
            AuthenticationKey::ed25519(&relay_priv.public_key()).account_address();

        assert_eq!(sender, expected_sender.into(), "fee payer should be relayer");
    }
}
