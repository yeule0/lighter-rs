//! Common utilities shared by the Lighter example scripts.
#![allow(dead_code, clippy::too_many_arguments)]

use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ── Config ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub chain_id: u32,
    pub account_index: i64,
    pub api_key_index: u8,
    pub private_key_hex: String,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let data = std::fs::read_to_string(path).map_err(|e| format!("read config: {e}"))?;
        serde_json::from_str(&data).map_err(|e| format!("parse config: {e}"))
    }
}

// ── API Client ────────────────────────────────────────────────────

pub struct ApiClient {
    pub base_url: String,
    pub chain_id: u32,
}

impl ApiClient {
    pub fn new(base_url: &str, chain_id: u32) -> Self {
        Self { base_url: base_url.trim_end_matches('/').to_string(), chain_id }
    }

    pub fn next_nonce(&self, account_index: i64, api_key_index: u8) -> Result<i64, String> {
        #[derive(Deserialize)]
        struct R { nonce: Option<i64> }
        let r: R = ureq::get(&format!(
            "{}/api/v1/nextNonce?account_index={}&api_key_index={}",
            self.base_url, account_index, api_key_index
        ))
        .call().map_err(|e| format!("nonce HTTP: {e}"))?
        .into_json().map_err(|e| format!("nonce parse: {e}"))?;
        r.nonce.ok_or_else(|| "no nonce in response".to_string())
    }

    pub fn submit(&self, tx_type: u8, tx_info: &str) -> Result<String, String> {
        #[derive(Deserialize)]
        struct R { tx_hash: Option<String>, message: Option<String>, code: Option<u16> }
        let resp = ureq::post(&format!("{}/api/v1/sendTx", self.base_url))
            .send_form(&[("tx_type", &tx_type.to_string()), ("tx_info", tx_info)])
            .map_err(|e| {
                match e {
                    ureq::Error::Status(code, r) => {
                        let body = r.into_string().unwrap_or_default();
                        format!("HTTP {}: {}", code, body)
                    }
                    _ => format!("HTTP error: {e}"),
                }
            })?;
        let r: R = resp.into_json().map_err(|e| format!("parse: {e}"))?;
        r.tx_hash.ok_or_else(|| r.message.unwrap_or_else(|| format!("code {}", r.code.unwrap_or(0))))
    }
}

// ── Signer ────────────────────────────────────────────────────────

pub struct Signer {
    km: lighter_rs::signer::KeyManager,
}

impl Signer {
    pub fn new(hex_key: &str) -> Result<Self, String> {
        lighter_rs::signer::KeyManager::from_hex(hex_key).map(|km| Self { km })
    }
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.km.public_key_bytes())
    }
    /// Sign a 40-byte hash, return Go-compatible base64-encoded signature.
    pub fn sign_base64(&self, hash: &[u8; 40]) -> Result<String, String> {
        let sig = self.km.sign(hash).map_err(|e| format!("sign: {e}"))?;
        Ok(BASE64.encode(sig))
    }
}

// ── Hashing ───────────────────────────────────────────────────────

use lighter_rs::field::goldilocks::GoldilocksField;
use lighter_rs::hash::poseidon2;

pub fn hash_create_order(
    chain_id: u32, account_index: i64, api_key_index: u8,
    market_index: i16, client_order_index: i64, base_amount: i64,
    price: u32, is_ask: u8, order_type: u8, time_in_force: u8,
    reduce_only: u8, trigger_price: u32, order_expiry: i64,
    nonce: i64, expired_at: i64,
) -> [u8; 40] {
    let elems = vec![
        GoldilocksField(chain_id as u64), GoldilocksField(14u64),
        GoldilocksField(nonce as u64), GoldilocksField(expired_at as u64),
        GoldilocksField(account_index as u64), GoldilocksField(api_key_index as u64),
        GoldilocksField(market_index as u64), GoldilocksField(client_order_index as u64),
        GoldilocksField(base_amount as u64), GoldilocksField(price as u64),
        GoldilocksField(is_ask as u64), GoldilocksField(order_type as u64),
        GoldilocksField(time_in_force as u64), GoldilocksField(reduce_only as u64),
        GoldilocksField(trigger_price as u64), GoldilocksField(order_expiry as u64),
    ];
    poseidon2::hash_to_quintic_extension(&elems).to_bytes_le()
}

