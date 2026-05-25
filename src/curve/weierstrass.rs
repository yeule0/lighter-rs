use crate::field::quintic::Fp5;
use crate::curve::scalar::Scalar;
use crate::curve::point::{self, A as A_ECGFP5};

/// Weierstrass point (x, y) on ECgFP5. Used for Schnorr verification.
#[derive(Debug, Clone, Copy)]
pub struct WeierstrassPoint {
    pub x: Fp5,
    pub y: Fp5,
    pub is_inf: bool,
}

/// A_WEIERSTRASS = (6148914689804861439, 263, 0, 0, 0) 
const A_W: Fp5 = Fp5::from_u64_arr([
    6148914689804861439, 263, 0, 0, 0,
]);

impl WeierstrassPoint {
    pub const GENERATOR: Self = Self {
        x: Fp5::from_u64_arr([
            11712523173042564207, 14090224426659529053, 13197813503519687414,
            16280770174934269299, 15998333998318935536,
        ]),
        y: Fp5::from_u64_arr([
            14639054205878357578, 17426078571020221072, 2548978194165003307,
            8663895577921260088, 9793640284382595140,
        ]),
        is_inf: false,
    };

    pub const NEUTRAL: Self = Self {
        x: Fp5::ZERO,
        y: Fp5::ZERO,
        is_inf: true,
    };

    // ------------------------------------------------------------------
    //
    // ------------------------------------------------------------------

    pub fn encode(&self) -> Fp5 {
        let a_div_3 = A_ECGFP5.mul(
            &Fp5::from_u64_arr([3, 0, 0, 0, 0]).inverse_or_zero(),
        );
        self.y.mul(&a_div_3.sub(&self.x).inverse_or_zero())
    }

    // ------------------------------------------------------------------
    //
    // ------------------------------------------------------------------

    pub fn decode(w: &Fp5) -> (Self, bool) {
        let e = w.square().sub(&A_ECGFP5);
        let delta = e.square().sub(&point::B_MUL4);
        let (r, success) = delta.canonical_sqrt();

        let two = Fp5::from_u64_arr([2, 0, 0, 0, 0]);
        let x1 = e.add(&r).mul(&two.inverse_or_zero());
        let x2 = e.sub(&r).mul(&two.inverse_or_zero());

        let mut x = x2;
        let x1_legendre = x1.legendre();
        if x1_legendre.canonical() == 1 {
            x = x1;
        }

        let y = Fp5::ZERO.sub(&w.mul(&x)); //

        if success {
            let a_div_3 = A_ECGFP5.mul(&two.add(&Fp5::ONE).inverse_or_zero()); // A/3
            x = x.add(&a_div_3);
        } else {
            x = Fp5::ZERO;
        }

        let is_inf = !success;
        if success || w.is_zero() {
            (Self { x, y, is_inf }, true)
        } else {
            (Self::NEUTRAL, false)
        }
    }

    // ------------------------------------------------------------------
    //
    // ------------------------------------------------------------------

    pub fn add(&self, q: &Self) -> Self {
        if self.is_inf { return *q; }
        if q.is_inf { return *self; }

        let x_same = self.x == q.x;
        let y_diff = self.y != q.y;

        let (lambda0, lambda1) = if x_same {
            (
                self.x.square().triple().add(&A_W),
                self.y.double(),
            )
        } else {
            (
                q.y.sub(&self.y),
                q.x.sub(&self.x),
            )
        };
        let lambda = lambda0.mul(&lambda1.inverse_or_zero());

        let x3 = lambda.square().sub(&self.x).sub(&q.x);
        let y3 = lambda.mul(&self.x.sub(&x3)).sub(&self.y);

        Self { x: x3, y: y3, is_inf: x_same && y_diff }
    }

    // ------------------------------------------------------------------
    //
    // ------------------------------------------------------------------

    pub fn double(&self) -> Self {
        if self.is_inf {
            return *self;
        }

        let lambda0 = self.x.square().triple().add(&A_W);
        let lambda1 = self.y.double();
        let lambda = lambda0.mul(&lambda1.inverse_or_zero());

        let x2 = lambda.square().sub(&self.x.double());
        let y2 = lambda.mul(&self.x.sub(&x2)).sub(&self.y);

        Self { x: x2, y: y2, is_inf: false }
    }

    // ------------------------------------------------------------------
    // PrecomputeWindow — for MulAdd2, windowBits=4
    // ------------------------------------------------------------------

    pub fn precompute_window(&self, window_bits: u32) -> Vec<Self> {
        assert!(window_bits >= 2);
        let n = 1usize << window_bits;
        let mut multiples = vec![Self::NEUTRAL; n];
        multiples[1] = *self;
        if n > 2 {
            multiples[2] = self.double();
            for i in 3..n {
                multiples[i] = self.add(&multiples[i - 1]);
            }
        }
        multiples
    }
}

// ------------------------------------------------------------------
// MulAdd2 — double scalar mul: s*G + e*P
// MulAdd2 — double scalar multiplication.
// ------------------------------------------------------------------

pub fn mul_add2(
    a: &WeierstrassPoint, b: &WeierstrassPoint,
    scalar_a: &Scalar, scalar_b: &Scalar,
) -> WeierstrassPoint {
    let a_win = a.precompute_window(4);
    let b_win = b.precompute_window(4);
    let a_limbs = scalar_a.split_to_4bit_limbs();
    let b_limbs = scalar_b.split_to_4bit_limbs();

    let num_limbs = a_limbs.len();
    let mut res = a_win[a_limbs[num_limbs - 1] as usize]
        .add(&b_win[b_limbs[num_limbs - 1] as usize]);

    for i in (0..num_limbs - 1).rev() {
        for _ in 0..4 {
            res = res.double();
        }
        res = res.add(
            &a_win[a_limbs[i] as usize].add(&b_win[b_limbs[i] as usize]),
        );
    }
    res
}

impl Fp5 {
    /// Triple: 3 * self.
    fn triple(&self) -> Self {
        let three = Fp5::from_u64_arr([3, 0, 0, 0, 0]);
        self.mul(&three)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_equals_add_self() {
        let g = WeierstrassPoint::GENERATOR;
        let g2 = g.double();
        let g2_add = g.add(&g);
        assert_eq!(g2.x, g2_add.x);
        assert_eq!(g2.y, g2_add.y);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let g = WeierstrassPoint::GENERATOR;
        let enc = g.encode();
        let (dec, ok) = WeierstrassPoint::decode(&enc);
        assert!(ok);
        assert_eq!(dec.x, g.x);
        assert_eq!(dec.y, g.y);
    }
}
