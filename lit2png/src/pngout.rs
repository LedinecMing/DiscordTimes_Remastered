//! PNG output helpers (guide §7.10).

use crate::lit::DecodedImage;
use crate::Result;
use image::{ImageBuffer, ImageFormat, Rgba, RgbaImage};

pub fn save_rgba(path: &std::path::Path, img: &DecodedImage) -> Result<()> {
    let mut im = RgbaImage::new(img.w, img.h);
    for y in 0..img.h {
        for x in 0..img.w {
            let i = (y * img.w + x) as usize;
            let a = img.alpha.as_ref().map(|a| a[i]).unwrap_or(255);
            im.put_pixel(x, y, Rgba([img.rgb[i * 3], img.rgb[i * 3 + 1], img.rgb[i * 3 + 2], a]));
        }
    }
    im.save_with_format(path, ImageFormat::Png)?;
    Ok(())
}

/// NEAREST ×`scale` upscale for tiny previews (UGS cursors etc.).
pub fn save_rgba_scaled(path: &std::path::Path, img: &DecodedImage, scale: u32) -> Result<()> {
    if scale <= 1 {
        return save_rgba(path, img);
    }
    let w = img.w * scale;
    let h = img.h * scale;
    let mut im = RgbaImage::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let sx = x / scale;
            let sy = y / scale;
            let i = (sy * img.w + sx) as usize;
            let a = img.alpha.as_ref().map(|a| a[i]).unwrap_or(255);
            im.put_pixel(x, y, Rgba([img.rgb[i * 3], img.rgb[i * 3 + 1], img.rgb[i * 3 + 2], a]));
        }
    }
    im.save_with_format(path, ImageFormat::Png)?;
    Ok(())
}

/// Save the alpha channel alone as a grayscale mask (for LIT files with an
/// alpha plane, alongside the RGBA main output).
pub fn save_alpha_mask(path: &std::path::Path, img: &DecodedImage) -> Result<()> {
    let alpha = img.alpha.as_ref().ok_or_else(|| crate::LitError::Other("no alpha".into()))?;
    let im: ImageBuffer<image::Luma<u8>, _> =
        ImageBuffer::from_raw(img.w, img.h, alpha.clone()).expect("alpha len == w*h");
    im.save_with_format(path, ImageFormat::Png)?;
    Ok(())
}
