use crate::field::goldilocks::GoldilocksField;
use crate::hash::poseidon2;
use crate::types::constants::*;
use crate::types::errors::TxError;
use crate::types::interface::{OrderInfo, TxInfo};
use crate::types::attributes::L2TxAttributes;

pub struct CreateOrderTx {
    pub account_index: i64,
    pub api_key_index: u8,
    pub order: OrderInfo,
    pub expired_at: i64,
    pub nonce: i64,
    pub sig: Vec<u8>,
    pub signed_hash: String,
    pub attributes: L2TxAttributes,
}

impl TxInfo for CreateOrderTx {
    fn get_tx_type(&self) -> u8 { TX_TYPE_L2_CREATE_ORDER }
    fn get_tx_hash(&self) -> Option<&str> { Some(&self.signed_hash) }

    fn validate(&self) -> Result<(), TxError> {
        self.attributes.validate()?;
        if self.account_index < MIN_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooLow); }
        if self.account_index > MAX_ACCOUNT_INDEX { return Err(TxError::AccountIndexTooHigh); }
        if self.api_key_index < MIN_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooLow); }
        if self.api_key_index > MAX_API_KEY_INDEX { return Err(TxError::ApiKeyIndexTooHigh); }
        let is_spot = self.order.market_index >= MIN_SPOT_MARKET_INDEX && self.order.market_index <= MAX_SPOT_MARKET_INDEX;
        let is_perps = self.order.market_index >= MIN_PERPS_MARKET_INDEX && self.order.market_index <= MAX_PERPS_MARKET_INDEX;
        if !is_spot && !is_perps { return Err(TxError::InvalidMarketIndex); }
        if self.order.client_order_index != NIL_CLIENT_ORDER_INDEX {
            if self.order.client_order_index < MIN_CLIENT_ORDER_INDEX { return Err(TxError::ClientOrderIndexTooLow); }
            if self.order.client_order_index > MAX_CLIENT_ORDER_INDEX { return Err(TxError::ClientOrderIndexTooHigh); }
        }
        if self.order.reduce_only != 1 && self.order.base_amount == NIL_ORDER_BASE_AMOUNT { return Err(TxError::BaseAmountTooLow); }
        if self.order.base_amount != NIL_ORDER_BASE_AMOUNT && self.order.base_amount < MIN_ORDER_BASE_AMOUNT { return Err(TxError::BaseAmountTooLow); }
        if self.order.base_amount > MAX_ORDER_BASE_AMOUNT { return Err(TxError::BaseAmountTooHigh); }
        if self.order.price < MIN_ORDER_PRICE { return Err(TxError::PriceTooLow); }
        if self.order.price > MAX_ORDER_PRICE { return Err(TxError::PriceTooHigh); }
        if self.order.is_ask != 0 && self.order.is_ask != 1 { return Err(TxError::IsAskInvalid); }
        let valid_tif = self.order.time_in_force == TIF_IMMEDIATE_OR_CANCEL || self.order.time_in_force == TIF_GOOD_TILL_TIME || self.order.time_in_force == TIF_POST_ONLY;
        if !valid_tif { return Err(TxError::OrderTimeInForceInvalid); }
        if (self.order.reduce_only != 0 && self.order.reduce_only != 1) || (is_spot && self.order.reduce_only == 1) { return Err(TxError::OrderReduceOnlyInvalid); }
        if (self.order.order_expiry < MIN_ORDER_EXPIRY || self.order.order_expiry > MAX_ORDER_EXPIRY) && self.order.order_expiry != NIL_ORDER_EXPIRY { return Err(TxError::OrderExpiryInvalid); }
        match self.order.order_type {
            ORDER_TYPE_MARKET => {
                if self.order.time_in_force != TIF_IMMEDIATE_OR_CANCEL { return Err(TxError::OrderTimeInForceInvalid); }
                if self.order.order_expiry != NIL_ORDER_EXPIRY { return Err(TxError::OrderExpiryInvalid); }
                if self.order.trigger_price != NIL_ORDER_TRIGGER_PRICE { return Err(TxError::OrderTriggerPriceInvalid); }
            }
            ORDER_TYPE_LIMIT => {
                if self.order.trigger_price != NIL_ORDER_TRIGGER_PRICE { return Err(TxError::OrderTriggerPriceInvalid); }
            }
            ORDER_TYPE_STOP_LOSS | ORDER_TYPE_TAKE_PROFIT => {
                if !is_perps { return Err(TxError::OrderTypeInvalid); }
                if self.order.time_in_force != TIF_IMMEDIATE_OR_CANCEL { return Err(TxError::OrderTimeInForceInvalid); }
                if self.order.trigger_price == NIL_ORDER_TRIGGER_PRICE { return Err(TxError::OrderTriggerPriceInvalid); }
                if self.order.order_expiry == NIL_ORDER_EXPIRY { return Err(TxError::OrderExpiryInvalid); }
            }
            ORDER_TYPE_STOP_LOSS_LIMIT | ORDER_TYPE_TAKE_PROFIT_LIMIT => {
                if !is_perps { return Err(TxError::OrderTypeInvalid); }
                if self.order.trigger_price == NIL_ORDER_TRIGGER_PRICE { return Err(TxError::OrderTriggerPriceInvalid); }
                if self.order.order_expiry == NIL_ORDER_EXPIRY { return Err(TxError::OrderExpiryInvalid); }
            }
            ORDER_TYPE_TWAP => {
                if self.order.time_in_force != TIF_GOOD_TILL_TIME { return Err(TxError::OrderTimeInForceInvalid); }
                if self.order.order_expiry == NIL_ORDER_EXPIRY { return Err(TxError::OrderExpiryInvalid); }
            }
            _ => return Err(TxError::OrderTypeInvalid),
        }
        if (self.order.trigger_price < MIN_ORDER_TRIGGER_PRICE || self.order.trigger_price > MAX_ORDER_TRIGGER_PRICE) && self.order.trigger_price != NIL_ORDER_TRIGGER_PRICE { return Err(TxError::OrderTriggerPriceInvalid); }
        if self.nonce < MIN_NONCE { return Err(TxError::NonceTooLow); }
        if self.expired_at < 0 || self.expired_at > MAX_TIMESTAMP { return Err(TxError::ExpiredAtInvalid); }
        Ok(())
    }

    fn hash(&self, chain_id: u32) -> Result<Vec<u8>, TxError> {
        let mut elems = Vec::with_capacity(16);
        elems.push(GoldilocksField(chain_id as u64));
        elems.push(GoldilocksField(TX_TYPE_L2_CREATE_ORDER as u64));
        elems.push(GoldilocksField(self.nonce as u64));
        elems.push(GoldilocksField(self.expired_at as u64));
        elems.push(GoldilocksField(self.account_index as u64));
        elems.push(GoldilocksField(self.api_key_index as u64));
        elems.push(GoldilocksField(self.order.market_index as u64));
        elems.push(GoldilocksField(self.order.client_order_index as u64));
        elems.push(GoldilocksField(self.order.base_amount as u64));
        elems.push(GoldilocksField(self.order.price as u64));
        elems.push(GoldilocksField(self.order.is_ask as u64));
        elems.push(GoldilocksField(self.order.order_type as u64));
        elems.push(GoldilocksField(self.order.time_in_force as u64));
        elems.push(GoldilocksField(self.order.reduce_only as u64));
        elems.push(GoldilocksField(self.order.trigger_price as u64));
        elems.push(GoldilocksField(self.order.order_expiry as u64));
        let tx_hash = poseidon2::hash_to_quintic_extension(&elems);
        self.attributes.hash_and_aggregate(&tx_hash)
    }
}
