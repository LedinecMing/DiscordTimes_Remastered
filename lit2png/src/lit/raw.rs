//! RAW branch, FLAGS & 0x4 (guide §2.5): header + W*H YUV triplets, row-major.

use super::color::yuv_to_rgb;
use super::DecodedImage;
use crate::{LitError, Result};

pub fn decode_raw(buf: &[u8], w: u32, h: u32) -> Result<DecodedImage> {
    let n = w as usize * h as usize;
    let need = 16 + n * 3;
    if buf.len() < need {
        return Err(LitError::Truncated(need, buf.len()));
    }
    let mut rgb = vec![0u8; n * 3];
    for i in 0..n {
        let (y, u, v) = (buf[16 + i * 3], buf[16 + i * 3 + 1], buf[16 + i * 3 + 2]);
        let [r, g, b] = yuv_to_rgb(y, u, v);
        rgb[i * 3..i * 3 + 3].copy_from_slice(&[r, g, b]);
    }
    // The 2003 raw files carry no alpha segment (verified: files with flags 0x4/0x6
    // are exactly 16 + W*H*3 bytes long).
    Ok(DecodedImage { w, h, rgb, alpha: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_roundtrip() {
        // Synthetic file in TgaLitConverter style: W=2,H=2, flags word = 4.
        let mut buf = vec![0u8; 16];
        buf[0..4].copy_from_slice(b"LIT\0");
        buf[4..8].copy_from_slice(&2u32.to_le_bytes());
        buf[8..12].copy_from_slice(&2u32.to_le_bytes());
        buf[12..16].copy_from_slice(&4u32.to_le_bytes());
        // neutral gray pixel + primary colors
        let yuv: [[u8; 3]; 4] = [[128, 128, 128], [255, 128, 128], [128, 63, 128], [128, 128, 255]];
        for px in yuv {
            buf.extend_from_slice(&px);
        }
        let img = decode_raw(&buf, 2, 2).unwrap();
        assert_eq!(img.rgb[0..3], [128, 128, 128]);
        assert_eq!(img.rgb[3..6], [255, 255, 255]); // Y=255 -> white
        // Y=128,U=63: R=128, G=128+0.34414*65≈150, B=128+1.772*(-65)≈13
        assert_eq!(img.rgb[6..9], [128, 150, 13]);
        // Y=128,V=255: R=128+1.402*127≈306->255, G=128-0.71414*127≈37, B=128
        assert_eq!(img.rgb[9..12], [255, 37, 128]);
    }

    #[test]
    fn raw_truncated_is_error() {
        let buf = vec![0u8; 20];
        let err = decode_raw(&buf, 100, 100).unwrap_err();
        assert!(matches!(err, LitError::Truncated(_, _)));
    }
}
