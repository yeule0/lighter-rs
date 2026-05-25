use crate::field::quintic::Fp5;

/// AVX2: processes components 0-3 via GoldilocksFieldx4, component 4 via scalar.
/// The component 4 fallback is cheap — add/sub are single-cycle operations.
#[cfg(target_arch = "x86_64")]
mod avx2_impl {
    use super::Fp5;
    use crate::field::goldilocks_avx2::GoldilocksFieldx4;

    #[target_feature(enable = "avx2")]
    pub unsafe fn add_avx2(a: &Fp5, b: &Fp5) -> Fp5 {
        let ax4 = GoldilocksFieldx4([a.0[0], a.0[1], a.0[2], a.0[3]]);
        let bx4 = GoldilocksFieldx4([b.0[0], b.0[1], b.0[2], b.0[3]]);
        let rx4 = ax4.add(&bx4).to_scalars();
        Fp5([rx4[0], rx4[1], rx4[2], rx4[3], a.0[4].add(&b.0[4])])
    }

    #[target_feature(enable = "avx2")]
    pub unsafe fn sub_avx2(a: &Fp5, b: &Fp5) -> Fp5 {
        let ax4 = GoldilocksFieldx4([a.0[0], a.0[1], a.0[2], a.0[3]]);
        let bx4 = GoldilocksFieldx4([b.0[0], b.0[1], b.0[2], b.0[3]]);
        let rx4 = ax4.sub(&bx4).to_scalars();
        Fp5([rx4[0], rx4[1], rx4[2], rx4[3], a.0[4].sub(&b.0[4])])
    }
}

// ------------------------------------------------------------------
// Public dispatch wrappers
// ------------------------------------------------------------------

/// Optimized Fp5 addition: uses AVX2 when available.
pub fn add(a: &Fp5, b: &Fp5) -> Fp5 {
    #[cfg(target_arch = "x86_64")]
    {
        if crate::has_avx2() {
            return unsafe { avx2_impl::add_avx2(a, b) };
        }
    }
    a.add(b)
}

/// Optimized Fp5 subtraction: uses AVX2 when available.
pub fn sub(a: &Fp5, b: &Fp5) -> Fp5 {
    #[cfg(target_arch = "x86_64")]
    {
        if crate::has_avx2() {
            return unsafe { avx2_impl::sub_avx2(a, b) };
        }
    }
    a.sub(b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::goldilocks::GoldilocksField;
    use crate::field::quintic::Fp5;

    fn f5(arr: [u64; 5]) -> Fp5 {
        Fp5::from_u64_arr(arr)
    }

    #[test]
    fn add_agrees_with_scalar() {
        let a = f5([1, 2, 3, 4, 5]);
        let b = f5([10, 20, 30, 40, 50]);
        let r = add(&a, &b);
        assert_eq!(r, a.add(&b));
    }

    #[test]
    fn sub_agrees_with_scalar() {
        let a = f5([100, 200, 300, 400, 500]);
        let b = f5([1, 2, 3, 4, 5]);
        let r = sub(&a, &b);
        assert_eq!(r, a.sub(&b));
    }

    #[test]
    fn add_wrap() {
        let p = GoldilocksField::MODULUS;
        let a = f5([p - 1, p - 1, p - 1, p - 1, p - 1]);
        let b = f5([2, 2, 2, 2, 2]);
        let r = add(&a, &b);
        assert_eq!(r, a.add(&b));
        assert_eq!(r.0[0].canonical(), 1);
    }
}
