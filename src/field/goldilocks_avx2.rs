#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

use crate::field::goldilocks::GoldilocksField;

/// Four Goldilocks field elements packed into one YMM register for SIMD processing.
#[derive(Debug, Clone, Copy)]
pub struct GoldilocksFieldx4(pub [GoldilocksField; 4]);

// ------------------------------------------------------------------
// AVX2 helper intrinsics (unsafe — must only be used in #[target_feature(enable = "avx2")] fns)
// ------------------------------------------------------------------

#[cfg(target_arch = "x86_64")]
mod avx2_helpers {
    use std::arch::x86_64::*;
    use crate::field::goldilocks::GoldilocksField;

    pub const EPS: i64 = 0x00000000FFFFFFFFu64 as i64;
    pub const MSB: i64 = 0x8000000000000000u64 as i64;
    pub const LO32_MASK: i64 = 0x00000000FFFFFFFFu64 as i64;
    pub const ONE: i64 = 1;

    /// Unsigned 64-bit greater-than on packed 4-way lanes.
    /// AVX2 lacks `cmpgt_epu64`; we flip the sign bit so signed compare works.
    #[inline(always)]
    pub unsafe fn cmpgt_epu64(a: __m256i, b: __m256i) -> __m256i {
        let msb = _mm256_set1_epi64x(MSB);
        let a_flip = _mm256_xor_si256(a, msb);
        let b_flip = _mm256_xor_si256(b, msb);
        _mm256_cmpgt_epi64(a_flip, b_flip)
    }

    /// Per-lane `(a + b) % 2^64` and a mask indicating where overflow occurred.
    /// Returns `(sum, overflow_mask)` where overflow_mask is all-ones for overflowed lanes.
    #[inline(always)]
    pub unsafe fn add_with_carry(a: __m256i, b: __m256i) -> (__m256i, __m256i) {
        let sum = _mm256_add_epi64(a, b);
        let overflow = cmpgt_epu64(a, sum);
        (sum, overflow)
    }

    /// Per-lane `(a - b) % 2^64` and a mask where borrow occurred.
    #[inline(always)]
    pub unsafe fn sub_with_borrow(a: __m256i, b: __m256i) -> (__m256i, __m256i) {
        let diff = _mm256_sub_epi64(a, b);
        let borrow = cmpgt_epu64(b, a); // borrow iff b > a (unsigned)
        (diff, borrow)
    }

    /// Load 4 u64 values from a `[GoldilocksField; 4]`.
    #[inline(always)]
    pub unsafe fn load_from(arr: &[GoldilocksField; 4]) -> __m256i {
        let vals: [u64; 4] = [arr[0].0, arr[1].0, arr[2].0, arr[3].0];
        _mm256_loadu_si256(vals.as_ptr() as *const __m256i)
    }

    /// Store a YMM register to a `[GoldilocksField; 4]`.
    #[inline(always)]
    pub unsafe fn store_to(v: __m256i) -> [GoldilocksField; 4] {
        let mut out = [0u64; 4];
        _mm256_storeu_si256(out.as_mut_ptr() as *mut __m256i, v);
        [
            GoldilocksField(out[0]),
            GoldilocksField(out[1]),
            GoldilocksField(out[2]),
            GoldilocksField(out[3]),
        ]
    }
}

// ------------------------------------------------------------------
// Scalar fallback
// ------------------------------------------------------------------

#[inline]
fn scalar_4way_op(
    a: &GoldilocksFieldx4,
    b: &GoldilocksFieldx4,
    op: fn(&GoldilocksField, &GoldilocksField) -> GoldilocksField,
) -> GoldilocksFieldx4 {
    let mut out = [GoldilocksField::ZERO; 4];
    for (o, (ai, bi)) in out.iter_mut().zip(a.0.iter().zip(b.0.iter())) {
        *o = op(ai, bi);
    }
    GoldilocksFieldx4(out)
}

// ------------------------------------------------------------------
// GoldilocksFieldx4 implementation
// ------------------------------------------------------------------

#[cfg(not(target_arch = "x86_64"))]
impl GoldilocksFieldx4 {
    pub fn from_scalars(a: [GoldilocksField; 4]) -> Self { Self(a) }
    pub fn to_scalars(&self) -> [GoldilocksField; 4] { self.0 }

