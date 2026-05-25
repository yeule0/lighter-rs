use crate::field::quintic::Fp5;

/// ECgFp5Scalar — the scalar field of the ECgFP5 elliptic curve.
///
/// Group order n ≈ 2^319 is a prime. Values are stored in normal
/// (non-Montgomery) representation over 5 × 64-bit little-endian limbs.
/// Montgomery multiplication is used internally for multiplication.
///
#[derive(Debug, Clone, Copy, Default)]
pub struct Scalar(pub [u64; 5]);

impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl Eq for Scalar {}

impl Scalar {
    // Group order n (prime, ≈2^319).
    pub const N: [u64; 5] = [
        0xE80FD996948BFFE1, 0xE8885C39D724A09C, 0x7FFFFFE6CFB80639,
        0x7FFFFFF100000016, 0x7FFFFFFD80000007,
    ];
    /// -1/N[0] mod 2^64
    pub const N0I: u64 = 0xD78BEF72057B7BDF;
    /// R^2 mod N where R = 2^320
    pub const R2: [u64; 5] = [
        0xA01001DCE33DC739, 0x6C3228D33F62ACCF, 0xD1D796CC91CF8525,
        0xAADFFF5D1574C1D8, 0x4ACA13B28CA251F5,
    ];
    /// 2^632 mod N
    pub const T632: [u64; 5] = [
        0x2B0266F317CA91B3, 0xEC1D26528E984773, 0x8651D7865E12DB94,
        0xDA2ADFF5941574D0, 0x53CACA12110CA256,
    ];

    pub const ZERO: Self = Scalar([0, 0, 0, 0, 0]);
    pub const ONE: Self = Scalar([1, 0, 0, 0, 0]);
    pub const TWO: Self = Scalar([2, 0, 0, 0, 0]);
    pub const NEG_ONE: Self = Scalar([
        0xE80FD996948BFFE0, 0xE8885C39D724A09C, 0x7FFFFFE6CFB80639,
        0x7FFFFFF100000016, 0x7FFFFFFD80000007,
    ]);

    // ------------------------------------------------------------------
    // Construction / serialization
    // ------------------------------------------------------------------

    pub fn from_bytes_le(bytes: &[u8; 40]) -> Self {
        let mut s = [0u64; 5];
        for i in 0..5 {
            s[i] = u64::from_le_bytes(bytes[i * 8..(i + 1) * 8].try_into().unwrap());
        }
        // Reduce mod N if >= N
        if bigint_cmp(&s, &Self::N) >= 0 {
            let big = limbs_to_bigint(&s);
            let reduced = bigint_reduce(&big);
            return Scalar(reduced);
        }
        Scalar(s)
    }

    pub fn to_bytes_le(&self) -> [u8; 40] {
        let mut out = [0u8; 40];
        for i in 0..5 {
            out[i * 8..(i + 1) * 8].copy_from_slice(&self.0[i].to_le_bytes());
        }
        out
    }

    /// Check if the scalar is canonical (< N).
    pub fn is_canonical(&self) -> bool {
        bigint_cmp(&self.0, &Self::N) < 0
    }

    // ------------------------------------------------------------------
    // Raw addition (no reduction)
    // ------------------------------------------------------------------

    fn add_inner(a: &[u64; 5], b: &[u64; 5]) -> [u64; 5] {
        let mut r = [0u64; 5];
        let mut carry: u64 = 0;
        for i in 0..5 {
            let sum = (a[i] as u128) + (b[i] as u128) + (carry as u128);
            r[i] = sum as u64;
            carry = (sum >> 64) as u64;
        }
        r
    }

    fn sub_inner(a: &[u64; 5], b: &[u64; 5]) -> ([u64; 5], u64) {
        let mut r = [0u64; 5];
        let mut borrow: u64 = 0;
        for i in 0..5 {
            let diff = (a[i] as u128).wrapping_sub((b[i] as u128) + (borrow as u128));
            r[i] = diff as u64;
            borrow = ((diff >> 64) & 1) as u64;
        }
        let c = if borrow != 0 { 0xFFFFFFFFFFFFFFFF } else { 0 };
        (r, c)
    }

