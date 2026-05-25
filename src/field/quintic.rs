use crate::field::goldilocks::GoldilocksField;

/// Quintic extension field Fp5 = GF(p^5) over the Goldilocks prime.
///
/// The irreducible polynomial is x^5 = W where W = 3.
///
/// Represents an element as a polynomial of degree at most 4:
///   a₀ + a₁·x + a₂·x² + a₃·x³ + a₄·x⁴
///
/// Multiplication follows the schoolbook method with reduction x^5 → W.
#[derive(Debug, Clone, Copy)]
pub struct Fp5(pub [GoldilocksField; 5]);

impl PartialEq for Fp5 {
    fn eq(&self, other: &Self) -> bool {
        self.0[0].canonical() == other.0[0].canonical()
            && self.0[1].canonical() == other.0[1].canonical()
            && self.0[2].canonical() == other.0[2].canonical()
            && self.0[3].canonical() == other.0[3].canonical()
            && self.0[4].canonical() == other.0[4].canonical()
    }
}
impl Eq for Fp5 {}

impl zeroize::Zeroize for Fp5 {
    fn zeroize(&mut self) {
        for c in &mut self.0 {
            c.zeroize();
        }
    }
}

// ------------------------------------------------------------------
// Constants
// ------------------------------------------------------------------

/// W = 3, the constant from the irreducible polynomial x^5 = W.
const W: GoldilocksField = GoldilocksField(3);
/// 2W = 6, used in optimized squaring.
const W2: GoldilocksField = GoldilocksField(6);
/// FP5_DTH_ROOT = W^{(p-1)/5}, used in Frobenius automorphism.
const DTH_ROOT: GoldilocksField = GoldilocksField(1041288259238279555);

impl Fp5 {
    pub const ZERO: Self = Self([
        GoldilocksField(0), GoldilocksField(0), GoldilocksField(0),
        GoldilocksField(0), GoldilocksField(0),
    ]);
    pub const ONE: Self = Self([
        GoldilocksField(1), GoldilocksField(0), GoldilocksField(0),
        GoldilocksField(0), GoldilocksField(0),
    ]);

    // ------------------------------------------------------------------
    // Construction / helpers
    // ------------------------------------------------------------------

    pub fn from_array(arr: [GoldilocksField; 5]) -> Self {
        Self(arr)
    }

    pub const fn from_u64_arr(arr: [u64; 5]) -> Self {
        Self([
            GoldilocksField(arr[0]),
            GoldilocksField(arr[1]),
            GoldilocksField(arr[2]),
            GoldilocksField(arr[3]),
            GoldilocksField(arr[4]),
        ])
    }

    /// Embed a base field element.
    pub fn from_base(g: GoldilocksField) -> Self {
        Self([g, GoldilocksField::ZERO, GoldilocksField::ZERO, GoldilocksField::ZERO, GoldilocksField::ZERO])
    }

    pub fn is_zero(&self) -> bool {
        self.0[0].is_zero() && self.0[1].is_zero() && self.0[2].is_zero()
            && self.0[3].is_zero() && self.0[4].is_zero()
    }

    // ------------------------------------------------------------------
    // Serde
    // ------------------------------------------------------------------

    pub fn to_bytes_le(&self) -> [u8; 40] {
        let mut out = [0u8; 40];
        for i in 0..5 {
            out[i * 8..(i + 1) * 8].copy_from_slice(&self.0[i].to_bytes_le());
        }
        out
    }

    pub fn from_bytes_le(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != 40 {
            return Err("Fp5::from_bytes_le: expected 40 bytes");
        }
        let mut arr = [GoldilocksField::ZERO; 5];
        for i in 0..5 {
            arr[i] = GoldilocksField::from_bytes_le(&bytes[i * 8..(i + 1) * 8])
                .map_err(|e| {
                    let _ = e;
                    "non-canonical Goldilocks element in Fp5 bytes"
                })?;
        }
        Ok(Self(arr))
    }

    // ------------------------------------------------------------------
    // Component-wise add / sub / neg / double
    // ------------------------------------------------------------------

