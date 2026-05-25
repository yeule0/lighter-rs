#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[cfg(target_arch = "x86_64")]
use crate::field::goldilocks::GoldilocksField;

/// Eight Goldilocks field elements packed into one ZMM register for AVX-512 SIMD.
///
/// Provides 8-way parallelism, operand masking via `k` registers, and 32 ZMM
#[derive(Debug, Clone, Copy)]
pub struct GoldilocksFieldx8(pub [GoldilocksField; 8]);

#[cfg(not(target_arch = "x86_64"))]
impl GoldilocksFieldx8 {
    pub fn from_scalars(a: [GoldilocksField; 8]) -> Self { Self(a) }
    pub fn to_scalars(&self) -> [GoldilocksField; 8] { self.0 }
    pub fn add(&self, o: &Self) -> Self { scalar_8way(self, o, GoldilocksField::add) }
    pub fn sub(&self, o: &Self) -> Self { scalar_8way(self, o, GoldilocksField::sub) }
    pub fn mul(&self, o: &Self) -> Self { scalar_8way(self, o, GoldilocksField::mul) }
}

#[cfg(target_arch = "x86_64")]
impl GoldilocksFieldx8 {
    pub fn from_scalars(a: [GoldilocksField; 8]) -> Self { Self(a) }
    pub fn to_scalars(&self) -> [GoldilocksField; 8] { self.0 }

    pub fn add(&self, o: &Self) -> Self {
        if crate::has_avx512f() { unsafe { self.add_avx512(o) } }
        else { scalar_8way(self, o, GoldilocksField::add) }
    }

    pub fn sub(&self, o: &Self) -> Self {
        if crate::has_avx512f() { unsafe { self.sub_avx512(o) } }
        else { scalar_8way(self, o, GoldilocksField::sub) }
    }

    pub fn mul(&self, o: &Self) -> Self {
        if crate::has_avx512f() { unsafe { self.mul_avx512(o) } }
        else { scalar_8way(self, o, GoldilocksField::mul) }
    }

    /// AVX-512 8-way addition.
    ///
    /// # Safety
    /// Requires AVX-512F. Caller must verify `is_x86_feature_detected!("avx512f")`.
    #[target_feature(enable = "avx512f")]
    pub unsafe fn add_avx512(&self, other: &Self) -> Self {
        let eps = _mm512_set1_epi64(GoldilocksField::EPSILON as i64);
        let a = load_u512(&self.0);
        let b = load_u512(&other.0);

        let sum = _mm512_add_epi64(a, b);
        let over: __mmask8 = _mm512_cmplt_epu64_mask(sum, a);
        let corr = _mm512_maskz_mov_epi64(over, eps);
        let sum = _mm512_add_epi64(sum, corr);
        let over2: __mmask8 = _mm512_cmplt_epu64_mask(sum, corr);
        let corr2 = _mm512_maskz_mov_epi64(over2, eps);
        Self(store_u512(_mm512_add_epi64(sum, corr2)))
    }

    /// AVX-512 8-way subtraction.
    ///
    /// # Safety
    /// Requires AVX-512F. Caller must verify `is_x86_feature_detected!("avx512f")`.
    #[target_feature(enable = "avx512f")]
    pub unsafe fn sub_avx512(&self, other: &Self) -> Self {
        let eps = _mm512_set1_epi64(GoldilocksField::EPSILON as i64);
        let a = load_u512(&self.0);
        let b = load_u512(&other.0);

        let diff = _mm512_sub_epi64(a, b);
        let borrow: __mmask8 = _mm512_cmplt_epu64_mask(a, b);
        let corr = _mm512_maskz_mov_epi64(borrow, eps);
        let diff = _mm512_sub_epi64(diff, corr);
        let borrow2: __mmask8 = _mm512_cmplt_epu64_mask(diff, corr);
        let corr2 = _mm512_maskz_mov_epi64(borrow2, eps);
        Self(store_u512(_mm512_sub_epi64(diff, corr2)))
    }

