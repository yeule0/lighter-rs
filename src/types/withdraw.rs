use crate::field::goldilocks::GoldilocksField;
use crate::hash::poseidon2;
use crate::types::constants::*;
use crate::types::errors::TxError;
use crate::types::interface::TxInfo;
use crate::types::attributes::L2TxAttributes;

pub struct WithdrawTx {
    pub from_account_index: i64,
    pub api_key_index: u8,
    pub asset_index: i16,
    pub route_type: u8,
    pub amount: u64,
    pub expired_at: i64,
    pub nonce: i64,
    pub sig: Vec<u8>,
    pub signed_hash: String,
    pub attributes: L2TxAttributes,
}

impl TxInfo for WithdrawTx {
    fn get_tx_type(&self) -> u8 { TX_TYPE_L2_WITHDRAW }
    fn get_tx_hash(&self) -> Option<&str> { Some(&self.signed_hash) }

    fn validate(&self) -> Result<(), TxError> {
        self.attributes.validate()?;
        if self.from_account_index < MIN_ACCOUNT_INDEX { return Err(TxError::FromAccountIndexTooLow); }
        if self.from_account_index > MAX_ACCOUNT_INDEX { return Err(TxError::FromAccountIndexTooHigh); }
        if self.api_key_index < MIN_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooLow); }
        if self.api_key_index > MAX_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooHigh); }
        if self.asset_index < MIN_ASSET_INDEX { return Err(TxError::AssetIndexTooLow); }
        if self.asset_index > MAX_ASSET_INDEX { return Err(TxError::AssetIndexTooHigh); }
        if self.amount < MIN_WITHDRAWAL_AMOUNT { return Err(TxError::WithdrawalAmountTooLow); }
        if self.amount > MAX_WITHDRAWAL_AMOUNT { return Err(TxError::WithdrawalAmountTooHigh); }
        if self.nonce < MIN_NONCE { return Err(TxError::NonceTooLow); }
        if self.expired_at < 0 || self.expired_at > MAX_TIMESTAMP { return Err(TxError::ExpiredAtInvalid); }
        Ok(())
    }

    fn hash(&self, chain_id: u32) -> Result<Vec<u8>, TxError> {
        let mut elems = Vec::with_capacity(10);
        elems.push(GoldilocksField(chain_id as u64));
        elems.push(GoldilocksField(TX_TYPE_L2_WITHDRAW as u64));
        elems.push(GoldilocksField(self.nonce as u64));
        elems.push(GoldilocksField(self.expired_at as u64));
        elems.push(GoldilocksField(self.from_account_index as u64));
        elems.push(GoldilocksField(self.api_key_index as u64));
        elems.push(GoldilocksField(self.asset_index as u64));
        elems.push(GoldilocksField(self.route_type as u64));
        elems.push(GoldilocksField(self.amount & 0xFFFFFFFF));
        elems.push(GoldilocksField(self.amount >> 32));
        let tx_hash = poseidon2::hash_to_quintic_extension(&elems);
        self.attributes.hash_and_aggregate(&tx_hash)
    }
}
