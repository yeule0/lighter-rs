use std::time::{Duration, Instant};

use lighter_rs::field::goldilocks::GoldilocksField;
use lighter_rs::field::quintic::Fp5;
use lighter_rs::hash::poseidon2;
use lighter_rs::signer::KeyManager;

const CHAIN_ID: u32 = 300;
const MARKET_INDEX: u8 = 0;
const ACCOUNT_INDEX: i64 = 200;
const API_KEY_INDEX: u8 = 1;

const ITERATIONS: u32 = 101;
const STARTING_NONCE: i64 = 1000;

#[allow(clippy::too_many_arguments)]
fn build_create_order_hash(
    chain_id: u32,
    nonce: i64,
    account_index: i64,
    api_key_index: u8,
    market_index: u8,
    base_amount: i64,
    price: u32,
    order_expiry: i64,
) -> Fp5 {
    let elems = vec![
        GoldilocksField(chain_id as u64),
        GoldilocksField(14), // create order
        GoldilocksField(nonce as u64),
        GoldilocksField(order_expiry as u64),
        GoldilocksField(account_index as u64),
        GoldilocksField(api_key_index as u64),
        GoldilocksField(market_index as u64),
        GoldilocksField(nonce as u64),
        GoldilocksField(base_amount as u64),
        GoldilocksField(price as u64),
        GoldilocksField(0u64),
        GoldilocksField(0u64),
        GoldilocksField(1u64),
        GoldilocksField(0u64),
        GoldilocksField(0u64),
        GoldilocksField(order_expiry as u64),
    ];
    poseidon2::hash_to_quintic_extension(&elems)
}

fn main() {
    let km = KeyManager::generate();

    let mut durations: Vec<Duration> = Vec::with_capacity((ITERATIONS - 1) as usize);
    let mut current_nonce = STARTING_NONCE;
    let base_amount: i64 = 100;
    let price: u32 = 140_000;
    let expiry: i64 = 9_999_999_999;

    for i in 0..ITERATIONS {
        let nonce = current_nonce;
        current_nonce += 1;

        let hash = build_create_order_hash(
            CHAIN_ID, nonce, ACCOUNT_INDEX, API_KEY_INDEX,
            MARKET_INDEX, base_amount, price, expiry,
        );
        let hash_bytes = hash.to_bytes_le();

        let start = Instant::now();
        let _sig = km.sign(&hash_bytes).unwrap();
        let duration = start.elapsed();

        if i > 0 {
            durations.push(duration);
        }
    }

    durations.sort();
    let n = durations.len();
    let total: Duration = durations.iter().sum();
    let avg = total / n as u32;
    let min = durations[0];
    let max = durations[n - 1];
    let p50 = durations[n / 2];
    let p99 = durations[n * 99 / 100];

    println!("lighter-rs  CreateOrder sign  (hash + schnorr)");
    println!();
    println!("  samples      {n}");
    println!("  avg          {:.2} µs", avg.as_nanos() as f64 / 1000.0);
    println!("  min          {:.2} µs", min.as_nanos() as f64 / 1000.0);
    println!("  p50          {:.2} µs", p50.as_nanos() as f64 / 1000.0);
    println!("  p99          {:.2} µs", p99.as_nanos() as f64 / 1000.0);
    println!("  max          {:.2} µs", max.as_nanos() as f64 / 1000.0);
    println!();
    println!("  throughput   {:.0} sigs/sec", n as f64 / total.as_secs_f64());
}