    pub fn add(&self, other: &Self) -> Self { scalar_4way_op(self, other, GoldilocksField::add) }
    pub fn sub(&self, other: &Self) -> Self { scalar_4way_op(self, other, GoldilocksField::sub) }
    pub fn mul(&self, other: &Self) -> Self { scalar_4way_op(self, other, GoldilocksField::mul) }
}

#[cfg(target_arch = "x86_64")]
impl GoldilocksFieldx4 {
    pub fn from_scalars(a: [GoldilocksField; 4]) -> Self { Self(a) }
    pub fn to_scalars(&self) -> [GoldilocksField; 4] { self.0 }

    // --- dispatch wrappers ---

    pub fn add(&self, other: &Self) -> Self {
        if crate::has_avx2() { unsafe { self.add_avx2_unchecked(other) } }
        else { scalar_4way_op(self, other, GoldilocksField::add) }
    }

    pub fn sub(&self, other: &Self) -> Self {
        if crate::has_avx2() { unsafe { self.sub_avx2_unchecked(other) } }
        else { scalar_4way_op(self, other, GoldilocksField::sub) }
    }

    pub fn mul(&self, other: &Self) -> Self {
        if crate::has_avx2() { unsafe { self.mul_avx2_unchecked(other) } }
        else { scalar_4way_op(self, other, GoldilocksField::mul) }
    }

    // --- AVX2 add (two-pass overflow correction) ---

    /// AVX2 4-way add.
    ///
    /// # Safety
    /// Requires AVX2. Caller must ensure `is_x86_feature_detected!("avx2")`.
    #[target_feature(enable = "avx2")]
    pub unsafe fn add_avx2_unchecked(&self, other: &Self) -> Self {
        use avx2_helpers::*;
        let a = load_from(&self.0);
        let b = load_from(&other.0);
        let eps = _mm256_set1_epi64x(EPS);

        let (sum, over) = add_with_carry(a, b);
        let correction = _mm256_and_si256(over, eps);
        let (sum, over2) = add_with_carry(sum, correction);
        let correction2 = _mm256_and_si256(over2, eps);
        let sum = _mm256_add_epi64(sum, correction2);
        Self(store_to(sum))
    }

    /// AVX2 4-way sub.
    ///
    /// # Safety
    /// Requires AVX2. Caller must ensure `is_x86_feature_detected!("avx2")`.
    #[target_feature(enable = "avx2")]
    pub unsafe fn sub_avx2_unchecked(&self, other: &Self) -> Self {
        use avx2_helpers::*;
        let a = load_from(&self.0);
        let b = load_from(&other.0);
        let eps = _mm256_set1_epi64x(EPS);

        let (diff, borrow) = sub_with_borrow(a, b);
        let correction = _mm256_and_si256(borrow, eps);
        let (diff, borrow2) = sub_with_borrow(diff, correction);
        let correction2 = _mm256_and_si256(borrow2, eps);
        let diff = _mm256_sub_epi64(diff, correction2);
        Self(store_to(diff))
    }

    // --- AVX2 multiply (32-bit decomposition → reduce128) ---

