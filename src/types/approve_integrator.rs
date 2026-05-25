use crate::field::goldilocks::GoldilocksField;
use crate::hash::poseidon2;
use crate::types::constants::*;
use crate::types::errors::TxError;
use crate::types::interface::TxInfo;
use crate::types::attributes::L2TxAttributes;

pub struct ApproveIntegratorTx {
    pub account_index: i64,
    pub api_key_index: u8,
    pub integrator_account_index: i64,
    pub max_perps_taker_fee: u32,
    pub max_perps_maker_fee: u32,
    pub max_spot_taker_fee: u32,
    pub max_spot_maker_fee: u32,
    pub approval_expiry: i64,
    pub expired_at: i64,
    pub nonce: i64,
    pub sig: Vec<u8>,
    pub signed_hash: String,
    pub attributes: L2TxAttributes,
}

impl TxInfo for ApproveIntegratorTx {
    fn get_tx_type(&self) -> u8 { TX_TYPE_L2_APPROVE_INTEGRATOR }
    fn get_tx_hash(&self) -> Option<&str> { Some(&self.signed_hash) }

    fn validate(&self) -> Result<(), TxError> {
        self.attributes.validate()?;
        if self.account_index < MIN_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooLow); }
        if self.account_index > MAX_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooHigh); }
        if self.api_key_index < MIN_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooLow); }
        if self.api_key_index > MAX_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooHigh); }
        if self.integrator_account_index < MIN_ACCOUNT_INDEX { return Err(TxError::IntegratorAccountIndexTooLow); }
        if self.integrator_account_index > MAX_ACCOUNT_INDEX { return Err(TxError::IntegratorAccountIndexTooHigh); }
        let perps_fees_ok = (self.max_perps_taker_fee as i64) <= FEE_TICK && (self.max_perps_maker_fee as i64) <= FEE_TICK;
        let spot_fees_ok = (self.max_spot_taker_fee as i64) <= FEE_TICK && (self.max_spot_maker_fee as i64) <= FEE_TICK;
        if !perps_fees_ok || !spot_fees_ok { return Err(TxError::FeeTooHigh); }
        let is_revoking = self.max_perps_taker_fee == 0 && self.max_perps_maker_fee == 0 && self.max_spot_taker_fee == 0 && self.max_spot_maker_fee == 0;
        if is_revoking != (self.approval_expiry == 0) { return Err(TxError::ApprovalExpiryZeroOnRevocation); }
        if self.approval_expiry < 0 || self.approval_expiry > MAX_TIMESTAMP { return Err(TxError::ApprovalExpiryInvalid); }
        if self.nonce < MIN_NONCE { return Err(TxError::NonceTooLow); }
        if self.expired_at < 0 || self.expired_at > MAX_TIMESTAMP { return Err(TxError::ExpiredAtInvalid); }
        Ok(())
    }

    fn hash(&self, chain_id: u32) -> Result<Vec<u8>, TxError> {
        let mut elems = Vec::with_capacity(12);
        elems.push(GoldilocksField(chain_id as u64));
        elems.push(GoldilocksField(TX_TYPE_L2_APPROVE_INTEGRATOR as u64));
        elems.push(GoldilocksField(self.nonce as u64));
        elems.push(GoldilocksField(self.expired_at as u64));
        elems.push(GoldilocksField(self.account_index as u64));
        elems.push(GoldilocksField(self.api_key_index as u64));
        elems.push(GoldilocksField(self.integrator_account_index as u64));
        elems.push(GoldilocksField(self.max_perps_taker_fee as u64));
        elems.push(GoldilocksField(self.max_perps_maker_fee as u64));
        elems.push(GoldilocksField(self.max_spot_taker_fee as u64));
        elems.push(GoldilocksField(self.max_spot_maker_fee as u64));
        elems.push(GoldilocksField(self.approval_expiry as u64));
        let tx_hash = poseidon2::hash_to_quintic_extension(&elems);
        self.attributes.hash_and_aggregate(&tx_hash)
    }
}