    // ------------------------------------------------------------------
    // Canonical add / sub
    // ------------------------------------------------------------------

    pub fn add(&self, rhs: &Self) -> Self {
        debug_assert!(self.is_canonical());
        debug_assert!(rhs.is_canonical());
        let r0 = Self::add_inner(&self.0, &rhs.0);
        let (r1, c) = Self::sub_inner(&r0, &Self::N);
        Self::select(c, r1, r0)
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        debug_assert!(self.is_canonical());
        debug_assert!(rhs.is_canonical());
        let (r0, c) = Self::sub_inner(&self.0, &rhs.0);
        let r1 = Self::add_inner(&r0, &Self::N);
        Self::select(c, r0, r1)
    }

    pub fn neg(&self) -> Self {
        Self::ZERO.sub(self)
    }

    // ------------------------------------------------------------------
    // Constant-time select
    // ------------------------------------------------------------------

    #[inline]
    fn select(c: u64, a0: [u64; 5], a1: [u64; 5]) -> Self {
        Scalar([
            a0[0] ^ (c & (a0[0] ^ a1[0])),
            a0[1] ^ (c & (a0[1] ^ a1[1])),
            a0[2] ^ (c & (a0[2] ^ a1[2])),
            a0[3] ^ (c & (a0[3] ^ a1[3])),
            a0[4] ^ (c & (a0[4] ^ a1[4])),
        ])
    }

    // ------------------------------------------------------------------
    // Multiplication
    // ------------------------------------------------------------------

    pub fn mul(&self, rhs: &Self) -> Self {
        debug_assert!(self.is_canonical());
        debug_assert!(rhs.is_canonical());
        let s_mont = self.monty_mul(&Self::R2);
        s_mont.monty_mul(&rhs.0)
    }

    /// Montgomery multiplication — matches Go MontyMul.
    /// `self` MUST be in Montgomery form (< N).
    /// `rhs` can be up to 2^320-1 (non-canonical).
    fn monty_mul(&self, rhs: &[u64; 5]) -> Self {
        debug_assert!(self.is_canonical());
        let s = &self.0;
        let mut r = [0u64; 5];

        for &m in rhs.iter() {
            let f = s[0].wrapping_mul(m).wrapping_add(r[0]).wrapping_mul(Self::N0I);

            let mut cc1: u64 = 0;
            let mut cc2: u64 = 0;
            for j in 0..5 {
                // z = s[j]*m + r[j] + cc1
                let z = (s[j] as u128) * (m as u128) + (r[j] as u128) + (cc1 as u128);
                cc1 = (z >> 64) as u64;
                // z = f*N[j] + z.lo + cc2
                let z = (f as u128) * (Self::N[j] as u128) + (z as u64 as u128) + (cc2 as u128);
                cc2 = (z >> 64) as u64;
                if j > 0 {
                    r[j - 1] = z as u64;
                }
            }
            r[4] = cc1.wrapping_add(cc2);
        }

        let (r2, c) = Self::sub_inner(&r, &Self::N);
        Self::select(c, r2, r)
    }

    // ------------------------------------------------------------------
    // From Fp5
    // ------------------------------------------------------------------

    pub fn from_fp5(fp5: &Fp5) -> Self {
        let mut limbs = [0u64; 5];
        for (limb, coeff) in limbs.iter_mut().zip(fp5.0.iter()) {
            *limb = coeff.canonical();
        }
        // Reduce mod N
        let big = limbs_to_bigint(&limbs);
        Scalar(bigint_reduce(&big))
    }

    // ------------------------------------------------------------------
    // Recode signed
    // ------------------------------------------------------------------

    pub fn recode_signed(&self, ss: &mut [i32], w: i32) {
        debug_assert!((2..=10).contains(&w));
        recode_signed_from_limbs(&self.0, ss, w)
    }

