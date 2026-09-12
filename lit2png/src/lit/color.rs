//! YUV → RGB conversion and RGBA assembly (guide §4.3, §4.4).

use super::DecodedImage;

pub fn yuv_to_rgb(y: u8, u: u8, v: u8) -> [u8; 3] {
    let (y, u, v) = (y as f64, u as f64, v as f64);
    let cl = |x: f64| x.round().clamp(0.0, 255.0) as u8;
    [
        cl(y + 1.402 * (v - 128.0)),
        cl(y - 0.34414 * (u - 128.0) - 0.71414 * (v - 128.0)),
        cl(y + 1.772 * (u - 128.0)),
    ]
}

/// 5-bit alpha plane byte → 8-bit PNG alpha. The engine keeps `plane >> 3`
/// (0..31); we rescale by *255/31 (guide §4.4).
#[inline]
pub fn alpha5_to_a8(a5: u8) -> u8 {
    ((a5 >> 3) as u16 * 255 / 31) as u8
}

/// Assemble the final `DecodedImage` from post-processed planes.
///
/// Planes are padded to BW×BH; the output crops to W×H. Indexing is row-major
/// top-down (verified against DC maps; no vertical flip needed — guide §7.7).
pub fn assemble(
    w: u32,
    h: u32,
    y_plane: &[u8],
    u_plane: &[u8],
    v_plane: &[u8],
    alpha: Option<&[u8]>,
) -> DecodedImage {
    let (w, h) = (w as usize, h as usize);
    let bw = if w % 8 == 0 { w } else { w / 8 * 8 + 8 }; // caller passes exact BW via plane width; recomputed defensively
    let _ = bw;
    let plane_w = {
        // plane width is inferred from the plane length vs height is unknown here;
        // callers pass planes sized BW*BH, so width = len / bh where bh = padded h.
        let bh = if h % 8 == 0 { h } else { h / 8 * 8 + 8 };
        y_plane.len() / bh.max(1)
    };
    let n = w * h;
    let mut rgb = vec![0u8; n * 3];
    let mut alpha_out = alpha.map(|_| vec![0u8; n]);
    for row in 0..h {
        for col in 0..w {
            let i = row * w + col;
            let p = row * plane_w + col;
            let [r, g, b] = yuv_to_rgb(y_plane[p], u_plane[p], v_plane[p]);
            rgb[i * 3] = r;
            rgb[i * 3 + 1] = g;
            rgb[i * 3 + 2] = b;
            if let (Some(a_out), Some(a_in)) = (&mut alpha_out, alpha) {
                a_out[i] = a_in[p];
            }
        }
    }
    DecodedImage { w: w as u32, h: h as u32, rgb, alpha: alpha_out }
}
