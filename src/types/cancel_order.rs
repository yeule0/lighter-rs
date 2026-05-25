use crate::field::goldilocks::GoldilocksField;
use crate::hash::poseidon2;
use crate::types::constants::*;
use crate::types::errors::TxError;
use crate::types::interface::TxInfo;
use crate::types::attributes::L2TxAttributes;

pub struct CancelOrderTx {
    pub account_index: i64,
    pub api_key_index: u8,
    pub market_index: i16,
    pub index: i64,
    pub expired_at: i64,
    pub nonce: i64,
    pub sig: Vec<u8>,
    pub signed_hash: String,
    pub attributes: L2TxAttributes,
}

impl TxInfo for CancelOrderTx {
    fn get_tx_type(&self) -> u8 { TX_TYPE_L2_CANCEL_ORDER }
    fn get_tx_hash(&self) -> Option<&str> { Some(&self.signed_hash) }

    fn validate(&self) -> Result<(), TxError> {
        self.attributes.validate()?;
        if self.account_index < MIN_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooLow); }
        if self.account_index > MAX_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooHigh); }
        if self.api_key_index < MIN_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooLow); }
        if self.api_key_index > MAX_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooHigh); }
        let is_spot = self.market_index >= MIN_SPOT_MARKET_INDEX && self.market_index <= MAX_SPOT_MARKET_INDEX;
        let is_perps = self.market_index >= MIN_PERPS_MARKET_INDEX && self.market_index <= MAX_PERPS_MARKET_INDEX;
        if !is_spot && !is_perps { return Err(TxError::InvalidMarketIndex); }
        if self.index < MIN_CLIENT_ORDER_INDEX { return Err(TxError::OrderIndexTooLow); }
        if self.index > MAX_ORDER_INDEX { return Err(TxError::OrderIndexTooHigh); }
        if self.nonce < MIN_NONCE { return Err(TxError::NonceTooLow); }
        if self.expired_at < 0 || self.expired_at > MAX_TIMESTAMP { return Err(TxError::ExpiredAtInvalid); }
        Ok(())
    }

    fn hash(&self, chain_id: u32) -> Result<Vec<u8>, TxError> {
        let mut elems = Vec::with_capacity(8);
        elems.push(GoldilocksField(chain_id as u64));
        elems.push(GoldilocksField(TX_TYPE_L2_CANCEL_ORDER as u64));
        elems.push(GoldilocksField(self.nonce as u64));
        elems.push(GoldilocksField(self.expired_at as u64));
        elems.push(GoldilocksField(self.account_index as u64));
        elems.push(GoldilocksField(self.api_key_index as u64));
        elems.push(GoldilocksField(self.market_index as u64));
        elems.push(GoldilocksField(self.index as u64));
        let tx_hash = poseidon2::hash_to_quintic_extension(&elems);
        self.attributes.hash_and_aggregate(&tx_hash)
    }
}
