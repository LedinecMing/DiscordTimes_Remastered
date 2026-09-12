//! UGS animation files of the UGRAPH engine (guide §3) plus the two atlas
//! container variants found in the 2003 game data (not covered by the guide's
//! simple frame-stream description — reverse-engineered and verified
//! byte-exact against the shipped files, see `tests/ugs.rs`).
//!
//! * **Frame stream** (Battle/Spells/Units/Windows .ugs): a bare sequence of
//!   `frame = [u16le W][u16le H][W*H u16le pixels]`. Pixel `v ^ 0xAAAA` is A8L8:
//!   high byte = luma, low byte = alpha.
//! * **Persones variant**: each frame's header is duplicated —
//!   `[u16 W][u16 H][u16 W][u16 H][pixels]` (8-byte header).
//! * **Objects/Icons/Items atlas variant**: file header
//!   `[u32 count][u32 ?]`, then `count` frames of
//!   `[u32 kind][u32 id][u32 W][u32 H=fw][u32 A=fh][u32 fx][u16 fw][u16 fh][fw*fh*u16 pixels]`
//!   (28-byte record header; `fx` is an engine-internal frame cookie). Some
//!   records carry a 4-byte shorter header (24) — both accepted; padding of
//!   zero/0xAA bytes between records is skipped.

use crate::lit::DecodedImage;
use crate::{LitError, Result};

pub struct UgsFrame {
    pub kind: Option<(u32, u32)>,
    pub image: DecodedImage,
}

fn a8l8_to_rgba(v: u16) -> [u8; 4] {
    let v2 = v ^ 0xAAAA;
    let luma = (v2 >> 8) as u8;
    let a = (v2 & 0xFF) as u8;
    [luma, luma, luma, a]
}

/// Decode a UGS file, auto-detecting the frame-stream vs atlas layout.
pub fn decode_ugs(buf: &[u8]) -> Result<Vec<UgsFrame>> {
    if let Some(frames) = try_decode_persones(buf) {
        return Ok(frames);
    }
    if let Some(frames) = try_decode_atlas(buf) {
        return Ok(frames);
    }
    decode_frame_stream(buf)
}

/// Plain frame stream: `[u16 W][u16 H][W*H u16 pixels]` repeated (guide §3).
pub fn decode_frame_stream(buf: &[u8]) -> Result<Vec<UgsFrame>> {
    let mut out = Vec::new();
    let mut off = 0usize;
    while off + 4 <= buf.len() {
        let w = u16::from_le_bytes(buf[off..off + 2].try_into().unwrap()) as usize;
        let h = u16::from_le_bytes(buf[off + 2..off + 4].try_into().unwrap()) as usize;
        if w == 0 || h == 0 || w > 4096 || h > 4096 {
            break;
        }
        let px = 4 + w * h * 2;
        if off + px > buf.len() {
            break; // truncated tail — stop at first incomplete frame
        }
        out.push(UgsFrame {
            kind: None,
            image: decode_pixels(&buf[off + 4..off + px], w, h),
        });
        off += px;
    }
    if out.is_empty() {
        return Err(LitError::Other("UGS: no decodable frames".into()));
    }
    Ok(out)
}

/// Persones variant: duplicated u16 header per frame.
fn try_decode_persones(buf: &[u8]) -> Option<Vec<UgsFrame>> {
    if buf.len() < 8 {
        return None;
    }
    let mut out = Vec::new();
    let mut off = 0usize;
    while off + 8 <= buf.len() {
        let (w, h) = (
            u16::from_le_bytes(buf[off..off + 2].try_into().unwrap()) as usize,
            u16::from_le_bytes(buf[off + 2..off + 4].try_into().unwrap()) as usize,
        );
        let (w2, h2) = (
            u16::from_le_bytes(buf[off + 4..off + 6].try_into().unwrap()) as usize,
            u16::from_le_bytes(buf[off + 6..off + 8].try_into().unwrap()) as usize,
        );
        if (w, h) != (w2, h2) || w == 0 || h == 0 || w > 4096 || h > 4096 {
            break;
        }
        let px = 8 + w * h * 2;
        if off + px > buf.len() {
            break;
        }
        out.push(UgsFrame {
            kind: None,
            image: decode_pixels(&buf[off + 8..off + px], w, h),
        });
        off += px;
    }
    if out.len() >= 2 && off == buf.len() {
        Some(out) // only accept when the whole file is consumed
    } else {
        None
    }
}

