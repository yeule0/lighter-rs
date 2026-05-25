use crate::field::quintic::Fp5;
use crate::curve::affine::AffinePoint;
use crate::curve::scalar::Scalar;
use std::sync::LazyLock;

/// Comb table for fast fixed-base scalar multiplication.
///
/// W=8, D=40 columns, 256 precomputed affine points.
/// Built lazily on first use. Eliminates per-sign precomputation
/// and MDouble overhead: each column uses 1 double + 1 affine add.
pub const COMB_W: usize = 8;
pub const COMB_D: usize = 319usize.div_ceil(COMB_W); // 40
const COMB_TABLE_SIZE: usize = 1 << COMB_W;

/// Precomputed comb table for the generator G.
/// T[a] = Σ_{i with bit(a,i)=1} 2^(i·D) · G
pub static GENERATOR_COMB: LazyLock<Vec<AffinePoint>> = LazyLock::new(|| {
    // Base points: B[i] = 2^(i*D) * G, projective
    let mut bases = vec![Point::NEUTRAL; COMB_W];
    bases[0] = Point::GENERATOR;
    for i in 1..COMB_W {
        bases[i] = bases[i - 1].m_double(COMB_D as u32);
    }

    // Build table via Gray-code addition: T[a] = T[a_without_lsb] + B[lsb]
    let mut table_proj = vec![Point::NEUTRAL; COMB_TABLE_SIZE];
    for a in 1..COMB_TABLE_SIZE {
        let lsb = a.trailing_zeros() as usize;
        let prev = a ^ (1 << lsb);
        table_proj[a] = table_proj[prev].add(&bases[lsb]);
    }

    batch_to_affine(&table_proj)
});

/// ECgFP5 elliptic curve point in projective coordinates (x, z, u, t).
///
/// Uses complete addition formulas (10M). Curve equation:
///   y² = x·(x² + A·x + B)  in Fp5
/// where A = 2, B = (0, 263, 0, 0, 0).
///
#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: Fp5,
    pub z: Fp5,
    pub u: Fp5,
    pub t: Fp5,
}

// ------------------------------------------------------------------
// Curve constants
// ------------------------------------------------------------------

/// A = Fp5(2, 0, 0, 0, 0)
pub const A: Fp5 = Fp5([
    crate::field::goldilocks::GoldilocksField(2),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
]);

/// B1 = 263
const B1: u64 = 263;
/// B = Fp5(0, 263, 0, 0, 0)
pub const B: Fp5 = Fp5([
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(B1),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
]);
pub const B_MUL2: Fp5 = Fp5([
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(2 * B1),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
]);
pub const B_MUL4: Fp5 = Fp5([
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(4 * B1),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
]);
const B_MUL16: Fp5 = Fp5([
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(16 * B1),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
    crate::field::goldilocks::GoldilocksField(0),
]);

impl Point {
    /// Neutral element N = (0, 1, 0, 1). The group law is P ⊕ Q = P + Q + N.
    pub const NEUTRAL: Self = Self {
        x: Fp5::ZERO,
        z: Fp5::ONE,
        u: Fp5::ZERO,
        t: Fp5::ONE,
    };

    /// Generator point G in projective coordinates.
    pub const GENERATOR: Self = Self {
        x: Fp5::from_u64_arr([
            12883135586176881569, 4356519642755055268, 5248930565894896907,
            2165973894480315022, 2448410071095648785,
        ]),
        z: Fp5::ONE,
        u: Fp5::ONE,
        t: Fp5::from_u64_arr([4, 0, 0, 0, 0]),
    };

    pub fn is_neutral(&self) -> bool {
        self.u.is_zero()
    }

    // ------------------------------------------------------------------
    // Equality u1*t2 == u2*t1
    // ------------------------------------------------------------------

    pub fn equals(&self, other: &Self) -> bool {
        self.u.mul(&other.t) == other.u.mul(&self.t)
    }

    // ------------------------------------------------------------------
    // Encode / Decode
    // ------------------------------------------------------------------

