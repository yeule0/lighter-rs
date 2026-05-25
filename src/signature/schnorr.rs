use crate::field::quintic::Fp5;
use crate::curve::scalar::Scalar;
use crate::curve::point::Point;
use crate::curve::weierstrass::{WeierstrassPoint, mul_add2};
use crate::hash::poseidon2;
use std::cell::RefCell;

// Thread-local nonce cache. Each thread pre-generates nonces from its own
// `thread_rng()`, avoiding any shared state. This eliminates Mutex contention
// under parallel batch signing.
thread_local! {
    static NONCE_CACHE: RefCell<Vec<Scalar>> = RefCell::new(Vec::with_capacity(64));
}

fn pop_nonce() -> Scalar {
    NONCE_CACHE.with(|cache| {
        let mut c = cache.borrow_mut();
        if let Some(n) = c.pop() {
            return n;
        }
        // Refill from thread-local RNG — no contention
        use rand::RngCore;
        let mut rng = rand::thread_rng();
        for _ in 0..64 {
            let mut bytes = [0u8; 64];
            rng.fill_bytes(&mut bytes);
            let arr: [u8; 40] = bytes[..40].try_into().unwrap();
            c.push(Scalar::from_bytes_le(&arr));
        }
        c.pop().unwrap()
    })
}

/// Schnorr signature over ECgFP5: 80 bytes (40-byte s || 40-byte e).
#[derive(Debug, Clone, Copy)]
pub struct Signature {
    pub s: Scalar,
    pub e: Scalar,
}

impl Signature {
    pub const BYTE_LEN: usize = 80;

    pub fn to_bytes(&self) -> [u8; 80] {
        let mut out = [0u8; 80];
        out[..40].copy_from_slice(&self.s.to_bytes_le());
        out[40..].copy_from_slice(&self.e.to_bytes_le());
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != 80 {
            return Err("signature must be 80 bytes");
        }
        let s_arr: [u8; 40] = bytes[..40].try_into().unwrap();
        let e_arr: [u8; 40] = bytes[40..].try_into().unwrap();
        Ok(Self {
            s: Scalar::from_bytes_le(&s_arr),
            e: Scalar::from_bytes_le(&e_arr),
        })
    }

    pub fn is_canonical(&self) -> bool {
        self.s.is_canonical() && self.e.is_canonical()
    }
}

// ------------------------------------------------------------------
// Key generation
// ------------------------------------------------------------------

/// Derive public key from secret scalar: pk = G * sk, encoded as Fp5.
/// Uses fast fixed-base comb multiplication.
pub fn public_key_from_secret(sk: &Scalar) -> Fp5 {
    Point::mul_generator(sk).encode()
}

// ------------------------------------------------------------------
// Signing
// ------------------------------------------------------------------

/// Sign a pre-hashed message (40-byte Fp5 element).
///
pub fn sign_hashed_message(hashed_msg: &Fp5, sk: &Scalar) -> Signature {
    // Nonce from pre-generated pool (fast path)
    let k = pop_nonce();
    let r = Point::mul_generator(&k).encode();

    let preimage: Vec<_> = r.0.iter().chain(hashed_msg.0.iter()).copied().collect();
    let e_fp5 = poseidon2::hash_to_quintic_extension(&preimage);
    let e = Scalar::from_fp5(&e_fp5);

    let s = k.sub(&e.mul(sk));

    Signature { s, e }
}

/// Sign with a provided nonce (deterministic, for testing).
pub fn sign_with_nonce(hashed_msg: &Fp5, sk: &Scalar, k: &Scalar) -> Signature {
    let r = Point::mul_generator(k).encode();
    let preimage: Vec<_> = r.0.iter().chain(hashed_msg.0.iter()).copied().collect();
    let e_fp5 = poseidon2::hash_to_quintic_extension(&preimage);
    let e = Scalar::from_fp5(&e_fp5);
    let s = k.sub(&e.mul(sk));
    Signature { s, e }
}

// ------------------------------------------------------------------
// Verification
// ------------------------------------------------------------------

/// Verify a Schnorr signature over a pre-hashed message.
///
pub fn verify(pub_key: &Fp5, hashed_msg: &Fp5, sig: &Signature) -> bool {
    if !sig.is_canonical() {
        return false;
    }

    // Decode public key as Weierstrass point
    let (pk_ws, ok) = WeierstrassPoint::decode(pub_key);
    if !ok {
        return false;
    }

    // R' = s*G + e*P (Weierstrass double-scalar mult)
    let r_v = mul_add2(
        &WeierstrassPoint::GENERATOR,
        &pk_ws,
        &sig.s,
        &sig.e,
    ).encode();

    let preimage: Vec<_> = r_v.0.iter().chain(hashed_msg.0.iter()).copied().collect();
    let e_v_fp5 = poseidon2::hash_to_quintic_extension(&preimage);
    let e_v = Scalar::from_fp5(&e_v_fp5);

    e_v == sig.e
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::quintic::Fp5;

    fn dummy_msg() -> Fp5 {
        Fp5::from_u64_arr([1, 2, 3, 4, 5])
    }

    #[test]
    fn sign_verify_roundtrip() {
        let sk = Scalar::from_bytes_le(&{
            let mut b = [0u8; 40];
            b[0] = 42;
            b
        });
        let pk = public_key_from_secret(&sk);
        let msg = dummy_msg();
        let sig = sign_hashed_message(&msg, &sk);
        assert!(verify(&pk, &msg, &sig));
    }

    #[test]
    fn sign_verify_random() {
        // Use a deterministic nonce for reproducibility
        let sk = Scalar::from_bytes_le(&{
            let mut b = [0u8; 40];
            b[0] = 123;
            b
        });
        let pk = public_key_from_secret(&sk);
        let msg = dummy_msg();

        let k = Scalar::from_bytes_le(&{
            let mut b = [0u8; 40];
            b[0] = 99;
            b[1] = 1;
            b
        });
        let sig = sign_with_nonce(&msg, &sk, &k);
        assert!(verify(&pk, &msg, &sig));
    }

    #[test]
    fn wrong_message_fails() {
        let sk = Scalar::from_bytes_le(&{ let mut b = [0u8; 40]; b[0] = 7; b });
        let pk = public_key_from_secret(&sk);
        let msg = dummy_msg();
        let sig = sign_hashed_message(&msg, &sk);
        let other_msg = Fp5::from_u64_arr([99, 0, 0, 0, 0]);
        assert!(!verify(&pk, &other_msg, &sig));
    }

    #[test]
    fn wrong_key_fails() {
        let sk = Scalar::from_bytes_le(&{ let mut b = [0u8; 40]; b[0] = 7; b });
        let sk2 = Scalar::from_bytes_le(&{ let mut b = [0u8; 40]; b[0] = 8; b });
        let pk2 = public_key_from_secret(&sk2);
        let msg = dummy_msg();
        let sig = sign_hashed_message(&msg, &sk);
        assert!(!verify(&pk2, &msg, &sig));
    }

    #[test]
    fn sig_serialization() {
        let sk = Scalar::from_bytes_le(&{ let mut b = [0u8; 40]; b[0] = 1; b });
        let msg = dummy_msg();
        let k = Scalar::from_bytes_le(&{ let mut b = [0u8; 40]; b[0] = 2; b });
        let sig = sign_with_nonce(&msg, &sk, &k);
        let bytes = sig.to_bytes();
        assert_eq!(bytes.len(), 80);
        let sig2 = Signature::from_bytes(&bytes).unwrap();
        assert_eq!(sig.s, sig2.s);
        assert_eq!(sig.e, sig2.e);
    }
}
