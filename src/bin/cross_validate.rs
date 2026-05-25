use std::time::Instant;
use std::env;

use lighter_rs::field::goldilocks::GoldilocksField;
use lighter_rs::field::quintic::Fp5;
use lighter_rs::hash::poseidon2;
use lighter_rs::curve::scalar::Scalar;
use lighter_rs::curve::point::Point;
use lighter_rs::signature::schnorr;
use lighter_rs::batch;

fn hex_to_bytes<const N: usize>(s: &str) -> [u8; N] {
    let bytes = hex::decode(s).expect("hex decode");
    bytes.try_into().unwrap()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let json_str = if args.len() > 1 {
        std::fs::read_to_string(&args[1]).expect("read file")
    } else {
        eprintln!("Usage: cross_validate <vector.json>");
        eprintln!("Reading from stdin...");
        std::io::read_to_string(std::io::stdin()).expect("read stdin")
    };

    let v: serde_json::Value = serde_json::from_str(&json_str).expect("parse json");

    let sk_bytes = hex_to_bytes::<40>(v["private_key"].as_str().unwrap());
    let k_bytes  = hex_to_bytes::<40>(v["sign_nonce"].as_str().unwrap());
    let pk_go    = v["public_key"].as_str().unwrap();
    let hash_go  = v["tx_hash"].as_str().unwrap();
    let sig_go   = v["signature"].as_str().unwrap();
    let chain_id = v["chain_id"].as_u64().unwrap() as u32;

    let sk = Scalar::from_bytes_le(&sk_bytes);
    let k  = Scalar::from_bytes_le(&k_bytes);

    // Build the same hash as Go
    let mut elems = Vec::with_capacity(16);
    macro_rules! gf { ($x:expr) => { GoldilocksField($x as u64) }; }
    elems.push(gf!(chain_id));
    elems.push(gf!(14));
    elems.push(gf!(v["nonce"].as_i64().unwrap()));
    elems.push(gf!(v["expired_at"].as_i64().unwrap()));
    elems.push(gf!(v["account_index"].as_i64().unwrap()));
    elems.push(gf!(v["api_key_index"].as_u64().unwrap()));
    elems.push(gf!(v["market_index"].as_i64().unwrap()));
    elems.push(gf!(v["client_order_index"].as_i64().unwrap()));
    elems.push(gf!(v["base_amount"].as_i64().unwrap()));
    elems.push(gf!(v["price"].as_u64().unwrap()));
    elems.push(gf!(v["is_ask"].as_u64().unwrap()));
    elems.push(gf!(v["order_type"].as_u64().unwrap()));
    elems.push(gf!(v["time_in_force"].as_u64().unwrap()));
    elems.push(gf!(v["reduce_only"].as_u64().unwrap()));
    elems.push(gf!(v["trigger_price"].as_u64().unwrap()));
    elems.push(gf!(v["order_expiry"].as_i64().unwrap()));

    let tx_hash = poseidon2::hash_to_quintic_extension(&elems);
    let tx_hash_hex = hex::encode(tx_hash.to_bytes_le());

    let sig = schnorr::sign_with_nonce(&tx_hash, &sk, &k);
    let sig_hex = hex::encode(sig.to_bytes());

    let pk_go_fp5 = Fp5::from_bytes_le(&hex::decode(pk_go).unwrap()).unwrap();
    let ok = schnorr::verify(&pk_go_fp5, &tx_hash, &sig);
    let pk_derived = schnorr::public_key_from_secret(&sk);
    let pk_derived_hex = hex::encode(pk_derived.to_bytes_le());

    println!("=== Cross-Validation (Rust vs Go) ===");
    println!("Tx hash:      {}", if tx_hash_hex == hash_go { "✓ MATCH" } else { "✗ MISMATCH" });
    println!("Signature:    {}", if sig_hex == sig_go { "✓ MATCH" } else { "✗ MISMATCH" });
    println!("Verify:       {}", if ok { "✓ PASS" } else { "✗ FAIL" });
    println!("PK derive:    {}", if pk_derived_hex == pk_go { "✓ MATCH" } else { "✗ MISMATCH" });

    // Timing
    let n = 100_000;
    let start = Instant::now();
    for _ in 0..n { let _ = schnorr::sign_with_nonce(&tx_hash, &sk, &k); }
    let det_us = start.elapsed().as_micros() as f64 / n as f64;

    let start = Instant::now();
    for _ in 0..n { let _ = schnorr::sign_hashed_message(&tx_hash, &sk); }
    let prod_us = start.elapsed().as_micros() as f64 / n as f64;

    let start = Instant::now();
    for _ in 0..n { let _ = Point::mul_generator(&k); }
    let kg_us = start.elapsed().as_micros() as f64 / n as f64;

    let start = Instant::now();
    for _ in 0..n { let _ = poseidon2::hash_to_quintic_extension(&elems); }
    let hash_us = start.elapsed().as_micros() as f64 / n as f64;

    let r = Point::mul_generator(&k);
    let start = Instant::now();
    for _ in 0..n { let _ = r.encode(); }
    let enc_us = start.elapsed().as_micros() as f64 / n as f64;

    println!();
    println!("=== Timing ===");
    println!("Deterministic:    {:6.2} µs", det_us);
    println!("Production:       {:6.2} µs  (nonce pool)", prod_us);
    println!("  k*G:            {:6.2} µs  (comb table)", kg_us);
    println!("  Hash:           {:6.2} µs  (Poseidon2)", hash_us);
    println!("  Encode:         {:6.2} µs  (Fp5 inverse)", enc_us);
    println!();
    println!("Go signing:       {:6.2} µs", v["sign_time_us"].as_f64().unwrap());
    println!("Speedup:          {:6.1}x", v["sign_time_us"].as_f64().unwrap() / prod_us);
    println!("AVX2:             {}", lighter_rs::has_avx2());
    println!("AVX-512:          {}", lighter_rs::has_avx512f());

    // Batch benchmark
    let field_set: Vec<_> = vec![
        gf!(chain_id), gf!(14),
        gf!(v["nonce"].as_i64().unwrap()), gf!(v["expired_at"].as_i64().unwrap()),
        gf!(v["account_index"].as_i64().unwrap()), gf!(v["api_key_index"].as_u64().unwrap()),
        gf!(v["market_index"].as_i64().unwrap()), gf!(v["client_order_index"].as_i64().unwrap()),
        gf!(v["base_amount"].as_i64().unwrap()), gf!(v["price"].as_u64().unwrap()),
        gf!(v["is_ask"].as_u64().unwrap()), gf!(v["order_type"].as_u64().unwrap()),
        gf!(v["time_in_force"].as_u64().unwrap()), gf!(v["reduce_only"].as_u64().unwrap()),
        gf!(v["trigger_price"].as_u64().unwrap()), gf!(v["order_expiry"].as_i64().unwrap()),
    ];
    let field_sets: Vec<_> = (0..1000).map(|_| field_set.clone()).collect();
    let hashes: Vec<_> = field_sets.iter().map(|fs| poseidon2::hash_to_quintic_extension(fs)).collect();
    let (_, pm) = batch::batch_sign_parallel(&hashes, &sk);
    let (_, sm) = batch::batch_sign_seq(&hashes, &sk);
    println!();
    println!("Batch (1000): seq {:.2} µs/sig  par {:.2} µs/sig  {:.1}x",
        sm.us_per_sig, pm.us_per_sig, sm.total_us / pm.total_us);
}