    /// Encode point as Fp5 element: t * u^(-1)
    pub fn encode(&self) -> Fp5 {
        self.t.mul(&self.u.inverse_or_zero())
    }

    /// Decode an Fp5 element into a curve point.
    /// Solves x² - (w² - A)x + B = 0 for x, picks the non-square root.
    /// Returns (point, true) on success.
    pub fn decode(w: &Fp5) -> (Self, bool) {
        let e = w.square().sub(&A);
        let delta = e.square().sub(&B_MUL4);
        let (r, c) = delta.canonical_sqrt();

        let x1 = e.add(&r).mul(&Fp5::from_u64_arr([2, 0, 0, 0, 0]).inverse_or_zero());
        let x2 = e.sub(&r).mul(&Fp5::from_u64_arr([2, 0, 0, 0, 0]).inverse_or_zero());

        let mut x = x1;
        let x1_legendre = x1.legendre();
        if x1_legendre.canonical() == 1 {
            x = x2;
        }

        if !c {
            x = Fp5::ZERO;
        }
        let z = Fp5::ONE;
        let mut u = Fp5::ONE;
        if !c {
            u = Fp5::ZERO;
        }
        let mut t = *w;
        if !c {
            t = Fp5::ONE;
        }

        if c || w.is_zero() {
            (Self { x, z, u, t }, true)
        } else {
            (Self::NEUTRAL, false)
        }
    }

    // ------------------------------------------------------------------
    // Add — complete 10M formula.
    // ------------------------------------------------------------------

    pub fn add(&self, rhs: &Self) -> Self {
        let x1 = &self.x;
        let z1 = &self.z;
        let u1 = &self.u;
        let t1 = &self.t;

        let x2 = &rhs.x;
        let z2 = &rhs.z;
        let u2 = &rhs.u;
        let t2 = &rhs.t;

        // t1 = x1 * x2
        let tt1 = x1.mul(x2);
        // t2 = z1 * z2
        let tt2 = z1.mul(z2);
        // t3 = u1 * u2
        let tt3 = u1.mul(u2);
        // t4 = t1 * t2
        let tt4 = t1.mul(t2);
        // t5 = (x1+z1)*(x2+z2) - t1 - t2
        let tt5 = x1.add(z1).mul(&x2.add(z2)).sub(&tt1.add(&tt2));
        // t6 = (u1+t1)*(u2+t2) - t3 - t4
        let tt6 = u1.add(t1).mul(&u2.add(t2)).sub(&tt3.add(&tt4));
        // t7 = t1 + t2 * B
        let tt7 = tt1.add(&tt2.mul(&B));
        // t8 = t4 * t7
        let tt8 = tt4.mul(&tt7);
        // t9 = t3 * (t5 * B_MUL2 + t7.double())
        let tt9 = tt3.mul(&tt5.mul(&B_MUL2).add(&tt7.double()));
        // t10 = (t4 + t3.double()) * (t5 + t7)
        let tt10 = tt4.add(&tt3.double()).mul(&tt5.add(&tt7));

        Self {
            x: tt10.sub(&tt8).mul(&B),
            z: tt8.sub(&tt9),
            u: tt6.mul(&tt2.mul(&B).sub(&tt1)),
            t: tt8.add(&tt9),
        }
    }

    // ------------------------------------------------------------------
    // Double.  4M+5S
    // ------------------------------------------------------------------

    pub fn double(&self) -> Self {
        let mut p = *self;
        p.set_double();
        p
    }

    pub fn set_double(&mut self) {
        let x = &self.x;
        let z = &self.z;
        let u = &self.u;
        let t = &self.t;

        let t1 = z.mul(t);
        let t2 = t1.mul(t);
        let x1 = t2.square();
        let z1 = t1.mul(u);
        let t3 = u.square();
        let w1 = t2.sub(&t3.mul(&x.add(z).double()));
        let t4 = z1.square();

        self.x = t4.mul(&B_MUL4);
        self.z = w1.square();
        self.u = w1.add(&z1).square().sub(&t4.add(&self.z));
        self.t = x1.double().sub(&t4.mul(&Fp5::from_u64_arr([4, 0, 0, 0, 0])).add(&self.z));
    }

