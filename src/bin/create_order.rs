//! Submit a CreateOrder to Lighter testnet.
//!
//! Usage: replace the values below and run `cargo run --release --bin create_order`

mod common;
use common::*;

// ── Replace these with your own values ────────────────────────────
const PRIVATE_KEY_HEX: &str = "your_private_key_hex_here";
const ACCOUNT_INDEX: i64 = 0; // your account index
const API_KEY_INDEX: u8 = 0; // your api key index
const BASE_URL: &str = "https://testnet.zklighter.elliot.ai";
const CHAIN_ID: u32 = 300;
// ──────────────────────────────────────────────────────────────────

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer = Signer::new(PRIVATE_KEY_HEX)?;
    let api = ApiClient::new(BASE_URL, CHAIN_ID);

    println!("lighter-rs  CreateOrder  (testnet)");
    println!("  account  {}", ACCOUNT_INDEX);
    println!("  api key  {}", API_KEY_INDEX);
    println!();

    let nonce = api.next_nonce(ACCOUNT_INDEX, API_KEY_INDEX)?;
    println!("  nonce    {}", nonce);

    let order_expiry = now_ms() + 60 * 60 * 1000; // 1 hour
    let expired_at = now_ms() + 24 * 60 * 60 * 1000;

    let mut tx = CreateOrderTx {
        account_index: ACCOUNT_INDEX,
        api_key_index: API_KEY_INDEX,
        market_index: 0,           // ETH-PERP
        client_order_index: nonce,
        base_amount: 10_000,       // 0.01 ETH
        price: 100_000,            // $1000.00
        is_ask: 0,                 // Bid
        order_type: 0,             // Limit
        time_in_force: 1,          // GTT
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

    println!("  tx hash  {}", hex::encode(hash));

    let tx_info = serde_json::to_string(&tx)?;
    let server_hash = api.submit(14, &tx_info)?;

    println!();
    println!("  ✓ submitted");
    println!("  tx hash  {}", server_hash);
    assert_eq!(server_hash, hex::encode(hash), "server hash mismatch!");
    Ok(())
}