/// Atlas variant (Objects.ugs): tagged records from byte 0 —
/// `[u32 kind][u32 id][u32 W][u32 H=fw][u32 A=fh][u32 fx][u16 fw][u16 fh][pixels]`
/// (some records omit the `fx` word → 24-byte header instead of 28).
/// The first `kind` word is small (≤255) which distinguishes the layout from
/// frame streams whose leading u16 W,H pair makes it huge.
fn try_decode_atlas(buf: &[u8]) -> Option<Vec<UgsFrame>> {
    if buf.len() < 32 {
        return None;
    }
    let first_kind = u32::from_le_bytes(buf[0..4].try_into().unwrap());
    if first_kind > 255 {
        return None;
    }
    let mut out = Vec::new();
    let mut off = 0usize;
    let mut last_end = 0usize;
    let mut junk_since_last = 0usize;
    while off + 28 <= buf.len() {
        let kind = u32::from_le_bytes(buf[off..off + 4].try_into().unwrap());
        let id = u32::from_le_bytes(buf[off + 4..off + 8].try_into().unwrap());
        let w = u32::from_le_bytes(buf[off + 8..off + 12].try_into().unwrap());
        let h = u32::from_le_bytes(buf[off + 12..off + 16].try_into().unwrap());
        let a = u32::from_le_bytes(buf[off + 16..off + 20].try_into().unwrap());
        let mut parsed = None;
        for hdr_len in [24u32, 28u32] {
            let fwo = off + hdr_len as usize;
            if fwo + 4 > buf.len() {
                continue;
            }
            let fw = u16::from_le_bytes(buf[fwo..fwo + 2].try_into().unwrap()) as usize;
            let fh = u16::from_le_bytes(buf[fwo + 2..fwo + 4].try_into().unwrap()) as usize;
            if fw == 0 || fh == 0 || fw > 1024 || fh > 1024 {
                continue;
            }
            let end = fwo + 4 + fw * fh * 2;
            if end > buf.len() {
                continue;
            }
            // Geometry cross-check: H and A must repeat the frame dims.
            if h == fw as u32 && a == fh as u32 && w <= 4096 && kind <= 255 {
                parsed = Some((hdr_len, fw, fh, end));
                break;
            }
        }
        match parsed {
            Some((hdr_len, fw, fh, end)) => {
                let px_off = off + hdr_len as usize + 4;
                out.push(UgsFrame {
                    kind: Some((kind, id)),
                    image: decode_pixels(&buf[px_off..end], fw, fh),
                });
                last_end = end;
                junk_since_last = 0;
                off = end;
            }
            None => {
                // Skip inter-record padding/junk one byte at a time, but give up if
                // we scan too far without finding a valid record.
                junk_since_last += 1;
                if junk_since_last > 1 << 20 {
                    break;
                }
                off += 1;
            }
        }
    }
    if out.len() >= 2 && last_end + 64 >= buf.len() {
        Some(out)
    } else {
        None
    }
}

fn decode_pixels(px: &[u8], w: usize, h: usize) -> DecodedImage {
    let n = w * h;
    let mut rgb = vec![0u8; n * 3];
    let mut alpha = vec![0u8; n];
    for i in 0..n {
        let v = u16::from_le_bytes(px[i * 2..i * 2 + 2].try_into().unwrap());
        let [r, g, b, a] = a8l8_to_rgba(v);
        rgb[i * 3..i * 3 + 3].copy_from_slice(&[r, g, b]);
        alpha[i] = a;
    }
    DecodedImage { w: w as u32, h: h as u32, rgb, alpha: Some(alpha) }
}