    pub fn add(&self, other: &Self) -> Self {
        Self([
            self.0[0].add(&other.0[0]),
            self.0[1].add(&other.0[1]),
            self.0[2].add(&other.0[2]),
            self.0[3].add(&other.0[3]),
            self.0[4].add(&other.0[4]),
        ])
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self([
            self.0[0].sub(&other.0[0]),
            self.0[1].sub(&other.0[1]),
            self.0[2].sub(&other.0[2]),
            self.0[3].sub(&other.0[3]),
            self.0[4].sub(&other.0[4]),
        ])
    }

    pub fn neg(&self) -> Self {
        Self([
            self.0[0].neg(),
            self.0[1].neg(),
            self.0[2].neg(),
            self.0[3].neg(),
            self.0[4].neg(),
        ])
    }

    pub fn double(&self) -> Self {
        self.add(self)
    }

    // ------------------------------------------------------------------
    // Mul — 25 base-field multiplications, reduced by x^5 = W
    // ------------------------------------------------------------------

    pub fn mul(&self, other: &Self) -> Self {
        let a = &self.0;
        let b = &other.0;

        let a0b0 = a[0].mul(&b[0]);
        let a1b4 = a[1].mul(&b[4]);
        let a2b3 = a[2].mul(&b[3]);
        let a3b2 = a[3].mul(&b[2]);
        let a4b1 = a[4].mul(&b[1]);
        let added = a1b4.add(&a2b3).add(&a3b2).add(&a4b1);
        let muld = W.mul(&added);
        let c0 = a0b0.add(&muld);

        let a0b1 = a[0].mul(&b[1]);
        let a1b0 = a[1].mul(&b[0]);
        let a2b4 = a[2].mul(&b[4]);
        let a3b3 = a[3].mul(&b[3]);
        let a4b2 = a[4].mul(&b[2]);
        let added = a2b4.add(&a3b3).add(&a4b2);
        let muld = W.mul(&added);
        let c1 = a0b1.add(&a1b0).add(&muld);

        let a0b2 = a[0].mul(&b[2]);
        let a1b1 = a[1].mul(&b[1]);
        let a2b0 = a[2].mul(&b[0]);
        let a3b4 = a[3].mul(&b[4]);
        let a4b3 = a[4].mul(&b[3]);
        let added = a3b4.add(&a4b3);
        let muld = W.mul(&added);
        let c2 = a0b2.add(&a1b1).add(&a2b0).add(&muld);

        let a0b3 = a[0].mul(&b[3]);
        let a1b2 = a[1].mul(&b[2]);
        let a2b1 = a[2].mul(&b[1]);
        let a3b0 = a[3].mul(&b[0]);
        let a4b4 = a[4].mul(&b[4]);
        let muld = W.mul(&a4b4);
        let c3 = a0b3.add(&a1b2).add(&a2b1).add(&a3b0).add(&muld);

        let a0b4 = a[0].mul(&b[4]);
        let a1b3 = a[1].mul(&b[3]);
        let a2b2 = a[2].mul(&b[2]);
        let a3b1 = a[3].mul(&b[1]);
        let a4b0 = a[4].mul(&b[0]);
        let c4 = a0b4.add(&a1b3).add(&a2b2).add(&a3b1).add(&a4b0);

        Self([c0, c1, c2, c3, c4])
    }

    // ------------------------------------------------------------------
    // Square — ~15 muls vs 25 for general mul
    // ------------------------------------------------------------------

    pub fn square(&self) -> Self {
        let a = &self.0;

        let a0s = a[0].square();
        let a1a4 = a[1].mul(&a[4]);
        let a2a3 = a[2].mul(&a[3]);
        let added = a1a4.add(&a2a3);
        let muld = W2.mul(&added);
        let c0 = a0s.add(&muld);

        let a0d = a[0].double();
        let a0da1 = a0d.mul(&a[1]);
        let a2a4dw = W2.mul(&a[2].mul(&a[4]));
        let a3a3w = W.mul(&a[3].square());
        let c1 = a0da1.add(&a2a4dw).add(&a3a3w);

        let a0da2 = a0d.mul(&a[2]);
        let a1sq = a[1].square();
        let a4a3dw = W2.mul(&a[4].mul(&a[3]));
        let c2 = a0da2.add(&a1sq).add(&a4a3dw);

        let a1d = a[1].double();
        let a0da3 = a0d.mul(&a[3]);
        let a1da2 = a1d.mul(&a[2]);
        let a4sqw = W.mul(&a[4].square());
        let c3 = a0da3.add(&a1da2).add(&a4sqw);

        let a0da4 = a0d.mul(&a[4]);
        let a1da3 = a1d.mul(&a[3]);
        let a2sq = a[2].square();
        let c4 = a0da4.add(&a1da3).add(&a2sq);

        Self([c0, c1, c2, c3, c4])
    }

