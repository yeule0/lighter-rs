use crate::field::quintic::Fp5;

/// Affine point on ECgFP5 in (x, u) fractional coordinates.
/// Used internally for window precomputation.
#[derive(Debug, Clone, Copy)]
pub struct AffinePoint {
    pub x: Fp5,
    pub u: Fp5,
}

impl AffinePoint {
    pub const NEUTRAL: Self = Self {
        x: Fp5::ZERO,
        u: Fp5::ZERO,
    };

    pub fn new(x: Fp5, u: Fp5) -> Self {
        Self { x, u }
    }

    /// Convert to projective point.
    pub fn to_point(&self) -> super::point::Point {
        super::point::Point {
            x: self.x,
            z: Fp5::ONE,
            u: self.u,
            t: Fp5::ONE,
        }
    }

    /// Negate in place.
    pub fn set_neg(&mut self) {
        self.u = self.u.neg();
    }

    /// Lookup a point in a window.
    pub fn lookup(win: &[AffinePoint], k: i32) -> Self {
        let sign = (k >> 31) as u32;
        let ka = ((k as u32) ^ sign).wrapping_sub(sign);
        let km1 = ka.wrapping_sub(1);

        let mut x = Fp5::ZERO;
        let mut u = Fp5::ZERO;
        for (i, w) in win.iter().enumerate() {
            let m = km1.wrapping_sub(i as u32);
            let mask = ((m | (!m).wrapping_add(1)) >> 31).wrapping_sub(1);
            let c = mask as u64;
            if c != 0 {
                x = w.x;
                u = w.u;
            }
        }

        let c = sign as u64 | ((sign as u64) << 32);
        if c != 0 {
            u = u.neg();
        }

        Self { x, u }
    }

    /// Variable-time lookup.
    pub fn lookup_var_time(win: &[AffinePoint], k: i32) -> Self {
        if k == 0 {
            Self::NEUTRAL
        } else if k > 0 {
            win[k as usize - 1]
        } else {
            let mut res = win[(-k) as usize - 1];
            res.set_neg();
            res
        }
    }
}
