use crate::field::goldilocks::GoldilocksField;
use crate::hash::poseidon2;
use crate::types::constants::*;
use crate::types::errors::TxError;
use crate::types::interface::TxInfo;
use crate::types::attributes::L2TxAttributes;

pub struct TransferTx {
    pub from_account_index: i64,
    pub api_key_index: u8,
    pub to_account_index: i64,
    pub asset_index: i16,
    pub from_route_type: u8,
    pub to_route_type: u8,
    pub amount: i64,
    pub usdc_fee: i64,
    pub memo: [u8; 32],
    pub expired_at: i64,
    pub nonce: i64,
    pub sig: Vec<u8>,
    pub signed_hash: String,
    pub attributes: L2TxAttributes,
}

impl TxInfo for TransferTx {
    fn get_tx_type(&self) -> u8 { TX_TYPE_L2_TRANSFER }
    fn get_tx_hash(&self) -> Option<&str> { Some(&self.signed_hash) }

    fn validate(&self) -> Result<(), TxError> {
        self.attributes.validate()?;
        if self.from_account_index < MIN_ACCOUNT_INDEX { return Err(TxError::FromAccountIndexTooLow); }
        if self.from_account_index > MAX_ACCOUNT_INDEX { return Err(TxError::FromAccountIndexTooHigh); }
        if self.api_key_index < MIN_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooLow); }
        if self.api_key_index > MAX_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooHigh); }
        if self.to_account_index < MIN_ACCOUNT_INDEX { return Err(TxError::ToAccountIndexTooLow); }
        if self.to_account_index > MAX_ACCOUNT_INDEX { return Err(TxError::ToAccountIndexTooHigh); }
        if self.asset_index < MIN_ASSET_INDEX { return Err(TxError::AssetIndexTooLow); }
        if self.asset_index > MAX_ASSET_INDEX { return Err(TxError::AssetIndexTooHigh); }
        if self.from_route_type != ASSET_ROUTE_PERPS && self.from_route_type != ASSET_ROUTE_SPOT { return Err(TxError::RouteTypeInvalid); }
        if self.to_route_type != ASSET_ROUTE_PERPS && self.to_route_type != ASSET_ROUTE_SPOT { return Err(TxError::RouteTypeInvalid); }
        if self.amount <= 0 { return Err(TxError::TransferAmountTooLow); }
        if self.amount > MAX_TRANSFER_AMOUNT { return Err(TxError::TransferAmountTooHigh); }
        if self.usdc_fee < 0 { return Err(TxError::TransferFeeNegative); }
        if self.usdc_fee > MAX_TRANSFER_AMOUNT { return Err(TxError::TransferFeeTooHigh); }
        if self.nonce < MIN_NONCE { return Err(TxError::NonceTooLow); }
        if self.expired_at < 0 || self.expired_at > MAX_TIMESTAMP { return Err(TxError::ExpiredAtInvalid); }
        Ok(())
    }

    fn hash(&self, chain_id: u32) -> Result<Vec<u8>, TxError> {
        let mut elems = Vec::with_capacity(14);
        elems.push(GoldilocksField(chain_id as u64));
        elems.push(GoldilocksField(TX_TYPE_L2_TRANSFER as u64));
        elems.push(GoldilocksField(self.nonce as u64));
        elems.push(GoldilocksField(self.expired_at as u64));
        elems.push(GoldilocksField(self.from_account_index as u64));
        elems.push(GoldilocksField(self.api_key_index as u64));
        elems.push(GoldilocksField(self.to_account_index as u64));
        elems.push(GoldilocksField(self.asset_index as u64));
        elems.push(GoldilocksField(self.from_route_type as u64));
        elems.push(GoldilocksField(self.to_route_type as u64));
        // amount split into lo/hi 32 bits
        elems.push(GoldilocksField((self.amount as u64) & 0xFFFFFFFF));
        elems.push(GoldilocksField((self.amount as u64) >> 32));
        // fee split
        elems.push(GoldilocksField((self.usdc_fee as u64) & 0xFFFFFFFF));
        elems.push(GoldilocksField((self.usdc_fee as u64) >> 32));
        let tx_hash = poseidon2::hash_to_quintic_extension(&elems);
        self.attributes.hash_and_aggregate(&tx_hash)
    }
}
