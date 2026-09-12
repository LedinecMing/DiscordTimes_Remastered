//! Post-processing of decoded planes (guide §4): deblocking filter on chroma,
//! ×2 chroma upsampling for 4:2:0 files.

/// Deblocking applied to U/V planes BEFORE upsampling (guide §4.1).
///
/// Block-row boundaries first, then block-column boundaries. `d = (a-b)` rounded
/// toward zero with `+3 >> 2`; `a -= d; b += d`.
pub fn deblock_plane(p: &mut [u8], nbx: usize, nby: usize) {
    let bw = nbx * 8;
    let bh = nby * 8;
    let blend = |a: i32, b: i32| -> i32 {
        let diff = a - b;
        if diff < 0 {
            (diff + 3) >> 2
        } else {
            diff >> 2
        }
    };
    // horizontal seams between block rows (all x)
    for row in 1..nby {
        for x in 0..bw {
            let o = (row * 8 - 1) * bw + x;
            let o2 = o + bw;
            let d = blend(p[o] as i32, p[o2] as i32);
            p[o] = (p[o] as i32 - d).clamp(0, 255) as u8;
            p[o2] = (p[o2] as i32 + d).clamp(0, 255) as u8;
        }
    }
    // vertical seams between block columns (all y)
    for col in 1..nbx {
        for y in 0..bh {
            let o = y * bw + col * 8 - 1;
            let o2 = o + 1;
            let d = blend(p[o] as i32, p[o2] as i32);
            p[o] = (p[o] as i32 - d).clamp(0, 255) as u8;
            p[o2] = (p[o2] as i32 + d).clamp(0, 255) as u8;
        }
    }
}

/// ×2 chroma upsampling (guide §4.2): replicate, then [1,2,1]/4 smoothing over X,
/// then over Y. `prev` is the pre-overwrite value; edges stay replicated.
pub fn upsample2x(src: &[u8], w: usize, h: usize) -> Vec<u8> {
    let w2 = w * 2;
    let h2 = h * 2;
    let mut dst = vec![0u8; w2 * h2];
    // 1) replication
    for y in 0..h {
        for x in 0..w {
            let v = src[y * w + x];
            let dy = y * 2;
            let dx = x * 2;
            dst[dy * w2 + dx] = v;
            dst[dy * w2 + dx + 1] = v;
            dst[(dy + 1) * w2 + dx] = v;
            dst[(dy + 1) * w2 + dx + 1] = v;
        }
    }
    // 2) smooth over X (left to right, prev = value before overwrite)
    for y in 0..h2 {
        let mut prev_old = dst[y * w2];
        for x in 1..w2 - 1 {
            let cur = dst[y * w2 + x] as i32;
            let next = dst[y * w2 + x + 1] as i32;
            dst[y * w2 + x] = ((2 * cur + prev_old as i32 + next + 2) >> 2) as u8;
            prev_old = cur as u8;
        }
    }
    // 3) smooth over Y
    for x in 0..w2 {
        let mut prev_old = dst[x];
        for y in 1..h2 - 1 {
            let cur = dst[y * w2 + x] as i32;
            let next = dst[(y + 1) * w2 + x] as i32;
            dst[y * w2 + x] = ((2 * cur + prev_old as i32 + next + 2) >> 2) as u8;
            prev_old = cur as u8;
        }
    }
    dst
}