    pub fn split_to_4bit_limbs(&self) -> [u8; 80] {
        let mut result = [0u8; 80];
        for i in 0..5 {
            for j in 0..16 {
                result[i * 16 + j] = ((self.0[i] >> (j * 4)) & 0xF) as u8;
            }
        }
        result
    }
}

// ------------------------------------------------------------------
// Big-integer helpers (5-limb, little-endian)
// Returns -1 if a < b, 0 if a == b, 1 if a > b.
fn bigint_cmp(a: &[u64; 5], b: &[u64; 5]) -> i32 {
    for i in (0..5).rev() {
        if a[i] < b[i] { return -1; }
        if a[i] > b[i] { return 1; }
    }
    0
}

/// Simple modular reduction of a 320-bit value mod N.
fn bigint_reduce(val: &[u64; 5]) -> [u64; 5] {
    let mut v = *val;
    while bigint_cmp(&v, &Scalar::N) >= 0 {
        let (r, _) = Scalar::sub_inner(&v, &Scalar::N);
        v = r;
    }
    v
}

fn limbs_to_bigint(limbs: &[u64; 5]) -> [u64; 5] {
    *limbs
}

fn recode_signed_from_limbs(limbs: &[u64], ss: &mut [i32], w: i32) {
    let mw: u32 = (1u32 << w) - 1;
    let hw: u32 = 1u32 << (w - 1);
    let mut acc: u64 = 0;
    let mut acc_len: i32 = 0;
    let mut j: usize = 0;
    let mut cc: u32 = 0;

    for s in ss.iter_mut() {
        let bb: u32;
        if acc_len < w {
            if j < limbs.len() {
                let nl = limbs[j];
                j += 1;
                bb = ((acc | (nl << acc_len)) as u32) & mw;
                acc = nl >> (w - acc_len);
            } else {
                bb = (acc as u32) & mw;
                acc = 0;
            }
            acc_len += 64 - w;
        } else {
            bb = (acc as u32) & mw;
            acc_len -= w;
            acc >>= w;
        }

        let b = bb.wrapping_add(cc);
        cc = hw.wrapping_sub(b) >> 31;
        *s = (b as i32) - ((cc << w) as i32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_add_one() {
        let r = Scalar::ZERO.add(&Scalar::ONE);
        assert_eq!(r, Scalar::ONE);
    }

    #[test]
    fn add_sub_roundtrip() {
        let a = Scalar::from_bytes_le(&[42u8; 40]);
        let r = a.add(&a).sub(&a);
        assert_eq!(r, a);
    }

    #[test]
    fn mul_one() {
        let a = Scalar::from_bytes_le(&Scalar::ONE.to_bytes_le());
        assert_eq!(a.mul(&Scalar::ONE), a);
    }

    #[test]
    fn mul_two() {
        let one = Scalar::ONE;
        let two = one.mul(&Scalar::TWO);
        assert_eq!(two, one.add(&one));
    }

    #[test]
    fn mul_commutative() {
        let a = Scalar::from_bytes_le(&Scalar::TWO.to_bytes_le());
        let b = Scalar::from_bytes_le(&Scalar([3,0,0,0,0]).to_bytes_le());
        assert_eq!(a.mul(&b), b.mul(&a));
    }

    #[test]
    fn neg_one_add_one() {
        let r = Scalar::NEG_ONE.add(&Scalar::ONE);
        assert_eq!(r, Scalar::ZERO);
    }

    #[test]
    fn serde_roundtrip() {
        let a = Scalar::ONE;
        let bytes = a.to_bytes_le();
        assert_eq!(bytes.len(), 40);
        let b = Scalar::from_bytes_le(&bytes);
        assert_eq!(a, b);
    }

    #[test]
    fn is_canonical() {
        assert!(Scalar::ZERO.is_canonical());
        assert!(Scalar::ONE.is_canonical());
        // NEG_ONE = N - 1, which is < N, so it IS canonical
        assert!(Scalar::NEG_ONE.is_canonical());
    }
}