    // ------------------------------------------------------------------
    // MDouble — repeated doubling
    // ------------------------------------------------------------------

    pub fn m_double(&self, n: u32) -> Self {
        let mut p = *self;
        p.set_m_double(n);
        p
    }

    pub fn set_m_double(&mut self, n: u32) {
        if n == 0 {
            return;
        }
        if n == 1 {
            self.set_double();
            return;
        }

        let x0 = self.x;
        let z0 = self.z;
        let u0 = self.u;
        let t0 = self.t;

        let t1 = z0.mul(&t0);
        let t2 = t1.mul(&t0);
        let x1_acc = t2.square();
        let z1_acc = t1.mul(&u0);
        let t3 = u0.square();
        let w1 = t2.sub(&x0.add(&z0).double().mul(&t3));
        let t4 = w1.square();
        let t5 = z1_acc.square();
        let mut x = t5.square().mul(&B_MUL16);
        let mut w = x1_acc.double().sub(&t5.mul(&Fp5::from_u64_arr([4, 0, 0, 0, 0])).add(&t4));
        let mut z = w1.add(&z1_acc).square().sub(&t4.add(&t5));

        for _ in 2..n {
            let t1_n = z.square();
            let t2_n = t1_n.square();
            let t3_n = w.square();
            let t4_n = t3_n.square();
            let t5_n = w.add(&z).square().sub(&t1_n.add(&t3_n));
            z = t5_n.mul(&x.add(&t1_n).double().sub(&t3_n));
            x = t2_n.mul(&t4_n).mul(&B_MUL16);
            w = Fp5::ZERO.sub(&t4_n.add(&t2_n.mul(&B_MUL4.sub(&Fp5::from_u64_arr([4, 0, 0, 0, 0])))));
        }

        let t1_f = w.square();
        let t2_f = z.square();
        let t3_f = w.add(&z).square().sub(&t1_f.add(&t2_f));
        let w1_f = t1_f.sub(&x.add(&t2_f).double());

        self.x = t3_f.square().mul(&B);
        self.z = w1_f.square();
        self.u = t3_f.mul(&w1_f);
        self.t = t1_f.double().mul(&t1_f.sub(&t2_f.double())).sub(&self.z);
    }

    // ------------------------------------------------------------------
    // AddAffine — mixed addition. 8M
    // ------------------------------------------------------------------

    pub fn add_affine(&self, rhs: &AffinePoint) -> Self {
        let x1 = &self.x;
        let z1 = &self.z;
        let u1 = &self.u;
        let t1 = &self.t;
        let x2 = &rhs.x;
        let u2 = &rhs.u;

        let tt1 = x1.mul(x2);
        let tt2 = *z1;
        let tt3 = u1.mul(u2);
        let tt4 = *t1;
        let tt5 = x1.add(&x2.mul(z1));
        let tt6 = u1.add(&u2.mul(t1));
        let tt7 = tt1.add(&tt2.mul(&B));
        let tt8 = tt4.mul(&tt7);
        let tt9 = tt3.mul(&tt5.mul(&B_MUL2).add(&tt7.double()));
        let tt10 = tt4.add(&tt3.double()).mul(&tt5.add(&tt7));

        Self {
            x: tt10.sub(&tt8).mul(&B),
            u: tt6.mul(&tt2.mul(&B).sub(&tt1)),
            z: tt8.sub(&tt9),
            t: tt8.add(&tt9),
        }
    }

    // ------------------------------------------------------------------
    // Scalar multiplication — window method with WINDOW=5
    // Scalar multiplication — comb method.
    // ------------------------------------------------------------------

    const WINDOW: usize = 5;
    const WIN_SIZE: usize = 1 << (Self::WINDOW - 1);

    /// General scalar multiplication — window method.
    pub fn mul(&self, scalar: &Scalar) -> Self {
        let win = self.make_window_affine();
        let mut digits = [0i32; (319 + Self::WINDOW) / Self::WINDOW];
        scalar.recode_signed(&mut digits, Self::WINDOW as i32);

        let mut p = AffinePoint::lookup_var_time(&win, digits[digits.len() - 1]).to_point();
        for &d in digits[..digits.len() - 1].iter().rev() {
            p.set_m_double(Self::WINDOW as u32);
            let lookup = AffinePoint::lookup(&win, d);
            p = p.add_affine(&lookup);
        }
        p
    }

