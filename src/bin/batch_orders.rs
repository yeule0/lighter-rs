//! Submit 3 CreateOrders in one batch to Lighter testnet.
//!
//! Usage: replace the values below and run `cargo run --release --bin batch_orders`

mod common;
use common::*;
use serde::Deserialize;

// ── Replace these with your own values ────────────────────────────
const PRIVATE_KEY_HEX: &str = "your_private_key_hex_here";
const ACCOUNT_INDEX: i64 = 0; // your account index
const API_KEY_INDEX: u8 = 0; // your api key index
const BASE_URL: &str = "https://testnet.zklighter.elliot.ai";
const CHAIN_ID: u32 = 300;
const BATCH_SIZE: i64 = 3;
// ──────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer = Signer::new(PRIVATE_KEY_HEX)?;
    let api = ApiClient::new(BASE_URL, CHAIN_ID);

    println!("lighter-rs  BatchCreateOrder x{}  (testnet)", BATCH_SIZE);
    println!("  account  {}", ACCOUNT_INDEX);
    println!();

    let base_nonce = api.next_nonce(ACCOUNT_INDEX, API_KEY_INDEX)?;

    let order_expiry = now_ms() + 60 * 60 * 1000;
    let expired_at = now_ms() + 24 * 60 * 60 * 1000;

    let mut tx_infos = Vec::new();
    let mut tx_types = Vec::new();

    for i in 0..BATCH_SIZE {
        let nonce = base_nonce + i;
        print!("  [{}] nonce {}  ", i, nonce);

        let mut tx = CreateOrderTx {
            account_index: ACCOUNT_INDEX,
            api_key_index: API_KEY_INDEX,
            market_index: 0,
            client_order_index: nonce + i,
            base_amount: 10_000 + i * 1000,
            price: 100_000u32.saturating_sub((i as u32) * 1000),
            is_ask: (i % 2) as u8,
            order_type: 0,
            time_in_force: 1,
            reduce_only: 0,
            trigger_price: 0,
            order_expiry,
            expired_at,
            nonce,
            sig: String::new(),
        };

        let hash = hash_create_order(
            api.chain_id, ACCOUNT_INDEX, API_KEY_INDEX,
            tx.market_index, tx.client_order_index, tx.base_amount,
            tx.price, tx.is_ask, tx.order_type, tx.time_in_force,
            tx.reduce_only, tx.trigger_price, tx.order_expiry,
            tx.nonce, tx.expired_at,
        );
        tx.sig = signer.sign_base64(&hash)?;

        tx_infos.push(serde_json::to_string(&tx)?);
        tx_types.push(14u8);
        println!("hash {}", hex::encode(hash));
    }

    println!();
    println!("Submitting batch...");

    #[derive(Deserialize)]
    struct R {
        tx_hash: Option<Vec<String>>,
        message: Option<String>,
    }

    let resp_str = ureq::post(&format!("{}/api/v1/sendTxBatch", api.base_url))
        .send_form(&[
            ("tx_types", &serde_json::to_string(&tx_types).unwrap()),
            ("tx_infos", &serde_json::to_string(&tx_infos).unwrap()),
        ])
        .map_err(|e| match e {
            ureq::Error::Status(code, r) =>
                format!("HTTP {}: {}", code, r.into_string().unwrap_or_default()),
            _ => format!("HTTP error: {e}"),
        })?
        .into_string()?;

    let r: R = serde_json::from_str(&resp_str)
        .map_err(|e| format!("parse response: {e}"))?;

    if let Some(hashes) = r.tx_hash {
        println!("  ✓ submitted {} orders", hashes.len());
        for (i, h) in hashes.iter().enumerate() {
            println!("  [{}] {}", i, h);
        }
    } else {
        println!("  ✗ {}", r.message.unwrap_or_default());
    }

    Ok(())
}
