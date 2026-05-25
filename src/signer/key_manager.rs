use crate::curve::scalar::Scalar;
use crate::field::quintic::Fp5;
use crate::hash::poseidon2;
use crate::signature::schnorr;
use zeroize::Zeroize;

pub struct KeyManager {
    secret_key: Scalar,
}

impl Drop for KeyManager {
    fn drop(&mut self) {
        self.secret_key.0.zeroize();
    }
}

impl KeyManager {
    /// Create from 40-byte little-endian private key bytes.
    pub fn from_bytes(bytes: &[u8; 40]) -> Self {
        Self {
            secret_key: Scalar::from_bytes_le(bytes),
        }
    }

    /// Create from hex-encoded private key (80 chars, optional 0x prefix).
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let s = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        let bytes = hex::decode(s).map_err(|e| format!("hex decode: {e}"))?;
        if bytes.len() != 40 {
            return Err(format!("expected 40 bytes, got {}", bytes.len()));
        }
        let arr: [u8; 40] = bytes.try_into().unwrap();
        Ok(Self::from_bytes(&arr))
    }

    /// Generate a random key.
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut bytes = [0u8; 64];
        rand::thread_rng().fill_bytes(&mut bytes);
        let arr: [u8; 40] = bytes[..40].try_into().unwrap();
        Self::from_bytes(&arr)
    }

    /// Return the public key as 40-byte Fp5 encoding.
    pub fn public_key_bytes(&self) -> [u8; 40] {
        schnorr::public_key_from_secret(&self.secret_key).to_bytes_le()
    }

    /// Return the private key bytes.
    pub fn private_key_bytes(&self) -> [u8; 40] {
        self.secret_key.to_bytes_le()
    }

    /// Sign a 40-byte hashed message (Fp5 element in LE bytes).
    /// Returns 80-byte signature.
    pub fn sign(&self, hashed_message: &[u8; 40]) -> Result<[u8; 80], String> {
        let msg = Fp5::from_bytes_le(hashed_message)
            .map_err(|e| format!("invalid hashed message: {e}"))?;
        let sig = schnorr::sign_hashed_message(&msg, &self.secret_key);
        Ok(sig.to_bytes())
    }

    /// Sign a message that's already in Fp5 form.
    pub fn sign_fp5(&self, msg: &Fp5) -> [u8; 80] {
        schnorr::sign_hashed_message(msg, &self.secret_key).to_bytes()
    }

    /// Batch-sign multiple pre-hashed Fp5 messages using Rayon parallel threads.
    /// Returns one 80-byte signature per message. Thread-local nonces — zero contention.
    pub fn batch_sign(&self, messages: &[Fp5]) -> Vec<[u8; 80]> {
        use rayon::prelude::*;
        messages
            .par_iter()
            .map(|msg| schnorr::sign_hashed_message(msg, &self.secret_key).to_bytes())
            .collect()
    }

    /// Batch-sign from raw 40-byte LE messages.
    pub fn batch_sign_bytes(&self, messages: &[[u8; 40]]) -> Result<Vec<[u8; 80]>, String> {
        let fps: Result<Vec<Fp5>, _> = messages
            .iter()
            .map(|m| Fp5::from_bytes_le(m))
            .collect();
        Ok(self.batch_sign(&fps?))
    }

    /// Create an auth token: "deadlineUnix:accountIndex:apiKeyIndex"
    /// The token is hashed via Poseidon2 and signed.
    pub fn create_auth_token(
        &self,
        deadline: i64,
        account_index: i64,
        api_key_index: u8,
    ) -> Result<String, String> {
        let auth_data = format!("{}:{}:{}", deadline, account_index, api_key_index);
        let elements: Vec<_> = auth_data
            .bytes()
            .map(|b| crate::field::goldilocks::GoldilocksField(b as u64))
            .collect();
        let hash = poseidon2::hash_to_quintic_extension(&elements);
        let sig = self.sign_fp5(&hash);
        Ok(format!("{}:{}", auth_data, hex::encode(sig)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keygen_sign_verify() {
        let km = KeyManager::generate();
        let pk = km.public_key_bytes();
        let msg = [42u8; 40];
        let sig = km.sign(&msg).unwrap();
        // Re-derive from bytes — should produce a valid (but different due to nonce) signature
        let sk_bytes = km.private_key_bytes();
        let km2 = KeyManager::from_bytes(&sk_bytes);
        let sig2 = km2.sign(&msg).unwrap();
        // Both signatures should verify against the same pubkey
        let msg_fp5 = crate::field::quintic::Fp5::from_bytes_le(&msg).unwrap();
        let pk_fp5 = crate::field::quintic::Fp5::from_bytes_le(&pk).unwrap();
        let sig_parsed = crate::signature::schnorr::Signature::from_bytes(&sig).unwrap();
        let sig2_parsed = crate::signature::schnorr::Signature::from_bytes(&sig2).unwrap();
        assert!(crate::signature::schnorr::verify(&pk_fp5, &msg_fp5, &sig_parsed));
        assert!(crate::signature::schnorr::verify(&pk_fp5, &msg_fp5, &sig2_parsed));
    }

    #[test]
    fn batch_sign_all_verify() {
        let km = KeyManager::generate();
        let pk = km.public_key_bytes();
        let pk_fp5 = Fp5::from_bytes_le(&pk).unwrap();

        let msgs: Vec<Fp5> = (0..100)
            .map(|i| Fp5::from_u64_arr([i, i + 1, i + 2, i + 3, i + 4]))
            .collect();
        let sigs = km.batch_sign(&msgs);

        assert_eq!(sigs.len(), 100);
        for (msg, sig) in msgs.iter().zip(sigs.iter()) {
            let sig_parsed = schnorr::Signature::from_bytes(sig).unwrap();
            assert!(schnorr::verify(&pk_fp5, msg, &sig_parsed));
        }
    }
}
