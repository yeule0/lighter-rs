use crate::types::errors::TxError;

/// OrderInfo holds the common order fields shared across order-related txs.
#[derive(Debug, Clone, Default)]
pub struct OrderInfo {
    pub market_index: i16,
    pub client_order_index: i64,
    pub base_amount: i64,
    pub price: u32,
    pub is_ask: u8,
    pub order_type: u8,
    pub time_in_force: u8,
    pub reduce_only: u8,
    pub trigger_price: u32,
    pub order_expiry: i64,
}

/// Every transaction must implement TxInfo.
pub trait TxInfo {
    fn get_tx_type(&self) -> u8;
    fn get_tx_hash(&self) -> Option<&str>;
    fn validate(&self) -> Result<(), TxError>;
    fn hash(&self, chain_id: u32) -> Result<Vec<u8>, TxError>;
}
