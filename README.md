# lighter-rs

Native Rust cryptographic signer for the Lighter DEX protocol. Implements Poseidon2
hashing and Schnorr signatures over the ECgFP5 elliptic curve — producing signatures
that are byte-for-byte identical to the official Go implementation.

This is a **crypto library**, not an SDK. It handles key management, transaction hashing,
and signature generation. It does not connect to the Lighter API, manage nonces, or
submit transactions — you bring your own HTTP client for that.

Built against the Go reference at [`elliottech/poseidon_crypto`](https://github.com/elliottech/poseidon_crypto) (v0.0.15).

## Performance

Signing a CreateOrder (hash 16 fields + Schnorr sign). Single-threaded on a Ryzen 9950X3D.

| Metric | Time |
|--------|------|
| Sign | **22 μs** |
| k·G (comb table) | 19 μs |
| Poseidon2 hash | 1.7 μs |
| Batch 1000 (Rayon) | **574k sigs/sec** |

Roughly 10× faster than the Go reference. AVX2 and AVX-512 are supported but the
bottleneck is Fp5 multiplication, SIMD helps primarily with batch throughput via
multi-threading, not single-sign latency.

## Getting started

Add to your `Cargo.toml`:

```toml
[dependencies]
lighter-rs = { git = "https://github.com/yeule0/lighter-rs" }
```

Generate a key and sign a message:

```rust
use lighter_rs::signer::KeyManager;

let km = KeyManager::generate();
let pk = km.public_key_bytes();        // 40 bytes

// The message must be a Poseidon2 hash of your transaction fields.
// See the example scripts for how to build this hash per tx type.
let hash: [u8; 40] = todo!("hash your tx fields with Poseidon2");
let sig: [u8; 80] = km.sign(&hash)?;

// Batch sign with Rayon parallelism:
let sigs = km.batch_sign(&[msg1, msg2, msg3]);
```

Self-contained example scripts in `src/bin/` sign and submit to testnet:

```bash
cargo run --release --bin create_order
cargo run --release --bin cancel_order
cargo run --release --bin batch_orders
```

Replace the four `const` values at the top of each script with your own.

## What's inside

The library is organised in layers, each building on the one below it:

```
GoldilocksField          GF(p), p = 2^64 − 2^32 + 1
  ├─ AVX2 (4-way)        YMM registers, unsigned compare via MSB flip
  └─ AVX-512 (8-way)     ZMM registers, mask register branch elimination

Fp5                     Quintic extension, x^5 = 3
  └─ AVX2 add/sub        First 4 components vectorised

Poseidon2               Sponge hash, WIDTH=12, RATE=8, 30 rounds
  ├─ External linear     u128 intermediates to avoid overflow
  └─ Internal linear     Diagonal matrix + accumulated sum

ECgFp5Scalar            319-bit prime order, 5-limb Montgomery
  └─ MontyMul            CIOS algorithm, zero heap allocations

ECgFP5 Point            Projective coords (x,z,u,t), complete formulas
  ├─ Comb table (W=8)    256 precomputed affine points, 40 columns
  └─ BatchToAffine       Montgomery batch inversion

Schnorr                  Sign + verify, thread-local nonce cache

KeyManager               Public API: sign, batch_sign, auth tokens

Tx types                 18 L2 transaction types + L2TxAttributes
```

## Correctness

Every layer is cross-validated against the Go reference. The test suite generates
vectors from `poseidon_crypto` and checks that our output matches byte-for-byte.

```
107 unit tests
 4 Go vector suites (Fp5, Poseidon2, curve, Schnorr)
 1 live cross-validation (CreateOrder hash + signature against Go)
```

A `cross_validate` binary compares our CreateOrder signature directly against Go's
output from `SchnorrSignHashedMessage2` — same private key, same nonce, same result.

## Transaction types

All 18 L2 transaction types from the protocol are implemented. Each has `Hash(chain_id)`
and `Validate()` methods that match the Go reference.

| Type | ID | Type | ID |
|------|----|------|----|
| CreateOrder | 14 | CancelOrder | 15 |
| CancelAllOrders | 16 | ModifyOrder | 17 |
| Transfer | 12 | Withdraw | 13 |
| ChangePubKey | 8 | CreateSubAccount | 9 |
| CreatePublicPool | 10 | UpdatePublicPool | 11 |
| MintShares | 18 | BurnShares | 19 |
| StakeAssets | 35 | UnstakeAssets | 36 |
| UpdateLeverage | 20 | UpdateMargin | 29 |
| CreateGroupedOrders | 28 | ApproveIntegrator | 45 |

## License

[MIT](LICENSE)