pub fn hash_cancel_order(
    chain_id: u32, account_index: i64, api_key_index: u8,
    market_index: i16, order_index: i64, nonce: i64, expired_at: i64,
) -> [u8; 40] {
    let elems = vec![
        GoldilocksField(chain_id as u64), GoldilocksField(15u64),
        GoldilocksField(nonce as u64), GoldilocksField(expired_at as u64),
        GoldilocksField(account_index as u64), GoldilocksField(api_key_index as u64),
        GoldilocksField(market_index as u64), GoldilocksField(order_index as u64),
    ];
    poseidon2::hash_to_quintic_extension(&elems).to_bytes_le()
}

pub fn hash_transfer(
    chain_id: u32, from_index: i64, api_key_index: u8, to_index: i64,
    asset_index: i16, from_route: u8, to_route: u8,
    amount: i64, usdc_fee: i64, nonce: i64, expired_at: i64,
) -> [u8; 40] {
    let elems = vec![
        GoldilocksField(chain_id as u64), GoldilocksField(12u64),
        GoldilocksField(nonce as u64), GoldilocksField(expired_at as u64),
        GoldilocksField(from_index as u64), GoldilocksField(api_key_index as u64),
        GoldilocksField(to_index as u64), GoldilocksField(asset_index as u64),
        GoldilocksField(from_route as u64), GoldilocksField(to_route as u64),
        GoldilocksField((amount as u64) & 0xFFFFFFFF), GoldilocksField((amount as u64) >> 32),
        GoldilocksField((usdc_fee as u64) & 0xFFFFFFFF), GoldilocksField((usdc_fee as u64) >> 32),
    ];
    poseidon2::hash_to_quintic_extension(&elems).to_bytes_le()
}

pub fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as i64
}

// ── Go-compatible JSON types (PascalCase keys, base64 Sig) ────────

#[derive(Serialize)]
pub struct CreateOrderTx {
    #[serde(rename = "AccountIndex")]     pub account_index: i64,
    #[serde(rename = "ApiKeyIndex")]      pub api_key_index: u8,
    #[serde(rename = "MarketIndex")]      pub market_index: i16,
    #[serde(rename = "ClientOrderIndex")] pub client_order_index: i64,
    #[serde(rename = "BaseAmount")]       pub base_amount: i64,
    #[serde(rename = "Price")]            pub price: u32,
    #[serde(rename = "IsAsk")]            pub is_ask: u8,
    #[serde(rename = "Type")]             pub order_type: u8,
    #[serde(rename = "TimeInForce")]      pub time_in_force: u8,
    #[serde(rename = "ReduceOnly")]       pub reduce_only: u8,
    #[serde(rename = "TriggerPrice")]     pub trigger_price: u32,
    #[serde(rename = "OrderExpiry")]      pub order_expiry: i64,
    #[serde(rename = "ExpiredAt")]        pub expired_at: i64,
    #[serde(rename = "Nonce")]            pub nonce: i64,
    #[serde(rename = "Sig")]              pub sig: String,
}

#[derive(Serialize)]
pub struct CancelOrderTx {
    #[serde(rename = "AccountIndex")] pub account_index: i64,
    #[serde(rename = "ApiKeyIndex")]  pub api_key_index: u8,
    #[serde(rename = "MarketIndex")]  pub market_index: i16,
    #[serde(rename = "Index")]        pub index: i64,
    #[serde(rename = "ExpiredAt")]    pub expired_at: i64,
    #[serde(rename = "Nonce")]        pub nonce: i64,
    #[serde(rename = "Sig")]          pub sig: String,
}

#[derive(Serialize)]
pub struct TransferTx {
    #[serde(rename = "FromAccountIndex")] pub from_account_index: i64,
    #[serde(rename = "ApiKeyIndex")]      pub api_key_index: u8,
    #[serde(rename = "ToAccountIndex")]   pub to_account_index: i64,
    #[serde(rename = "AssetIndex")]       pub asset_index: i16,
    #[serde(rename = "FromRouteType")]    pub from_route_type: u8,
    #[serde(rename = "ToRouteType")]      pub to_route_type: u8,
    #[serde(rename = "Amount")]           pub amount: i64,
    #[serde(rename = "USDCFee")]          pub usdc_fee: i64,
    #[serde(rename = "Memo")]             pub memo: [u8; 32],
    #[serde(rename = "ExpiredAt")]        pub expired_at: i64,
    #[serde(rename = "Nonce")]            pub nonce: i64,
    #[serde(rename = "Sig")]              pub sig: String,
}
