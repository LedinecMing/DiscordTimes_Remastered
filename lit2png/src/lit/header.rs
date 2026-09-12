//! LIT container header + geometry (guide §2).
//!
//! ```text
//! offset 0  char[4] magic = "LIT\0"
//! offset 4  u32le W
//! offset 8  u32le H
//! offset 12 u32le FLAGS  (0x2 macro16 4:2:0, 0x4 raw, 0x8 alpha plane)
//! offset 16 plane segments: [Y: Y_SIZE][U: C_SIZE][V: C_SIZE][A: Y_SIZE if 0x8]
//! ```

pub const FLAG_MACRO16: u32 = 0x2;
pub const FLAG_RAW: u32 = 0x4;
pub const FLAG_ALPHA: u32 = 0x8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LitHeader {
    pub width: u32,
    pub height: u32,
    pub flags: u32,
}

/// Padded block geometry of a DCT-mode file (guide §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Geometry {
    pub bw: usize,
    pub bh: usize,
    pub ybx: usize,
    pub yby: usize,
    pub cbx: usize,
    pub cby: usize,
    pub y_size: usize,
    pub c_size: usize,
    /// Alpha segment stride — always the full luma geometry (§2.3, DownCorner check).
    pub alpha_size: usize,
}

impl Geometry {
    /// Total size of the data area after the 16-byte header.
    pub fn data_size(&self, has_alpha: bool) -> usize {
        self.y_size + 2 * self.c_size + if has_alpha { self.alpha_size } else { 0 }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ParsedHeader {
    pub header: LitHeader,
    pub geometry: Option<Geometry>,
    /// Expected total file size in bytes (header + all segments); `None` for raw files
    /// with unknown trailing data.
    pub expected_size: Option<usize>,
}

pub fn parse(buf: &[u8]) -> crate::Result<ParsedHeader> {
    if buf.len() < 16 || &buf[0..4] != b"LIT\0" {
        return Err(crate::LitError::BadMagic);
    }
    let w = u32::from_le_bytes(buf[4..8].try_into().unwrap());
    let h = u32::from_le_bytes(buf[8..12].try_into().unwrap());
    let flags = u32::from_le_bytes(buf[12..16].try_into().unwrap());
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return Err(crate::LitError::BadDims(w, h));
    }

    if flags & FLAG_RAW != 0 {
        // Raw mode: W*H YUV triplets after the header. Files written by the community
        // tool also zero the flag word entirely; both variants carry W*H*3 bytes.
        let need = 16 + w as usize * h as usize * 3;
        return Ok(ParsedHeader {
            header: LitHeader { width: w, height: h, flags },
            geometry: None,
            expected_size: Some(need),
        });
    }

    let macro16 = flags & FLAG_MACRO16 != 0;
    let (bw, bh, cbx, cby) = if macro16 {
        let bw = (w as usize + 15) / 16 * 16;
        let bh = (h as usize + 15) / 16 * 16;
        (bw, bh, bw / 16, bh / 16)
    } else {
        let bw = (w as usize + 7) / 8 * 8;
        let bh = (h as usize + 7) / 8 * 8;
        (bw, bh, bw / 8, bh / 8)
    };
    let y_size = bw * bh + 128;
    let c_size = if macro16 { bw * bh / 4 + 128 } else { bw * bh + 128 };
    let geometry = Geometry {
        bw,
        bh,
        ybx: bw / 8,
        yby: bh / 8,
        cbx,
        cby,
        y_size,
        c_size,
        // Alpha steps by the full luma geometry (verified byte-exact on DownCorner
        // and all 86 alpha files of the 2003 set; guide §2.3 / §9.9).
        alpha_size: y_size,
    };
    let has_alpha = flags & FLAG_ALPHA != 0;
    let expected = 16 + geometry.data_size(has_alpha);
    Ok(ParsedHeader {
        header: LitHeader { width: w, height: h, flags },
        geometry: Some(geometry),
        expected_size: Some(expected),
    })
}
