use crate::field::goldilocks::GoldilocksField;
use crate::hash::poseidon2;
use crate::types::constants::*;
use crate::types::errors::TxError;
use crate::types::interface::{OrderInfo, TxInfo};
use crate::types::attributes::L2TxAttributes;

pub struct CreateGroupedOrdersTx {
    pub account_index: i64,
    pub api_key_index: u8,
    pub grouping_type: u8,
    pub orders: Vec<OrderInfo>,
    pub expired_at: i64,
    pub nonce: i64,
    pub sig: Vec<u8>,
    pub signed_hash: String,
    pub attributes: L2TxAttributes,
}

impl TxInfo for CreateGroupedOrdersTx {
    fn get_tx_type(&self) -> u8 { TX_TYPE_L2_CREATE_GROUPED_ORDERS }
    fn get_tx_hash(&self) -> Option<&str> { Some(&self.signed_hash) }

    fn validate(&self) -> Result<(), TxError> {
        self.attributes.validate()?;
        if self.account_index < MIN_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooLow); }
        if self.account_index > MAX_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooHigh); }
        if self.api_key_index < MIN_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooLow); }
        if self.api_key_index > MAX_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooHigh); }
        if self.orders.is_empty() || self.orders.len() as i64 > MAX_GROUPED_ORDER_COUNT { return Err(TxError::OrderGroupSizeInvalid); }
        if self.nonce < MIN_NONCE { return Err(TxError::NonceTooLow); }
        if self.expired_at < 0 || self.expired_at > MAX_TIMESTAMP { return Err(TxError::ExpiredAtInvalid); }
        Ok(())
    }

    fn hash(&self, chain_id: u32) -> Result<Vec<u8>, TxError> {
        let mut elems = Vec::with_capacity(11);
        elems.push(GoldilocksField(chain_id as u64));
        elems.push(GoldilocksField(TX_TYPE_L2_CREATE_GROUPED_ORDERS as u64));
        elems.push(GoldilocksField(self.nonce as u64));
        elems.push(GoldilocksField(self.expired_at as u64));
        elems.push(GoldilocksField(self.account_index as u64));
        elems.push(GoldilocksField(self.api_key_index as u64));
        elems.push(GoldilocksField(self.grouping_type as u64));

        let mut aggregated = poseidon2::empty_hash_out();
        for (idx, order) in self.orders.iter().enumerate() {
            let order_hash = poseidon2::hash_no_pad(&[
                GoldilocksField(order.market_index as u64),
                GoldilocksField(order.client_order_index as u64),
                GoldilocksField(order.base_amount as u64),
                GoldilocksField(order.price as u64),
                GoldilocksField(order.is_ask as u64),
                GoldilocksField(order.order_type as u64),
                GoldilocksField(order.time_in_force as u64),
                GoldilocksField(order.reduce_only as u64),
                GoldilocksField(order.trigger_price as u64),
                GoldilocksField(order.order_expiry as u64),
            ]);
            if idx == 0 {
                aggregated = order_hash;
            } else {
                aggregated = poseidon2::hash_n_to_one(&[aggregated, order_hash]);
            }
        }
        elems.extend_from_slice(&aggregated);

        let tx_hash = poseidon2::hash_to_quintic_extension(&elems);
        self.attributes.hash_and_aggregate(&tx_hash)
    }
}