    /// Fast fixed-base k*G via precomputed comb table (W=7, 128 points).
    /// 46 doubles + 46 affine adds. Eliminates per-sign precomputation.
    pub fn mul_generator(scalar: &Scalar) -> Self {
        let table = &*GENERATOR_COMB;
        let limbs = scalar.0;
        let mut p = Self::NEUTRAL;
        for j in (0..COMB_D).rev() {
            p = p.double();
            let mut v: usize = 0;
            for i in 0..COMB_W {
                let bit_pos = j + i * COMB_D;
            if bit_pos < 320
                && (limbs[bit_pos / 64] >> (bit_pos % 64)) & 1 == 1
            {
                v |= 1 << i;
            }
            }
            if v > 0 {
                p = p.add_affine(&table[v]);
            }
        }
        p
    }

    // ------------------------------------------------------------------
    // Window precomputation — matches Go MakeWindowAffine + BatchToAffine
    // ------------------------------------------------------------------

    pub fn make_window_affine(&self) -> Vec<AffinePoint> {
        let mut tmp = vec![Self::NEUTRAL; Self::WIN_SIZE];
        tmp[0] = *self;
        for i in 1..Self::WIN_SIZE {
            if (i & 1) == 0 {
                tmp[i] = tmp[i - 1].add(self);
            } else {
                tmp[i] = tmp[i >> 1].double();
            }
        }
        batch_to_affine(&tmp)
    }
}

// ------------------------------------------------------------------
// BatchToAffine — Montgomery inversion trick
// Montgomery batch inversion.
// ------------------------------------------------------------------

pub fn batch_to_affine(src: &[Point]) -> Vec<AffinePoint> {
    let n = src.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        let p = src[0];
        let m1 = p.z.mul(&p.t).inverse_or_zero();
        return vec![AffinePoint::new(
            p.x.mul(&p.t).mul(&m1),
            p.u.mul(&p.z).mul(&m1),
        )];
    }

    let mut res = vec![AffinePoint::NEUTRAL; n];
    let mut m = src[0].z.mul(&src[0].t);
    for i in 1..n {
        let x = m;
        m = m.mul(&src[i].z);
        let u = m;
        m = m.mul(&src[i].t);
        res[i] = AffinePoint::new(x, u);
    }

    m = m.inverse_or_zero();

    for i in (1..n).rev() {
        res[i].u = src[i].u.mul(&res[i].u).mul(&m);
        m = m.mul(&src[i].t);
        res[i].x = src[i].x.mul(&res[i].x).mul(&m);
        m = m.mul(&src[i].z);
    }
    res[0].u = src[0].u.mul(&src[0].z).mul(&m);
    m = m.mul(&src[0].t);
    res[0].x = src[0].x.mul(&m);

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_is_neutral() {
        assert!(Point::NEUTRAL.is_neutral());
    }

    #[test]
    fn generator_not_neutral() {
        assert!(!Point::GENERATOR.is_neutral());
    }

    #[test]
    fn add_neutral() {
        let g = Point::GENERATOR;
        let gn = g.add(&Point::NEUTRAL);
        assert!(gn.equals(&g));
    }

    #[test]
    fn double_and_add() {
        let g = Point::GENERATOR;
        let g2 = g.double();
        let g2_add = g.add(&g);
        assert!(g2.equals(&g2_add));
    }

    #[test]
    fn encode_decode_roundtrip() {
        let g = Point::GENERATOR;
        let enc = g.encode();
        let (dec, ok) = Point::decode(&enc);
        assert!(ok);
        assert!(g.equals(&dec));
    }

    #[test]
    fn scalar_mul_generator() {
        let g = Point::GENERATOR;
        let g2 = g.mul(&Scalar::TWO);
        assert!(g2.equals(&g.double()));
    }
}
