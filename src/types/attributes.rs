use crate::field::goldilocks::GoldilocksField;
use crate::field::quintic::Fp5;
use crate::hash::poseidon2;
use crate::types::errors::TxError;
use crate::types::constants::{MAX_ACCOUNT_INDEX, FEE_TICK, NB_ATTRIBUTES_PER_TX, NIL_INTEGRATOR_INDEX};

pub const ATTR_TYPE_INTEGRATOR_ACCOUNT_INDEX: u8 = 1;
pub const ATTR_TYPE_INTEGRATOR_TAKER_FEE: u8 = 2;
pub const ATTR_TYPE_INTEGRATOR_MAKER_FEE: u8 = 3;
pub const ATTR_TYPE_SKIP_TX_NONCE: u8 = 4;

/// L2TxAttributes: up to 4 optional attributes attached to any transaction.
#[derive(Debug, Clone, Default)]
pub struct L2TxAttributes {
    pub integrator_account_index: i64,
    pub integrator_taker_fee: u32,
    pub integrator_maker_fee: u32,
    pub skip_tx_nonce: bool,
    count: u8,
}

impl L2TxAttributes {
    pub fn new() -> Self { Self::default() }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn set_integrator_account_index(&mut self, idx: i64) -> Result<(), TxError> {
        if !(0..=MAX_ACCOUNT_INDEX).contains(&idx) { return Err(TxError::AttributeValueOutOfRange); }
        if self.count >= NB_ATTRIBUTES_PER_TX as u8 { return Err(TxError::TooManyAttributes); }
        self.integrator_account_index = idx;
        self.count += 1;
        Ok(())
    }

    pub fn set_integrator_taker_fee(&mut self, fee: u32) -> Result<(), TxError> {
        if fee as i64 > FEE_TICK { return Err(TxError::AttributeValueOutOfRange); }
        if self.count >= NB_ATTRIBUTES_PER_TX as u8 { return Err(TxError::TooManyAttributes); }
        self.integrator_taker_fee = fee;
        self.count += 1;
        Ok(())
    }

    pub fn set_integrator_maker_fee(&mut self, fee: u32) -> Result<(), TxError> {
        if fee as i64 > FEE_TICK { return Err(TxError::AttributeValueOutOfRange); }
        if self.count >= NB_ATTRIBUTES_PER_TX as u8 { return Err(TxError::TooManyAttributes); }
        self.integrator_maker_fee = fee;
        self.count += 1;
        Ok(())
    }

    pub fn set_skip_tx_nonce(&mut self) -> Result<(), TxError> {
        if self.count >= NB_ATTRIBUTES_PER_TX as u8 { return Err(TxError::TooManyAttributes); }
        self.skip_tx_nonce = true;
        self.count += 1;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), TxError> {
        if self.is_empty() { return Ok(()); }
        let has_fees = self.integrator_taker_fee != 0 || self.integrator_maker_fee != 0;
        if has_fees && self.integrator_account_index == NIL_INTEGRATOR_INDEX {
            return Err(TxError::IntegratorAccountIndexRequired);
        }
        Ok(())
    }

    /// Hash attributes into Fp5, then aggregate with the tx hash.
    pub fn hash_and_aggregate(&self, tx_hash: &Fp5) -> Result<Vec<u8>, TxError> {
        if self.is_empty() {
            return Ok(tx_hash.to_bytes_le().to_vec());
        }

        let mut elems = Vec::with_capacity(NB_ATTRIBUTES_PER_TX * 2);
        // Collect non-zero attributes sorted by type
        let mut attrs: Vec<(u8, u64)> = Vec::new();
        if self.integrator_account_index != NIL_INTEGRATOR_INDEX {
            attrs.push((ATTR_TYPE_INTEGRATOR_ACCOUNT_INDEX, self.integrator_account_index as u64));
        }
        if self.integrator_taker_fee != 0 {
            attrs.push((ATTR_TYPE_INTEGRATOR_TAKER_FEE, self.integrator_taker_fee as u64));
        }
        if self.integrator_maker_fee != 0 {
            attrs.push((ATTR_TYPE_INTEGRATOR_MAKER_FEE, self.integrator_maker_fee as u64));
        }
        if self.skip_tx_nonce {
            attrs.push((ATTR_TYPE_SKIP_TX_NONCE, 1));
        }
        attrs.sort_by_key(|a| a.0);

        for (typ, val) in &attrs {
            elems.push(GoldilocksField(*typ as u64));
            elems.push(GoldilocksField(*val));
        }
        // Pad to NB_ATTRIBUTES_PER_TX * 2 with zeros
        while elems.len() < NB_ATTRIBUTES_PER_TX * 2 {
            elems.push(GoldilocksField::ZERO);
        }

        let attr_hash = poseidon2::hash_to_quintic_extension(&elems);
        // Aggregate: HashToQuinticExtension(tx_hash[..5] + attr_hash[..5])
        let combined: Vec<_> = tx_hash.0.iter().chain(attr_hash.0.iter()).copied().collect();
        Ok(poseidon2::hash_to_quintic_extension(&combined).to_bytes_le().to_vec())
    }
}
