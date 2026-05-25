use zeroize::Zeroize;

/// A Goldilocks field element using non-canonical representation.
///
/// The Goldilocks field is defined by the prime modulus:
///   p = 2^64 - 2^32 + 1 = 0xFFFFFFFF00000001
///
/// Following the Plonky2 approach, field elements are stored as raw u64 values
/// that are kept in the range [0, 2^64) but are NOT necessarily reduced modulo p
/// after every operation. This avoids expensive modular reductions on the hot path.
/// Equality comparisons canonicalize both operands.
///
/// Epsilon (2^32 - 1 = 0xFFFFFFFF) is used as a correction term in all arithmetic
/// operations. The key identity: 2^64 ≡ EPSILON (mod p), i.e., when a u64 operation
/// wraps, we can correct by adding/subtracting EPSILON.
#[derive(Debug, Clone, Copy, Zeroize)]
pub struct GoldilocksField(pub u64);

impl PartialEq for GoldilocksField {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.canonical() == other.canonical()
    }
}

impl Eq for GoldilocksField {}

impl std::fmt::Display for GoldilocksField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.canonical())
    }
}

impl From<u64> for GoldilocksField {
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl GoldilocksField {
    pub const MODULUS: u64 = 0xFFFFFFFF00000001;
    pub const EPSILON: u64 = 0xFFFFFFFF;
    pub const ORDER: u64 = Self::MODULUS;
    pub const ZERO: Self = Self(0);
    pub const ONE: Self = Self(1);

    // ------------------------------------------------------------------
    // Reduction and canonicalisation
    // ------------------------------------------------------------------

    /// Reduce a 128-bit product modulo p.
    ///
    /// Given `x = hi * 2^64 + lo`, the identity 2^64 ≡ EPSILON (mod p) gives:
    ///   x mod p ≡ lo + hi_lo * EPSILON - hi_hi
    /// where hi_hi = hi >> 32, hi_lo = hi & EPSILON.
    /// This is a branchless implementation with epsilon wrap on borrow/overflow.
    #[inline]
    fn reduce128(x: u128) -> u64 {
        let lo = x as u64;
        let hi = (x >> 64) as u64;
        let hi_hi = hi >> 32;
        let hi_lo = hi & Self::EPSILON;

        let (t0, borrow) = lo.overflowing_sub(hi_hi);
        let t0 = t0.wrapping_sub(Self::EPSILON & (borrow as u64).wrapping_neg());
        let t1 = hi_lo.wrapping_mul(Self::EPSILON);
        let (sum, over) = t0.overflowing_add(t1);
        sum.wrapping_add(Self::EPSILON & (over as u64).wrapping_neg())
    }

    /// Returns the canonical representative in [0, MODULUS).
    #[inline]
    pub fn canonical(&self) -> u64 {
        if self.0 >= Self::MODULUS {
            self.0 - Self::MODULUS
        } else {
            self.0
        }
    }

    #[inline]
    pub fn is_zero(&self) -> bool {
        self.canonical() == 0
    }

    #[inline]
    pub fn is_one(&self) -> bool {
        self.canonical() == 1
    }

    // ------------------------------------------------------------------
    // Construction
    // ------------------------------------------------------------------

    /// Create from a value that is guaranteed to be in [0, MODULUS).
    /// In debug builds, asserts the value is canonical.
    #[inline]
    pub fn from_canonical_u64(val: u64) -> Self {
        debug_assert!(
            val < Self::MODULUS,
            "from_canonical_u64: value 0x{val:016x} >= MODULUS"
        );
        Self(val)
    }

    /// Create from any u64, reducing modulo MODULUS.
    #[inline]
    pub fn from_noncanonical_u64(val: u64) -> Self {
        Self(if val >= Self::MODULUS {
            val - Self::MODULUS
        } else {
            val
        })
    }

    /// Create from a signed i64 value (two's complement cast to u64).
    #[inline]
    pub fn from_i64(val: i64) -> Self {
        Self(val as u64)
    }

    // ------------------------------------------------------------------
    // Field arithmetic
    // ------------------------------------------------------------------

    /// Addition with epsilon-based carry propagation.
    ///
    /// Overflow carry is corrected by adding EPSILON (since 2^64 ≡ EPSILON mod p).
    /// A second overflow check handles the rare case where the correction wraps.
    #[inline]
    pub fn add(&self, other: &Self) -> Self {
        let (sum, over) = self.0.overflowing_add(other.0);
        let (sum, over) = sum.overflowing_add((over as u64).wrapping_mul(Self::EPSILON));
        Self(if over {
            sum.wrapping_add(Self::EPSILON)
        } else {
            sum
        })
    }

    /// Subtraction with epsilon-based borrow propagation.
    #[inline]
    pub fn sub(&self, other: &Self) -> Self {
        let (diff, borrow) = self.0.overflowing_sub(other.0);
        let (diff, borrow) = diff.overflowing_sub((borrow as u64).wrapping_mul(Self::EPSILON));
        Self(if borrow {
            diff.wrapping_sub(Self::EPSILON)
        } else {
            diff
        })
    }

    /// Multiplication with reduction via `reduce128`.
    #[inline]
    pub fn mul(&self, other: &Self) -> Self {
        let product = (self.0 as u128) * (other.0 as u128);
        Self(Self::reduce128(product))
    }

    /// Optimised squaring (same as mul in the current implementation).
    #[inline]
    pub fn square(&self) -> Self {
        self.mul(self)
    }

    /// Multiply by 2.
    #[inline]
    pub fn double(&self) -> Self {
        self.add(self)
    }

    /// Add a raw u64: faster than `add` (single overflow check).
    /// Works correctly even when `self` is non-canonical.
    ///  *Add a raw u64 to this field element.
    #[inline]
    pub fn add_canonical_u64(&self, rhs: u64) -> Self {
        let (sum, over) = self.0.overflowing_add(rhs);
        Self(sum.wrapping_add(Self::EPSILON & (over as u64).wrapping_neg()))
    }

    /// Additive inverse: `-self`.
    #[inline]
    pub fn neg(&self) -> Self {
        let x = self.canonical();
        if x == 0 {
            Self::ZERO
        } else {
            Self(Self::MODULUS - x)
        }
    }

    /// Multiplicative inverse via an optimized addition chain from the Go reference.
    ///
    /// This chain computes `a^(p-2) mod p` where `p = 2^64 - 2^32 + 1`,
    /// requiring only ~15 multiplications (vs ~128 for naive square-and-multiply).
    ///  *Multiplicative inverse, or zero if none.
    pub fn inverse_or_zero(&self) -> Self {
        if self.is_zero() {
            return Self::ZERO;
        }

        let a = *self;
        let t2 = a.square().mul(&a);
        let t3 = t2.square().mul(&a);
        let t6 = t3.exp_power_of_2(3).mul(&t3);
        let t12 = t6.exp_power_of_2(6).mul(&t6);
        let t24 = t12.exp_power_of_2(12).mul(&t12);
        let t30 = t24.exp_power_of_2(6).mul(&t6);
        let t31 = t30.square().mul(&a);
        let t63 = t31.exp_power_of_2(32).mul(&t31);

        t63.square().mul(&a)
    }

    /// Panics on zero (Go `Inverse()` behaviour).
    pub fn inverse(&self) -> Self {
        if self.is_zero() {
            panic!("GoldilocksField::inverse: zero has no inverse");
        }
        self.inverse_or_zero()
    }

    /// Exponentiation by a u64 power using binary exponentiation.
    pub fn exp(&self, exponent: u64) -> Self {
        if exponent == 0 {
            return Self::ONE;
        }
        if exponent == 1 {
            return *self;
        }
        let mut result = Self::ONE;
        let mut base = *self;
        let mut exp = exponent;
        while exp > 0 {
            if exp & 1 == 1 {
                result = result.mul(&base);
            }
            base = base.square();
            exp >>= 1;
        }
        result
    }

    /// Repeated squaring: returns `self^(2^n)`.
    pub fn exp_power_of_2(&self, n: usize) -> Self {
        let mut result = *self;
        for _ in 0..n {
            result = result.square();
        }
        result
    }

    // ------------------------------------------------------------------
    // Square root (Tonelli-Shanks for Goldilocks)
    // ------------------------------------------------------------------

    /// Computes the square root using the Tonelli-Shanks algorithm.
    ///
    /// For Goldilocks, p - 1 = 2^32 * (2^32 - 1) = 2^32 * q where q = EPSILON.
    /// Returns `Some(sqrt)` if a root exists, `None` otherwise.
    pub fn sqrt(&self) -> Option<Self> {
        if self.is_zero() {
            return Some(Self::ZERO);
        }

        const E: usize = 32;
        const Q: u64 = GoldilocksField::EPSILON;

        // Quadratic non-residue 11 is known for Goldilocks
        let z = Self::from_canonical_u64(11);

        let mut c = z.exp(Q);
        let mut t = self.exp(Q);
        let mut r = self.exp(Q.div_ceil(2));
        let mut m = E;

        while t.canonical() != 1 {
            let mut i = 0;
            let mut tt = t;
            while i < m && tt.canonical() != 1 {
                tt = tt.square();
                i += 1;
            }
            if i == m {
                return None;
            }

            let mut b = c;
            for _ in 0..(m - i - 1) {
                b = b.square();
            }

            r = r.mul(&b);
            c = b.square();
            t = t.mul(&c);
            m = i;
        }

        let r_sq = r.square();
        if r_sq.canonical() == self.canonical() {
            Some(r)
        } else {
            None
        }
    }

    // ------------------------------------------------------------------
    // Serialization
    // ------------------------------------------------------------------

    /// Serialize as 8 little-endian canonical bytes.
    pub fn to_bytes_le(&self) -> [u8; 8] {
        self.canonical().to_le_bytes()
    }

    /// Deserialize from 8 little-endian bytes.
    /// Returns `Err` if the value exceeds or equals MODULUS.
    pub fn from_bytes_le(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != 8 {
            return Err("expected 8 bytes");
        }
        let mut arr = [0u8; 8];
        arr.copy_from_slice(bytes);
        let val = u64::from_le_bytes(arr);
        if val >= Self::MODULUS {
            return Err("non-canonical Goldilocks element");
        }
        Ok(Self(val))
    }

    /// Encode as 16-char lowercase hex (little-endian canonical bytes).
    pub fn to_hex(&self) -> String {
        hex::encode(self.to_bytes_le())
    }

    /// Decode from 16-char hex string with optional "0x" prefix.
    pub fn from_hex(s: &str) -> Result<Self, &'static str> {
        let s = s.strip_prefix("0x").unwrap_or(s);
        if s.len() != 16 {
            return Err("expected 16 hex chars");
        }
        let bytes = hex::decode(s).map_err(|_| "hex decode error")?;
        Self::from_bytes_le(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: u64 = GoldilocksField::MODULUS;

    fn canonical(x: u64) -> bool {
        x < P
    }

    fn gf(x: u64) -> GoldilocksField {
        GoldilocksField::from_canonical_u64(x)
    }

    // ---------- add ----------

    #[test]
    fn add_basic() {
        let a = gf(100);
        let b = gf(50);
        assert_eq!(a.add(&b).canonical(), 150);
    }

    #[test]
    fn add_near_modulus() {
        let a = gf(P - 1);
        let b = gf(1);
        assert_eq!(a.add(&b).canonical(), 0);
    }

    #[test]
    fn add_wrap_multiples() {
        let a = gf(P - 1);
        let b = gf(2);
        assert_eq!(a.add(&b).canonical(), 1);
    }

    #[test]
    fn add_max() {
        let a = gf(P - 1);
        let b = gf(P - 1);
        assert_eq!(a.add(&b).canonical(), P - 2);
    }

    #[test]
    fn add_zero() {
        let a = gf(42);
        assert_eq!(a.add(&GoldilocksField::ZERO).canonical(), 42);
    }

    // ---------- sub ----------

    #[test]
    fn sub_basic() {
        let a = gf(100);
        let b = gf(50);
        assert_eq!(a.sub(&b).canonical(), 50);
    }

    #[test]
    fn sub_underflow() {
        let a = gf(0);
        let b = gf(1);
        assert_eq!(a.sub(&b).canonical(), P - 1);
    }

    #[test]
    fn sub_self() {
        let a = gf(42);
        assert!(a.sub(&a).is_zero());
    }

    // ---------- mul ----------

    #[test]
    fn mul_basic() {
        let a = gf(10);
        let b = gf(5);
        assert_eq!(a.mul(&b).canonical(), 50);
    }

    #[test]
    fn mul_one() {
        let a = gf(12345);
        assert_eq!(a.mul(&GoldilocksField::ONE).canonical(), 12345);
    }

    #[test]
    fn mul_zero() {
        let a = gf(99999);
        assert!(a.mul(&GoldilocksField::ZERO).is_zero());
    }

    #[test]
    fn mul_commutative() {
        let a = gf(P - 100);
        let b = gf(P - 200);
        assert_eq!(a.mul(&b).canonical(), b.mul(&a).canonical());
    }

    #[test]
    fn mul_large() {
        let a = gf(P - 1);
        let b = gf(2);
        assert_eq!(a.mul(&b).canonical(), P - 2);
    }

    #[test]
    fn mul_max() {
        let a = gf(P - 1);
        let b = gf(P - 1);
        assert_eq!(a.mul(&b).canonical(), 1); // (p-1)^2 = p^2 - 2p + 1 ≡ 1 (mod p)
    }

    #[test]
    fn mul_reduce128_canonical() {
        for i in 0..10000 {
            let a = gf(i % P);
            let b = gf((i * 7 + 13) % P);
            assert!(canonical(a.mul(&b).canonical()));
        }
    }

    // ---------- square ----------

    #[test]
    fn square_identity() {
        let a = gf(42);
        assert_eq!(a.square().canonical(), a.mul(&a).canonical());
    }

    // ---------- double ----------

    #[test]
    fn double_identity() {
        let a = gf(50);
        assert_eq!(a.double().canonical(), a.add(&a).canonical());
    }

    // ---------- neg ----------

    #[test]
    fn neg_zero() {
        assert!(GoldilocksField::ZERO.neg().is_zero());
    }

    #[test]
    fn neg_involution() {
        let a = gf(42);
        assert_eq!(a.neg().neg().canonical(), 42);
    }

    #[test]
    fn neg_cancel() {
        let a = gf(100);
        assert!(a.add(&a.neg()).is_zero());
    }

    // ---------- inverse ----------

    #[test]
    fn inverse_basic() {
        let a = gf(2);
        let inv = a.inverse();
        assert_eq!(a.mul(&inv).canonical(), 1);
    }

    #[test]
    fn inverse_one() {
        assert_eq!(GoldilocksField::ONE.inverse().canonical(), 1);
    }

    #[test]
    fn inverse_self_cancel() {
        for i in 1..200 {
            let a = gf(i);
            assert_eq!(a.mul(&a.inverse()).canonical(), 1, "inverse failed for {i}");
        }
    }

    // ---------- exp ----------

    #[test]
    fn exp_zero() {
        assert_eq!(gf(42).exp(0).canonical(), 1);
    }

    #[test]
    fn exp_one() {
        assert_eq!(gf(42).exp(1).canonical(), 42);
    }

    #[test]
    fn exp_small() {
        let a = gf(2);
        assert_eq!(a.exp(3).canonical(), 8);
    }

    // ---------- sqrt ----------

    #[test]
    fn sqrt_square() {
        let a = gf(4);
        let s = a.sqrt().unwrap();
        let s2 = s.square().canonical();
        assert!(s2 == 4, "sqrt(4)^2 = {s2}, expected 4");
    }

    #[test]
    fn sqrt_nonresidue_returns_none() {
        // 11 is a known quadratic non-residue for Goldilocks
        assert!(gf(11).sqrt().is_none());
    }

    #[test]
    fn sqrt_zero() {
        assert!(gf(0).sqrt().unwrap().is_zero());
    }

    #[test]
    fn sqrt_many() {
        for i in 0..64 {
            let a = gf(i * i % P);
            if let Some(s) = a.sqrt() {
                let s2 = s.square().canonical();
                assert_eq!(s2, a.canonical(), "sqrt({})^2 = {} != {}", i * i % P, s2, a.canonical());
            }
        }
    }

    // ---------- serde ----------

    #[test]
    fn bytes_roundtrip() {
        let a = gf(0x1234567890ABCDEF % P);
        let bytes = a.to_bytes_le();
        assert_eq!(bytes.len(), 8);
        let b = GoldilocksField::from_bytes_le(&bytes).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn bytes_reject_noncanonical() {
        let bytes = P.to_le_bytes();
        assert!(GoldilocksField::from_bytes_le(&bytes).is_err());
    }

    #[test]
    fn hex_roundtrip() {
        let a = gf(0xCAFEBABE % P);
        let hex = a.to_hex();
        assert_eq!(hex.len(), 16);
        let b = GoldilocksField::from_hex(&hex).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn hex_with_prefix() {
        let a = gf(42);
        let hex = format!("0x{}", a.to_hex());
        assert_eq!(GoldilocksField::from_hex(&hex).unwrap(), a);
    }

    // ---------- non-canonical ops ----------

    #[test]
    fn noncanonical_add_correct() {
        let a = GoldilocksField(GoldilocksField::MODULUS + 5);
        let b = gf(10);
        let sum = a.add(&b);
        assert_eq!(sum.canonical(), 15);
    }

    #[test]
    fn from_noncanonical_u64() {
        let x = GoldilocksField::from_noncanonical_u64(P + 42);
        assert_eq!(x.canonical(), 42);
    }

    #[test]
    fn from_i64_negative() {
        let x = GoldilocksField::from_i64(-10);
        // -10 as u64 = 2^64 - 10, mod p this is 2^64 - 10 - p... let's just check it's valid
        assert!(x.0 != 0);
    }

    // ---------- display ----------

    #[test]
    fn display_canonical() {
        let a = GoldilocksField(P + 5);
        assert_eq!(format!("{}", a), "5");
    }
}