    /// AVX-512 8-way multiply (32-bit decomposition).
    ///
    /// # Safety
    /// Requires AVX-512F. Caller must verify `is_x86_feature_detected!("avx512f")`.
    #[target_feature(enable = "avx512f")]
    pub unsafe fn mul_avx512(&self, other: &Self) -> Self {
        let a = load_u512(&self.0);
        let b = load_u512(&other.0);
        let eps = _mm512_set1_epi64(GoldilocksField::EPSILON as i64);
        let lo32 = _mm512_set1_epi64(0x00000000FFFFFFFFu64 as i64);
        let one = _mm512_set1_epi64(1);

        let a_lo = _mm512_and_epi64(a, lo32);
        let a_hi = _mm512_srli_epi64(a, 32);
        let b_lo = _mm512_and_epi64(b, lo32);
        let b_hi = _mm512_srli_epi64(b, 32);

        let p00 = _mm512_mul_epu32(a_lo, b_lo);
        let p01 = _mm512_mul_epu32(a_lo, b_hi);
        let p10 = _mm512_mul_epu32(a_hi, b_lo);
        let p11 = _mm512_mul_epu32(a_hi, b_hi);

        let (mid, mc) = add512c(p01, p10);
        let mid_lo = _mm512_and_epi64(mid, lo32);
        let mid_hi = _mm512_srli_epi64(mid, 32);

        let mid_lo_s = _mm512_slli_epi64(mid_lo, 32);
        let (lo, lc) = add512c(p00, mid_lo_s);

        let mut hi = _mm512_add_epi64(p11, mid_hi);
        hi = _mm512_add_epi64(hi, _mm512_maskz_mov_epi64(lc, one));
        hi = _mm512_add_epi64(hi, _mm512_slli_epi64(_mm512_maskz_mov_epi64(mc, one), 32));

        let ho: __mmask8 = _mm512_cmplt_epu64_mask(hi, p11);

        let hi_hi = _mm512_srli_epi64(hi, 32);
        let hi_lo = _mm512_and_epi64(hi, lo32);

        let (t0, borrow) = sub512b(lo, hi_hi);
        let t0 = _mm512_sub_epi64(t0, _mm512_maskz_mov_epi64(borrow, eps));

        let t1 = _mm512_mul_epu32(hi_lo, eps);
        let (t2, carry) = add512c(t0, t1);
        let t2 = _mm512_add_epi64(t2, _mm512_maskz_mov_epi64(carry, eps));

        let extra = _mm512_maskz_mov_epi64(ho, eps);
        let (t2, fc) = add512c(t2, extra);
        let t2 = _mm512_add_epi64(t2, _mm512_maskz_mov_epi64(fc, eps));

        Self(store_u512(t2))
    }
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn load_u512(arr: &[GoldilocksField; 8]) -> __m512i {
    let vals: [u64; 8] = core::array::from_fn(|i| arr[i].0);
    _mm512_loadu_si512(vals.as_ptr() as *const _)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
unsafe fn store_u512(v: __m512i) -> [GoldilocksField; 8] {
    let mut out = [0u64; 8];
    _mm512_storeu_si512(out.as_mut_ptr() as *mut _, v);
    core::array::from_fn(|i| GoldilocksField(out[i]))
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn add512c(a: __m512i, b: __m512i) -> (__m512i, __mmask8) {
    let sum = _mm512_add_epi64(a, b);
    (sum, _mm512_cmplt_epu64_mask(sum, a))
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx512f")]
unsafe fn sub512b(a: __m512i, b: __m512i) -> (__m512i, __mmask8) {
    let diff = _mm512_sub_epi64(a, b);
    (diff, _mm512_cmplt_epu64_mask(a, b))
}

#[inline]
fn scalar_8way(
    a: &GoldilocksFieldx8, b: &GoldilocksFieldx8,
    op: fn(&GoldilocksField, &GoldilocksField) -> GoldilocksField,
) -> GoldilocksFieldx8 {
    let mut out = [GoldilocksField::ZERO; 8];
    for (o, (ai, bi)) in out.iter_mut().zip(a.0.iter().zip(b.0.iter())) {
        *o = op(ai, bi);
    }
    GoldilocksFieldx8(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::goldilocks::GoldilocksField;

    fn gf(x: u64) -> GoldilocksField { GoldilocksField::from_canonical_u64(x) }

    fn gf8(a: [u64; 8]) -> GoldilocksFieldx8 {
        GoldilocksFieldx8(core::array::from_fn(|i| gf(a[i])))
    }

    const P: u64 = GoldilocksField::MODULUS;

    #[test]
    fn add_basic() {
        let r = gf8([1, 2, 3, 4, 5, 6, 7, 8])
            .add(&gf8([10, 20, 30, 40, 50, 60, 70, 80]))
            .to_scalars();
        for (i, v) in r.iter().enumerate() {
            assert_eq!(v.canonical(), 11 * (i as u64 + 1));
        }
    }

    #[test]
    fn add_max() {
        let r = gf8([P - 1; 8]).add(&gf8([P - 1; 8])).to_scalars();
        for v in r.iter() { assert_eq!(v.canonical(), P - 2); }
    }

    #[test]
    fn sub_underflow() {
        let r = gf8([0; 8]).sub(&gf8([1, 2, 3, 4, 5, 6, 7, 8])).to_scalars();
        for (i, v) in r.iter().enumerate() {
            assert_eq!(v.canonical(), P - (i as u64 + 1));
        }
    }

    #[test]
    fn agrees_with_scalar_add() {
        let a = gf8([P - 100, 42, P / 2, 1, 100, P - 1, 7, 13]);
        let b = gf8([P - 50, 99, P / 4, P - 1, 50, 1, 42, P - 13]);
        let r = a.add(&b).to_scalars();
        for (i, (rv, (ai, bi))) in r.iter().zip(a.0.iter().zip(b.0.iter())).enumerate() {
            assert_eq!(rv.canonical(), ai.add(bi).canonical(), "idx {i}");
        }
    }

    #[test]
    fn agrees_with_scalar_mul() {
        let a = gf8([2, 3, 5, 7, 11, P - 2, P / 3, 1]);
        let b = gf8([4, 5, 6, 8, 13, P - 2, P / 5, P - 1]);
        let r = a.mul(&b).to_scalars();
        for (i, (rv, (ai, bi))) in r.iter().zip(a.0.iter().zip(b.0.iter())).enumerate() {
            assert_eq!(rv.canonical(), ai.mul(bi).canonical(), "idx {i}");
        }
    }
}