    pub fn scalar_mul(&self, s: &GoldilocksField) -> Self {
        Self([
            self.0[0].mul(s),
            self.0[1].mul(s),
            self.0[2].mul(s),
            self.0[3].mul(s),
            self.0[4].mul(s),
        ])
    }

    /// Raises this element to the power 2^power by repeated squaring.
    pub fn exp_power_of_2(&self, power: usize) -> Self {
        let mut res = *self;
        for _ in 0..power {
            res = res.square();
        }
        res
    }

    // ------------------------------------------------------------------
    // Frobenius automorphism
    // For GF(p^5): Frob(a) = a^p, where a^p = Σ a_i · (W^{(p-1)/5})^i · x^i
    // Frobenius(x Element) Element` + `RepeatedFrobenius`
    // ------------------------------------------------------------------

    /// Compute z0^k for k = 0..4 where z0 = DTH_ROOT^count.
    fn powers(z0: GoldilocksField) -> [GoldilocksField; 5] {
        let mut p = [GoldilocksField::ZERO; 5];
        p[0] = GoldilocksField::ONE;
        for i in 1..5 {
            p[i] = p[i - 1].mul(&z0);
        }
        p
    }

    /// Frobenius automorphism applied `count` times.
    pub fn repeated_frobenius(&self, count: usize) -> Self {
        let count = count % 5;
        if count == 0 {
            return *self;
        }

        // z0 = DTH_ROOT^count
        let mut z0 = DTH_ROOT;
        for _ in 1..count {
            z0 = z0.mul(&DTH_ROOT);
        }

        let z_powers = Self::powers(z0);
        let mut res = [GoldilocksField::ZERO; 5];
        for i in 0..5 {
            res[i] = self.0[i].mul(&z_powers[i]);
        }
        Self(res)
    }

    /// Frobenius automorphism applied once.
    pub fn frobenius(&self) -> Self {
        self.repeated_frobenius(1)
    }

    // ------------------------------------------------------------------
    // Inverse — via Frobenius identity
    // InverseOrZero(a Element) Element`
    // ------------------------------------------------------------------

    pub fn inverse_or_zero(&self) -> Self {
        if self.is_zero() {
            return Self::ZERO;
        }

        // d = Frob(a)
        let d = self.frobenius();
        // e = d · Frob(d)
        let e = d.mul(&d.frobenius());
        // f = e · Frob²(e)
        let f = e.mul(&e.repeated_frobenius(2));

        // g = a₀·f₀ + W·(a₁·f₄ + a₂·f₃ + a₃·f₂ + a₄·f₁)
        let a = &self.0;
        let ff = &f.0;
        let a0b0 = a[0].mul(&ff[0]);
        let a1b4 = a[1].mul(&ff[4]);
        let a2b3 = a[2].mul(&ff[3]);
        let a3b2 = a[3].mul(&ff[2]);
        let a4b1 = a[4].mul(&ff[1]);
        let added = a1b4.add(&a2b3).add(&a3b2).add(&a4b1);
        let muld = W.mul(&added);
        let g = a0b0.add(&muld);

        // f · g⁻¹
        f.scalar_mul(&g.inverse())
    }

    // ------------------------------------------------------------------
    // Square root — matches Go `Sqrt` + `CanonicalSqrt`
    // ------------------------------------------------------------------

