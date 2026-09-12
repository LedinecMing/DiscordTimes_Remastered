//! LIT decoder: container → RGBA `DecodedImage`.

pub mod color;
pub mod header;
pub mod idct;
pub mod plane;
pub mod post;
pub mod raw;

use crate::{LitError, Result};

#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub w: u32,
    pub h: u32,
    /// RGB, 3 bytes per pixel, row-major.
    pub rgb: Vec<u8>,
    /// Alpha 0..255 per pixel, if the source had an alpha plane.
    pub alpha: Option<Vec<u8>>,
}

/// Full LIT decode (DCT or RAW branch, guide §2/§4).
pub fn decode(buf: &[u8]) -> Result<DecodedImage> {
    let parsed = header::parse(buf)?;
    let hdr = parsed.header;
    if hdr.flags & header::FLAG_RAW != 0 {
        return raw::decode_raw(buf, hdr.width, hdr.height);
    }

    let g = parsed.geometry.expect("geometry present in DCT mode");
    let has_alpha = hdr.flags & header::FLAG_ALPHA != 0;
    let need = 16 + g.data_size(has_alpha);
    if buf.len() < need {
        return Err(LitError::Truncated(need, buf.len()));
    }

    let idct = idct::IdctTables::new();
    let y_seg = &buf[16..16 + g.y_size];
    let u_seg = &buf[16 + g.y_size..16 + g.y_size + g.c_size];
    let v_seg = &buf[16 + g.y_size + g.c_size..16 + g.y_size + 2 * g.c_size];
    let a_seg = if has_alpha {
        Some(&buf[16 + g.y_size + 2 * g.c_size..need])
    } else {
        None
    };

    let y_plane = plane::decode_plane(y_seg, g.ybx, g.yby, &idct)?;
    let mut u_plane = plane::decode_plane(u_seg, g.cbx, g.cby, &idct)?;
    let mut v_plane = plane::decode_plane(v_seg, g.cbx, g.cby, &idct)?;

    // Deblocking only on chroma, before upsampling (§4.1).
    post::deblock_plane(&mut u_plane, g.cbx, g.cby);
    post::deblock_plane(&mut v_plane, g.cbx, g.cby);

    let macro16 = hdr.flags & header::FLAG_MACRO16 != 0;
    let (u_full, v_full, plane_w) = if macro16 {
        (
            post::upsample2x(&u_plane, g.bw / 2, g.bh / 2),
            post::upsample2x(&v_plane, g.bw / 2, g.bh / 2),
            g.bw,
        )
    } else {
        (u_plane, v_plane, g.bw)
    };

    // Alpha plane decodes over the full luma geometry; engine uses plane>>3 (§4.4).
    let alpha_plane = a_seg
        .map(|seg| plane::decode_plane(seg, g.ybx, g.yby, &idct))
        .transpose()?
        .map(|p| p.iter().map(|&a| color::alpha5_to_a8(a)).collect::<Vec<u8>>());

    Ok(assemble_cropped(hdr.width, hdr.height, &y_plane, &u_full, &v_full, plane_w, alpha_plane.as_deref()))
}

fn assemble_cropped(
    w: u32,
    h: u32,
    y: &[u8],
    u: &[u8],
    v: &[u8],
    bw: usize,
    alpha: Option<&[u8]>,
) -> DecodedImage {
    let (w, h) = (w as usize, h as usize);
    let n = w * h;
    let mut rgb = vec![0u8; n * 3];
    let mut alpha_out = alpha.map(|_| vec![0u8; n]);
    for row in 0..h {
        for col in 0..w {
            let i = row * w + col;
            let p = row * bw + col;
            let [r, g, b] = color::yuv_to_rgb(y[p], u[p], v[p]);
            rgb[i * 3..i * 3 + 3].copy_from_slice(&[r, g, b]);
            if let Some(dst) = alpha_out.as_mut() {
                dst[i] = alpha.unwrap()[p];
            }
        }
    }
    DecodedImage { w: w as u32, h: h as u32, rgb, alpha: alpha_out }
}
