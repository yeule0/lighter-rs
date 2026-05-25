use crate::field::goldilocks::GoldilocksField;
use crate::hash::poseidon2;
use crate::types::constants::*;
use crate::types::errors::TxError;
use crate::types::interface::TxInfo;
use crate::types::attributes::L2TxAttributes;

pub struct UpdatePublicPoolTx {
    pub account_index: i64,
    pub api_key_index: u8,
    pub pool_index: i64,
    pub status: u8,
    pub operator_fee: i64,
    pub min_operator_share_rate: u16,
    pub expired_at: i64,
    pub nonce: i64,
    pub sig: Vec<u8>,
    pub signed_hash: String,
    pub attributes: L2TxAttributes,
}

impl TxInfo for UpdatePublicPoolTx {
    fn get_tx_type(&self) -> u8 { TX_TYPE_L2_UPDATE_PUBLIC_POOL }
    fn get_tx_hash(&self) -> Option<&str> { Some(&self.signed_hash) }

    fn validate(&self) -> Result<(), TxError> {
        self.attributes.validate()?;
        if self.account_index < MIN_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooLow); }
        if self.account_index > MAX_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooHigh); }
        if self.api_key_index < MIN_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooLow); }
        if self.api_key_index > MAX_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooHigh); }
        if self.pool_index < MIN_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooLow); }
        if self.pool_index > MAX_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooHigh); }
        if self.status > 1 { return Err(TxError::PoolStatusInvalid); }
        if self.operator_fee <= 0 || self.operator_fee > FEE_TICK { return Err(TxError::PoolOperatorFeeInvalid); }
        if self.min_operator_share_rate == 0 { return Err(TxError::MinOperatorShareRateTooLow); }
        if self.min_operator_share_rate > SHARE_TICK { return Err(TxError::MinOperatorShareRateTooHigh); }
        if self.nonce < MIN_NONCE { return Err(TxError::NonceTooLow); }
        if self.expired_at < 0 || self.expired_at > MAX_TIMESTAMP { return Err(TxError::ExpiredAtInvalid); }
        Ok(())
    }

    fn hash(&self, chain_id: u32) -> Result<Vec<u8>, TxError> {
        let mut elems = Vec::with_capacity(10);
        elems.push(GoldilocksField(chain_id as u64));
        elems.push(GoldilocksField(TX_TYPE_L2_UPDATE_PUBLIC_POOL as u64));
        elems.push(GoldilocksField(self.nonce as u64));
        elems.push(GoldilocksField(self.expired_at as u64));
        elems.push(GoldilocksField(self.account_index as u64));
        elems.push(GoldilocksField(self.api_key_index as u64));
        elems.push(GoldilocksField(self.pool_index as u64));
        elems.push(GoldilocksField(self.status as u64));
        elems.push(GoldilocksField(self.operator_fee as u64));
        elems.push(GoldilocksField(self.min_operator_share_rate as u64));
        let tx_hash = poseidon2::hash_to_quintic_extension(&elems);
        self.attributes.hash_and_aggregate(&tx_hash)
    }
}
