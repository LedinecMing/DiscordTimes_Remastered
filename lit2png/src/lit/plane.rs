//! Plane segment decoder (guide §2.4, §7.5).
//!
//! Segment layout: `[u8 64 Q][u8 64 SP][i8 pool: 64 columns × T values]` where
//! `T = nbx*nby` blocks. Value of slot `j` in block `b` is
//! `Q[j] * (i8) pool[SP[j]*T + b]` — note the SIGNED pool bytes and that
//! quantization is a MULTIPLICATION (guide §9.1–9.2).

use super::idct::IdctTables;
use crate::{LitError, Result};

pub fn decode_plane(seg: &[u8], nbx: usize, nby: usize, idct: &IdctTables) -> Result<Vec<u8>> {
    let t = nbx * nby;
    let need = 128 + 64 * t;
    if seg.len() < need {
        return Err(LitError::Truncated(need, seg.len()));
    }
    let q: [u8; 64] = seg[0..64].try_into().unwrap();
    let sp: [u8; 64] = seg[64..128].try_into().unwrap();
    let pool = &seg[128..need];

    let mut out = vec![0u8; nbx * 8 * nby * 8];
    let mut coef = [0f64; 64];
    for b in 0..t {
        for j in 0..64 {
            let v = pool[sp[j] as usize * t + b] as i8 as f64;
            coef[j] = v * q[j] as f64;
        }
        let px = idct.idct2d(&coef);
        let (br, bc) = (b / nbx, b % nbx);
        let row0 = br * 8 * nbx * 8 + bc * 8;
        for y in 0..8 {
            let base = row0 + y * nbx * 8;
            for x in 0..8 {
                // Engine rounding (guide §5): the engine IDCT emits an 8×-scaled
                // value that is rounded to int, then (v+4)>>3. This reproduces
                // the reference slot-probe vector [17,15,10,4] exactly where a
                // plain round(f) would give [17,15,10,3].
                // Clamp to bytes immediately: post-processing operates on clamped
                // plane values (guide §9.5).
                let scaled = (px[y * 8 + x] * 8.0).round() as i32;
                let v = ((scaled + 4) >> 3).clamp(0, 255) as u8;
                out[base + x] = v;
            }
        }
    }
    Ok(out)
}

/// DC column of the pool (column 0, row-major blocks) — used by calibration tests.
pub fn dc_column(seg: &[u8], nbx: usize, nby: usize) -> Result<Vec<i8>> {
    let t = nbx * nby;
    let need = 128 + 64 * t;
    if seg.len() < need {
        return Err(LitError::Truncated(need, seg.len()));
    }
    let pool = &seg[128..need];
    Ok((0..t).map(|b| pool[b] as i8).collect())
}

/// RMS per pool column — column = frequency rank (guide §6.3.4).
pub fn column_rms(seg: &[u8], nbx: usize, nby: usize) -> Result<Vec<f64>> {
    let t = nbx * nby;
    let need = 128 + 64 * t;
    if seg.len() < need {
        return Err(LitError::Truncated(need, seg.len()));
    }
    let pool = &seg[128..need];
    let mut rms = Vec::with_capacity(64);
    for c in 0..64 {
        let mut s = 0f64;
        for b in 0..t {
            let v = pool[c * t + b] as i8 as f64;
            s += v * v;
        }
        rms.push((s / t as f64).sqrt());
    }
    Ok(rms)
}