    pub fn sqrt(&self) -> Option<Self> {
        // Step 1: v = self^(2^31)
        let v = self.exp_power_of_2(31);
        // Step 2: d = self · v^(2^32) · v⁻¹
        let v32 = v.exp_power_of_2(32);
        let vinv = v.inverse_or_zero();
        let d = self.mul(&v32).mul(&vinv);
        // Step 3: e = Frob(d · Frob²(d))
        let dr = d.repeated_frobenius(2);
        let e = d.mul(&dr).frobenius();
        // Step 4: f = e²
        let f = e.square();
        // Step 5: g = self₀·f₀ + W·(self₁·f₄ + self₂·f₃ + self₃·f₂ + self₄·f₁)
        let x1f4 = self.0[1].mul(&f.0[4]);
        let x2f3 = self.0[2].mul(&f.0[3]);
        let x3f2 = self.0[3].mul(&f.0[2]);
        let x4f1 = self.0[4].mul(&f.0[1]);
        let added = x1f4.add(&x2f3).add(&x3f2).add(&x4f1);
        let muld = W.mul(&added);
        let x0f0 = self.0[0].mul(&f.0[0]);
        let gg = x0f0.add(&muld);
        // Step 6: s = sqrt(g) in base field
        let s = gg.sqrt()?;
        // Step 7: result = s_fp5 · e⁻¹
        let einv = e.inverse_or_zero();
        Some(Fp5::from_base(s).mul(&einv))
    }

    /// Sign function: returns true if Sgn0(self) = 0.
    /// Sgn0(x) = 0 iff the LSB of the first non-zero limb is 0.
    pub fn sgn0(&self) -> bool {
        let mut sign = false;
        let mut zero = true;
        for c in &self.0 {
            let sign_i = (c.canonical() & 1) == 0;
            let zero_i = c.is_zero();
            sign = sign || (zero && sign_i);
            zero = zero && zero_i;
        }
        sign
    }

    /// Canonical square root: returns (canonical_sqrt, found).
    /// The canonical sqrt has Sgn0 = false, i.e. its first non-zero coefficient's LSB is 0.
    pub fn canonical_sqrt(&self) -> (Self, bool) {
        match self.sqrt() {
            Some(sqrt_x) => {
                if sqrt_x.sgn0() {
                    (sqrt_x.neg(), true)
                } else {
                    (sqrt_x, true)
                }
            }
            None => (Self::ZERO, false),
        }
    }

