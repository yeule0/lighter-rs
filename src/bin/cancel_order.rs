//! Submit a CancelOrder to Lighter testnet.
//!
//! Usage: replace the values below and run `cargo run --release --bin cancel_order`

mod common;
use common::*;

// ── Replace these with your own values ────────────────────────────
const PRIVATE_KEY_HEX: &str = "your_private_key_hex_here";
const ACCOUNT_INDEX: i64 = 0; // your account index
const API_KEY_INDEX: u8 = 0; // your api key index
const BASE_URL: &str = "https://testnet.zklighter.elliot.ai";
const CHAIN_ID: u32 = 300;
const ORDER_INDEX: i64 = 1;    // The order to cancel — replace with actual order index
// ──────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer = Signer::new(PRIVATE_KEY_HEX)?;
    let api = ApiClient::new(BASE_URL, CHAIN_ID);

    println!("lighter-rs  CancelOrder  (testnet)");
    println!("  account     {}", ACCOUNT_INDEX);
    println!("  api key     {}", API_KEY_INDEX);
    println!("  order idx   {}", ORDER_INDEX);
    println!();

    let nonce = api.next_nonce(ACCOUNT_INDEX, API_KEY_INDEX)?;
    println!("  nonce       {}", nonce);

    let expired_at = now_ms() + 24 * 60 * 60 * 1000;
    let mut tx = CancelOrderTx {
        account_index: ACCOUNT_INDEX,
        api_key_index: API_KEY_INDEX,
        market_index: 0,
        index: ORDER_INDEX,
        expired_at,
        nonce,
        sig: String::new(),
    };

    let hash = hash_cancel_order(
        api.chain_id, ACCOUNT_INDEX, API_KEY_INDEX,
        tx.market_index, tx.index, tx.nonce, tx.expired_at,
    );
    tx.sig = signer.sign_base64(&hash)?;

    let tx_info = serde_json::to_string(&tx)?;
    let server_hash = api.submit(15, &tx_info)?;

    println!("  ✓ submitted");
    println!("  tx hash     {}", server_hash);
    Ok(())
}
