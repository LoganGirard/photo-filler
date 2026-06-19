use image::{DynamicImage, GenericImageView, ImageReader, RgbaImage};
use imageproc::distance_transform::Norm;
use imageproc::morphology::dilate;
use std::io::Cursor;
use std::path::Path;

use crate::page::mm_to_px;

pub struct Sticker {
    pub source: RgbaImage,
    pub source_width_mm: f32,
    pub source_height_mm: f32,
    pub source_dpi: u32,
}

impl Sticker {
    pub fn load_from_path(path: &Path, source_dpi: u32) -> Result<Self, String> {
        let img = ImageReader::open(path)
            .map_err(|e| format!("open: {e}"))?
            .with_guessed_format()
            .map_err(|e| format!("format: {e}"))?
            .decode()
            .map_err(|e| format!("decode: {e}"))?;
        Self::from_dynamic_image(img, source_dpi)
    }

    pub fn load_from_bytes(bytes: &[u8], source_dpi: u32) -> Result<Self, String> {
        let img = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(|e| format!("format: {e}"))?
            .decode()
            .map_err(|e| format!("decode: {e}"))?;
        Self::from_dynamic_image(img, source_dpi)
    }

    fn from_dynamic_image(img: DynamicImage, source_dpi: u32) -> Result<Self, String> {
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        let source_width_mm = w as f32 * 25.4 / source_dpi as f32;
        let source_height_mm = h as f32 * 25.4 / source_dpi as f32;
        Ok(Self {
            source: rgba,
            source_width_mm,
            source_height_mm,
            source_dpi,
        })
    }

    pub fn width_mm(&self) -> f32 {
        self.source_width_mm
    }

    pub fn height_mm(&self) -> f32 {
        self.source_height_mm
    }

    /// Resample the sticker to the target DPI assuming the original mm size.
    pub fn resampled(&self, target_dpi: u32) -> RgbaImage {
        self.resampled_scaled(target_dpi, 1.0)
    }

    /// Resample the sticker so its on-page size is `source_mm × scale_factor`
    /// at `target_dpi`. `scale_factor` of 0.5 halves the sticker's mm dims;
    /// 2.0 doubles them.
    pub fn resampled_scaled(&self, target_dpi: u32, scale_factor: f32) -> RgbaImage {
        let scale = scale_factor.max(0.01);
        let target_w = mm_to_px(self.source_width_mm * scale, target_dpi);
        let target_h = mm_to_px(self.source_height_mm * scale, target_dpi);
        let (sw, sh) = self.source.dimensions();
        if target_w == sw && target_h == sh {
            return self.source.clone();
        }
        let dyn_img = DynamicImage::ImageRgba8(self.source.clone());
        dyn_img
            .resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3)
            .to_rgba8()
    }
}

/// Binary mask: 255 = opaque (occupied), 0 = transparent.
pub type Mask = image::GrayImage;

/// Build a binary mask from an RGBA image using an alpha threshold.
pub fn mask_from_rgba(img: &RgbaImage, alpha_threshold: u8) -> Mask {
    let (w, h) = img.dimensions();
    let mut out = image::GrayImage::new(w, h);
    for (x, y, p) in img.enumerate_pixels() {
        let v = if p[3] > alpha_threshold { 255u8 } else { 0u8 };
        out.put_pixel(x, y, image::Luma([v]));
    }
    out
}

/// Dilate a binary mask by `pixels` pixels using L∞ norm. Loops in chunks of
/// 255 to stay within imageproc's u8 kernel parameter.
pub fn dilate_mask(mask: &Mask, pixels: u32) -> Mask {
    if pixels == 0 {
        return mask.clone();
    }
    let mut out = mask.clone();
    let mut remaining = pixels;
    while remaining > 0 {
        let step = remaining.min(255) as u8;
        out = dilate(&out, Norm::LInf, step);
        remaining -= step as u32;
    }
    out
}

/// Tightly crop a mask to its non-zero bounding box. Returns the cropped mask and
/// the (x_offset, y_offset) of the crop within the original.
pub fn crop_to_content(mask: &Mask) -> Option<(Mask, u32, u32)> {
    let (w, h) = mask.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x = 0u32;
    let mut max_y = 0u32;
    let mut any = false;
    for y in 0..h {
        for x in 0..w {
            if mask.get_pixel(x, y)[0] > 0 {
                if x < min_x {
                    min_x = x;
                }
                if y < min_y {
                    min_y = y;
                }
                if x > max_x {
                    max_x = x;
                }
                if y > max_y {
                    max_y = y;
                }
                any = true;
            }
        }
    }
    if !any {
        return None;
    }
    let cw = max_x - min_x + 1;
    let ch = max_y - min_y + 1;
    let mut out = image::GrayImage::new(cw, ch);
    for y in 0..ch {
        for x in 0..cw {
            out.put_pixel(x, y, *mask.get_pixel(min_x + x, min_y + y));
        }
    }
    Some((out, min_x, min_y))
}

/// Count opaque pixels in a mask (used for coverage calculations).
pub fn mask_area_px(mask: &Mask) -> u64 {
    let mut n = 0u64;
    for p in mask.pixels() {
        if p[0] > 0 {
            n += 1;
        }
    }
    n
}