    /// Legendre symbol: returns 0, 1 or -1 (as Goldilocks field element).
    pub fn legendre(&self) -> GoldilocksField {
        let frob1 = self.frobenius();
        let frob2 = frob1.frobenius();

        let frob1tfrob2 = frob1.mul(&frob2);
        let frob2frob1tfrob2 = frob1tfrob2.repeated_frobenius(2);

        let xr_ext = self.mul(&frob1tfrob2).mul(&frob2frob1tfrob2);
        let xr = xr_ext.0[0]; // extract base field element

        let xr31 = xr.exp_power_of_2(31);
        let xr31_inv = xr31.inverse_or_zero();

        let xr63 = xr31.exp_power_of_2(32);
        xr63.mul(&xr31_inv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f5(arr: [u64; 5]) -> Fp5 {
        Fp5::from_u64_arr(arr)
    }

    #[test]
    fn zero_is_zero() {
        assert!(Fp5::ZERO.is_zero());
        assert!(!Fp5::ONE.is_zero());
    }

    #[test]
    fn add_sub() {
        let a = f5([1, 2, 3, 4, 5]);
        let b = f5([10, 20, 30, 40, 50]);
        let c = a.add(&b);
        assert_eq!(c.0[0].canonical(), 11);
        assert_eq!(c.0[4].canonical(), 55);
        let d = c.sub(&b);
        assert_eq!(d, a);
    }

    #[test]
    fn add_wrap() {
        let p = GoldilocksField::MODULUS;
        let a = f5([p - 1, p - 1, p - 1, p - 1, p - 1]);
        let b = f5([2, 2, 2, 2, 2]);
        let c = a.add(&b);
        assert_eq!(c.0[0].canonical(), 1);
        assert_eq!(c.0[4].canonical(), 1);
    }

    #[test]
    fn neg_double() {
        let a = f5([1, 2, 3, 4, 5]);
        assert!(a.add(&a.neg()).is_zero());
        let d = a.double();
        assert_eq!(d, a.add(&a));
    }

    #[test]
    fn mul_one() {
        let a = f5([42, 100, 200, 300, 400]);
        assert_eq!(a.mul(&Fp5::ONE), a);
    }

    #[test]
    fn mul_zero() {
        let a = f5([42, 100, 200, 300, 400]);
        assert!(a.mul(&Fp5::ZERO).is_zero());
    }

    #[test]
    fn mul_commutative() {
        let a = f5([3, 0, 0, 0, 0]);
        let b = f5([5, 0, 0, 0, 0]);
        assert_eq!(a.mul(&b), b.mul(&a));
        assert_eq!(a.mul(&b).0[0].canonical(), 15);
    }

    #[test]
    fn mul_x() {
        // x * x = x^2, represented as (0, 0, 1, 0, 0)
        let x = f5([0, 1, 0, 0, 0]);
        let x2 = x.mul(&x);
        assert_eq!(x2.0[2].canonical(), 1);
        assert_eq!(x2.0[0].canonical(), 0);
        assert_eq!(x2.0[1].canonical(), 0);
    }

    #[test]
    fn mul_x4_x() {
        // x^4 * x = x^5 = W = 3 (mod x^5 = W)
        let x = f5([0, 1, 0, 0, 0]);
        let x2 = x.mul(&x);          // (0, 0, 1, 0, 0)
        let x4 = x2.mul(&x2);        // (0, 0, 0, 0, 1)
        let x5 = x4.mul(&x);
        assert_eq!(x5.0[0].canonical(), 3);
    }

    #[test]
    fn square_matches_mul() {
        let a = f5([2, 3, 5, 7, 11]);
        let sq = a.square();
        let prod = a.mul(&a);
        assert_eq!(sq, prod, "square != mul*self");
    }

    #[test]
    fn square_random() {
        // Test a few random-like values
        let vals = [
            [1, 2, 3, 4, 5],
            [100, 200, 300, 400, 500],
            [GoldilocksField::MODULUS - 1, 0, 0, 0, 0],
            [0, 0, 0, 0, 1],
            [42, 99, 127, 256, 512],
        ];
        for v in vals {
            let a = f5(v);
            assert_eq!(a.square(), a.mul(&a), "square mismatch for {v:?}");
        }
    }

    #[test]
    fn inverse_cancel() {
        let a = f5([2, 3, 5, 7, 11]);
        let inv = a.inverse_or_zero();
        assert_eq!(a.mul(&inv), Fp5::ONE, "a * a^-1 != 1");
    }

    #[test]
    fn inverse_one() {
        assert_eq!(Fp5::ONE.inverse_or_zero(), Fp5::ONE);
    }

    #[test]
    fn frobenius_five_times() {
        let a = f5([2, 3, 5, 7, 11]);
        // Frob^5 = identity in GF(p^5)
        let frob5 = a.repeated_frobenius(5);
        assert_eq!(frob5, a);
    }

    #[test]
    fn scalar_mul() {
        let a = f5([1, 2, 3, 4, 5]);
        let two = GoldilocksField::from_canonical_u64(2);
        let b = a.scalar_mul(&two);
        assert_eq!(b, a.add(&a));
    }

    #[test]
    fn sqrt_square_roundtrip() {
        // a = 4 embedded in Fp5 — sqrt should be ±2
        let a = Fp5::from_base(GoldilocksField::from_canonical_u64(4));
        let s = a.sqrt().unwrap();
        assert_eq!(s.square(), a);
    }

    #[test]
    fn serde_roundtrip() {
        let a = f5([1, 2, 3, 4, 5]);
        let bytes = a.to_bytes_le();
        assert_eq!(bytes.len(), 40);
        let b = Fp5::from_bytes_le(&bytes).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn sgn0_test() {
        // ONE = (1,0,0,0,0): first limb 1 has LSB=1, not zero → Sgn0 returns false
        assert!(!Fp5::ONE.sgn0());
        // (2,0,0,0,0): first limb 2 has LSB=0, not zero → Sgn0 returns true
        assert!(f5([2, 0, 0, 0, 0]).sgn0());
        // ZERO: all limbs zero. Go Sgn0 returns true for zero (the canonical LSB=0 of the first
        // limb which is zero, and zero=true so sign=true).
        assert!(Fp5::ZERO.sgn0());
    }

    #[test]
    fn legendre_square() {
        // A square element should have Legendre symbol 1
        let a = f5([2, 3, 5, 7, 11]);
        let a_sq = a.square();
        let ls = a_sq.legendre();
        assert_eq!(ls.canonical(), 1);
    }
}