    /// AVX2 4-way multiply using 32-bit decomposition.
    ///
    /// # Safety
    /// Requires AVX2. Caller must ensure `is_x86_feature_detected!("avx2")`.
    #[target_feature(enable = "avx2")]
    pub unsafe fn mul_avx2_unchecked(&self, other: &Self) -> Self {
        use avx2_helpers::*;
        let a = load_from(&self.0);
        let b = load_from(&other.0);
        let eps = _mm256_set1_epi64x(EPS);
        let lo32_mask = _mm256_set1_epi64x(LO32_MASK);
        let one = _mm256_set1_epi64x(ONE);

        // --- step 1: decompose into 32-bit halves ---
        let a_lo = _mm256_and_si256(a, lo32_mask);
        let a_hi = _mm256_srli_epi64(a, 32);
        let b_lo = _mm256_and_si256(b, lo32_mask);
        let b_hi = _mm256_srli_epi64(b, 32);

        // --- step 2: four 32×32→64 cross products ---
        let p00 = _mm256_mul_epu32(a_lo, b_lo);  // a_lo * b_lo
        let p01 = _mm256_mul_epu32(a_lo, b_hi);  // a_lo * b_hi
        let p10 = _mm256_mul_epu32(a_hi, b_lo);  // a_hi * b_lo
        let p11 = _mm256_mul_epu32(a_hi, b_hi);  // a_hi * b_hi

        // --- step 3: build 128-bit (hi, lo) per lane ---
        // mid = p01 + p10    (may overflow u64 by 1)
        let (mid, mid_carry) = add_with_carry(p01, p10);
        let mid_lo = _mm256_and_si256(mid, lo32_mask);
        let mid_hi = _mm256_srli_epi64(mid, 32);

        // lo_64 = p00 + (mid_lo << 32)   (may overflow by 1)
        let mid_lo_shifted = _mm256_slli_epi64(mid_lo, 32);
        let (lo, lo_carry) = add_with_carry(p00, mid_lo_shifted);

        // hi_64 = p11 + mid_hi + lo_carry + (mid_carry << 32)
        let mut hi = _mm256_add_epi64(p11, mid_hi);
        hi = _mm256_add_epi64(hi, _mm256_and_si256(lo_carry, one));
        let mid_carry_shifted = _mm256_slli_epi64(
            _mm256_and_si256(mid_carry, one),
            32,
        );
        hi = _mm256_add_epi64(hi, mid_carry_shifted);

        // Overflow detection: if hi < p11 (unsigned), an overflow occurred.
        // The extra 2^64 ≡ EPSILON (mod p) will be added after the reduction.
        let hi_overflow = cmpgt_epu64(p11, hi);  // p11 > hi iff overflow
        let hi_extra = _mm256_and_si256(hi_overflow, one);

        // --- step 4: reduction ---
        // result ≡ lo + hi_lo * EPSILON - hi_hi  (mod p)
        let hi_hi = _mm256_srli_epi64(hi, 32);
        let hi_lo = _mm256_and_si256(hi, lo32_mask);

        // t0 = lo - hi_hi  (epsilon-wrap on borrow)
        let (t0, borrow0) = sub_with_borrow(lo, hi_hi);
        let t0 = _mm256_sub_epi64(t0, _mm256_and_si256(borrow0, eps));

        // t1 = hi_lo * EPSILON  (both < 2^32, 32×32→64 fits)
        let t1 = _mm256_mul_epu32(hi_lo, eps);

        // t2 = t0 + t1  (epsilon-wrap on overflow)
        let (t2, carry1) = add_with_carry(t0, t1);
        let t2 = _mm256_add_epi64(t2, _mm256_and_si256(carry1, eps));

        // Add extra EPSILON if hi overflowed
        let extra = _mm256_and_si256(hi_extra, eps);
        let (t2, final_overflow) = add_with_carry(t2, extra);
        let t2 = _mm256_add_epi64(t2, _mm256_and_si256(final_overflow, eps));

        Self(store_to(t2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::goldilocks::GoldilocksField;

    fn gf(x: u64) -> GoldilocksField {
        GoldilocksField::from_canonical_u64(x)
    }

    fn gf4(a: [u64; 4]) -> GoldilocksFieldx4 {
        GoldilocksFieldx4([gf(a[0]), gf(a[1]), gf(a[2]), gf(a[3])])
    }

    const P: u64 = GoldilocksField::MODULUS;

    #[test]
    fn avx2_add_basic() {
        let a = gf4([1, 2, 3, 4]);
        let b = gf4([10, 20, 30, 40]);
        let r = a.add(&b).to_scalars();
        assert_eq!(r[0].canonical(), 11);
        assert_eq!(r[1].canonical(), 22);
        assert_eq!(r[2].canonical(), 33);
        assert_eq!(r[3].canonical(), 44);
    }

    #[test]
    fn avx2_add_wrap() {
        let a = gf4([P - 1, P - 1, 0, 42]);
        let b = gf4([1, 2, 0, 10]);
        let r = a.add(&b).to_scalars();
        assert_eq!(r[0].canonical(), 0);
        assert_eq!(r[1].canonical(), 1);
        assert_eq!(r[2].canonical(), 0);
        assert_eq!(r[3].canonical(), 52);
    }

    #[test]
    fn avx2_add_max() {
        let a = gf4([P - 1; 4]);
        let b = gf4([P - 1; 4]);
        let r = a.add(&b).to_scalars();
        assert_eq!(r[0].canonical(), P - 2);
        assert_eq!(r[1].canonical(), P - 2);
        assert_eq!(r[2].canonical(), P - 2);
        assert_eq!(r[3].canonical(), P - 2);
    }

    #[test]
    fn avx2_sub_basic() {
        let a = gf4([100, 200, 300, 400]);
        let b = gf4([30, 20, 10, 5]);
        let r = a.sub(&b).to_scalars();
        assert_eq!(r[0].canonical(), 70);
        assert_eq!(r[1].canonical(), 180);
        assert_eq!(r[2].canonical(), 290);
        assert_eq!(r[3].canonical(), 395);
    }

    #[test]
    fn avx2_sub_underflow() {
        let a = gf4([0, 0, 0, 0]);
        let b = gf4([1, 2, 3, 4]);
        let r = a.sub(&b).to_scalars();
        assert_eq!(r[0].canonical(), P - 1);
        assert_eq!(r[1].canonical(), P - 2);
        assert_eq!(r[2].canonical(), P - 3);
        assert_eq!(r[3].canonical(), P - 4);
    }

    #[test]
    fn avx2_agrees_with_scalar_add() {
        let a = gf4([P - 100, 42, P / 2, 1]);
        let b = gf4([P - 50, 99, P / 4, P - 1]);
        let r_avx = a.add(&b).to_scalars();
        for (i, (r_val, (ai, bi))) in r_avx.iter().zip(a.0.iter().zip(b.0.iter())).enumerate() {
            let r_scalar = ai.add(bi);
            assert_eq!(r_val.canonical(), r_scalar.canonical(), "add mismatch at index {i}");
        }
    }

    #[test]
    fn avx2_agrees_with_scalar_sub() {
        let a = gf4([P - 100, 42, P / 2, 1]);
        let b = gf4([P - 50, 99, P / 4, P - 1]);
        let r_avx = a.sub(&b).to_scalars();
        for (i, (r_val, (ai, bi))) in r_avx.iter().zip(a.0.iter().zip(b.0.iter())).enumerate() {
            let r_scalar = ai.sub(bi);
            assert_eq!(r_val.canonical(), r_scalar.canonical(), "sub mismatch at index {i}");
        }
    }

    #[test]
    fn avx2_mul_basic() {
        let a = gf4([2, 3, 5, 7]);
        let b = gf4([4, 5, 6, 8]);
        let r = a.mul(&b).to_scalars();
        assert_eq!(r[0].canonical(), 8);
        assert_eq!(r[1].canonical(), 15);
        assert_eq!(r[2].canonical(), 30);
        assert_eq!(r[3].canonical(), 56);
    }

    #[test]
    fn avx2_mul_agrees_with_scalar() {
        for &seed in &[0u64, 1, 42, P - 1, P / 2, 0xFFFFFFFF, (0x1234567890ABCDEFu64).wrapping_rem(P)] {
            let a = gf4([
                seed,
                (seed + 1) % P,
                (seed as u128 * 7 % P as u128) as u64,
                (seed ^ 0xABCD) % P,
            ]);
            let b = gf4([
                (seed as u128 * 3 % P as u128) as u64,
                (seed + 100) % P,
                P - 1,
                seed,
            ]);
            let r_avx = a.mul(&b).to_scalars();
            for (i, (r_val, (ai, bi))) in r_avx.iter().zip(a.0.iter().zip(b.0.iter())).enumerate() {
                let r_scalar = ai.mul(bi);
                assert_eq!(r_val.canonical(), r_scalar.canonical(),
                    "mul mismatch at idx {i}: a={} b={} avx={} scalar={}",
                    ai.canonical(), bi.canonical(),
                    r_val.canonical(), r_scalar.canonical());
            }
        }
    }

    #[test]
    fn avx2_mul_large_values() {
        let a = gf4([P - 2, P - 3, P / 2, 1]);
        let b = gf4([P - 2, P - 4, P / 3, P - 1]);
        let r_avx = a.mul(&b).to_scalars();
        for (i, (r_val, (ai, bi))) in r_avx.iter().zip(a.0.iter().zip(b.0.iter())).enumerate() {
            let r_scalar = ai.mul(bi);
            assert_eq!(r_val.canonical(), r_scalar.canonical(),
                "large mul mismatch at idx {i}");
        }
    }
}
