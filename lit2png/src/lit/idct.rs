//! Float IDCT with the precomputed 64×64 basis (guide §5, §7.6).
//!
//! Slot `c = v*8 + u` is natural row-major: `u` (horizontal frequency) pairs with
//! pixel `x`, `v` pairs with `y`. Swapping the pairing transposes the image —
//! guarded by the slot-probe test (guide §6.1).

pub struct IdctTables {
    /// `bas[c*64 + p]`: contribution of coefficient slot `c` to pixel `p = y*8+x`.
    bas: Vec<f64>,
}

impl Default for IdctTables {
    fn default() -> Self {
        Self::new()
    }
}

impl IdctTables {
    pub fn new() -> Self {
        let pi = std::f64::consts::PI;
        let cu = |k: usize| if k == 0 { std::f64::consts::FRAC_1_SQRT_2 } else { 1.0 };
        let cos = |k: usize, i: usize| ((2 * i + 1) as f64 * k as f64 * pi / 16.0).cos();
        let mut bas = vec![0f64; 64 * 64];
        for v in 0..8 {
            for u in 0..8 {
                let c = v * 8 + u;
                for y in 0..8 {
                    for x in 0..8 {
                        let p = y * 8 + x;
                        bas[c * 64 + p] = 0.25 * cu(u) * cu(v) * cos(u, x) * cos(v, y);
                    }
                }
            }
        }
        Self { bas }
    }

    pub fn idct2d(&self, coef: &[f64; 64]) -> [f64; 64] {
        let mut out = [0f64; 64];
        for p in 0..64 {
            let mut s = 0f64;
            for c in 0..64 {
                s += coef[c] * self.bas[c * 64 + p];
            }
            out[p] = s;
        }
        out
    }
}
