use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use image::{DynamicImage, GrayImage, ImageFormat, Luma, Rgba, RgbaImage};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::io::Cursor;

use crate::page::{mm_to_px, Orientation, PaperSize};
use crate::shape::{
    crop_to_content, dilate_mask, mask_area_px, mask_from_rgba, Mask, Sticker,
};

/// Internal raster resolution for the packer. Keeps collision checks fast
/// regardless of the user's output DPI; placements are stored in mm so the
/// renderer can re-rasterize at the chosen output DPI. 40 DPI ≈ 0.64 mm per
/// pixel — small enough to keep packing snappy; combined with supersampled
/// rotation (no NN holes) and rotate-then-dilate ordering, this is enough
/// for the margin band to hold around the rotated outline.
const PACK_DPI: u32 = 40;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackRequest {
    pub paper: PaperSize,
    pub orientation: Orientation,
    pub dpi: u32,
    pub page_margin_mm: f32,
    pub inter_copy_margin_mm: f32,
    pub max_count: Option<u32>,
    pub rotation_step_deg: f32,
    pub quality: Quality,
    /// Multiplier on the sticker's mm dimensions. 1.0 = original; 0.5 = half size.
    #[serde(default = "default_scale")]
    pub scale_factor: f32,
}

fn default_scale() -> f32 {
    1.0
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    Fast,
    Balanced,
    Thorough,
}

impl Quality {
    /// Raster grid stride in pixels — smaller = better packing, slower.
    pub fn scan_stride(&self) -> u32 {
        match self {
            Quality::Fast => 8,
            Quality::Balanced => 4,
            Quality::Thorough => 2,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placement {
    pub x_mm: f32,
    pub y_mm: f32,
    pub rotation_deg: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackResult {
    pub placements: Vec<Placement>,
    pub coverage_pct: f32,
    pub page_width_mm: f32,
    pub page_height_mm: f32,
    pub elapsed_ms: u64,
    pub max_reached: bool,
    pub sticker_width_mm: f32,
    pub sticker_height_mm: f32,
    /// Base64-PNG of the un-rotated dilated margin band (transparent inside the
    /// sticker shape and outside the halo; opaque-cyan in the band). Drawn
    /// rotated under each placement when the user toggles "show margin".
    pub margin_band_png_b64: String,
    /// Offset of the band PNG's (0, 0) pixel from the source's (0, 0) corner,
    /// in mm of the scaled sticker. Add to a placement's (x_mm, y_mm) before
    /// drawing the band (after rotating).
    pub margin_band_offset_x_mm: f32,
    pub margin_band_offset_y_mm: f32,
    pub margin_band_width_mm: f32,
    pub margin_band_height_mm: f32,
}

/// One pre-rotated stamp (binary mask + its anchor offset). The anchor is the
/// (x, y) within the rotated bitmap that corresponds to the original sticker's
/// top-left, so placements stay aligned with the un-rotated source image.
struct Stamp {
    rotation_deg: f32,
    mask: Mask,
    width: u32,
    height: u32,
    anchor_x: i32,
    anchor_y: i32,
}

/// Rotate a binary mask by `angle_deg` around the source's (0, 0) corner.
/// Supersamples by checking 4 sub-pixel offsets per output pixel and ORing them
/// together — this eliminates the 1-px holes that pure nearest-neighbour
/// sampling produces at oblique angles, which would otherwise let copies slot
/// inside each other's margin zone. Returns the rotated mask cropped to its
/// rotated bounding box and the (anchor_x, anchor_y) of the source (0, 0).
fn rotate_mask(mask: &Mask, angle_deg: f32) -> (Mask, i32, i32) {
    if angle_deg.abs() < f32::EPSILON {
        return (mask.clone(), 0, 0);
    }
    let (w, h) = mask.dimensions();
    let theta = angle_deg.to_radians();
    let cos_t = theta.cos();
    let sin_t = theta.sin();

    let corners = [
        (0.0f32, 0.0f32),
        (w as f32, 0.0),
        (0.0, h as f32),
        (w as f32, h as f32),
    ];
    let mapped: Vec<(f32, f32)> = corners
        .iter()
        .map(|(x, y)| (x * cos_t - y * sin_t, x * sin_t + y * cos_t))
        .collect();
    let min_x = mapped.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let max_x = mapped.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let min_y = mapped.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max_y = mapped.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);

    let out_w = (max_x - min_x).ceil() as u32;
    let out_h = (max_y - min_y).ceil() as u32;
    let mut out = GrayImage::new(out_w.max(1), out_h.max(1));

    const SUB_OFFSETS: [(f32, f32); 4] = [
        (-0.25, -0.25),
        (0.25, -0.25),
        (-0.25, 0.25),
        (0.25, 0.25),
    ];
    let w_f = w as f32;
    let h_f = h as f32;
    for oy in 0..out_h {
        for ox in 0..out_w {
            let dx = ox as f32 + min_x;
            let dy = oy as f32 + min_y;
            let sx = dx * cos_t + dy * sin_t;
            let sy = -dx * sin_t + dy * cos_t;
            let mut hit = false;
            for (osx, osy) in &SUB_OFFSETS {
                let nsx = sx + osx;
                let nsy = sy + osy;
                if nsx >= 0.0 && nsy >= 0.0 && nsx < w_f && nsy < h_f {
                    if mask.get_pixel(nsx as u32, nsy as u32)[0] > 0 {
                        hit = true;
                        break;
                    }
                }
            }
            if hit {
                out.put_pixel(ox, oy, Luma([255]));
            }
        }
    }
    let anchor_x = (-min_x).round() as i32;
    let anchor_y = (-min_y).round() as i32;
    (out, anchor_x, anchor_y)
}

/// Build one rotated, margin-dilated stamp per rotation angle. The raw alpha
/// mask is rotated first (supersampled, so the silhouette stays hole-free),
/// then dilated by `pad_px` — that ordering keeps the margin band a clean
/// uniform width around the true rotated outline. Each pair of placed copies
/// is therefore separated by ≥ `2 * pad_px` on the page, never less.
fn build_stamps(raw_mask: &Mask, pad_px: u32, rotation_step_deg: f32) -> Vec<Stamp> {
    let mut angles: Vec<f32> = Vec::new();
    let mut a = 0.0f32;
    let step = rotation_step_deg.max(1.0);
    while a < 360.0 - 0.0001 {
        angles.push(a);
        a += step;
    }
    angles
        .into_par_iter()
        .map(|angle| {
            let (rotated, ax, ay) = rotate_mask(raw_mask, angle);
            let dilated = dilate_mask(&rotated, pad_px);
            let (cropped, ox, oy) =
                crop_to_content(&dilated).unwrap_or_else(|| (dilated.clone(), 0, 0));
            let anchor_x = ax - ox as i32;
            let anchor_y = ay - oy as i32;
            let (cw, ch) = cropped.dimensions();
            Stamp {
                rotation_deg: angle,
                mask: cropped,
                width: cw,
                height: ch,
                anchor_x,
                anchor_y,
            }
        })
        .collect()
}

/// Test whether `stamp` placed with its top-left at (x, y) on `occupancy`
/// would overlap any occupied pixel (or run off the page within the printable
/// area `[margin..page_size - margin]`).
fn fits_at(
    occupancy: &GrayImage,
    stamp: &Stamp,
    x: i32,
    y: i32,
    margin_min_x: i32,
    margin_min_y: i32,
    margin_max_x: i32,
    margin_max_y: i32,
) -> bool {
    if x < margin_min_x || y < margin_min_y {
        return false;
    }
    if x + stamp.width as i32 > margin_max_x || y + stamp.height as i32 > margin_max_y {
        return false;
    }
    for sy in 0..stamp.height {
        for sx in 0..stamp.width {
            if stamp.mask.get_pixel(sx, sy)[0] > 0 {
                if occupancy.get_pixel((x as u32) + sx, (y as u32) + sy)[0] > 0 {
                    return false;
                }
            }
        }
    }
    true
}

fn stamp_onto(occupancy: &mut GrayImage, stamp: &Stamp, x: i32, y: i32) {
    for sy in 0..stamp.height {
        for sx in 0..stamp.width {
            if stamp.mask.get_pixel(sx, sy)[0] > 0 {
                occupancy.put_pixel((x as u32) + sx, (y as u32) + sy, Luma([255]));
            }
        }
    }
}

pub fn pack(sticker: &Sticker, req: &PackRequest) -> PackResult {
    let t0 = std::time::Instant::now();

    let scale = req.scale_factor.max(0.01);
    let (page_w_mm, page_h_mm) = req.paper.dimensions_mm(req.orientation);
    let page_w_px = mm_to_px(page_w_mm, PACK_DPI);
    let page_h_px = mm_to_px(page_h_mm, PACK_DPI);
    let page_margin_px = mm_to_px(req.page_margin_mm, PACK_DPI) as i32;
    let inter_margin_px = mm_to_px(req.inter_copy_margin_mm, PACK_DPI);
    let sticker_w_mm = sticker.width_mm() * scale;
    let sticker_h_mm = sticker.height_mm() * scale;
    let empty_band = || empty_band_payload();

    // Rasterize the sticker at the internal packing resolution. We deliberately
    // do NOT pre-crop the raw alpha mask: the renderer rotates the full source
    // RGBA about its (0, 0) corner, so the packer's stamp anchors must also
    // reference source (0, 0). Cropping first would silently shift placements
    // by the source's transparent-padding width and push copies off the page.
    let sticker_rgba = sticker.resampled_scaled(PACK_DPI, scale);
    let raw_mask = mask_from_rgba(&sticker_rgba, 16);
    if mask_area_px(&raw_mask) == 0 {
        let (band_png_b64, band_off_x, band_off_y, band_w, band_h) = empty_band();
        return PackResult {
            placements: Vec::new(),
            coverage_pct: 0.0,
            page_width_mm: page_w_mm,
            page_height_mm: page_h_mm,
            elapsed_ms: t0.elapsed().as_millis() as u64,
            max_reached: false,
            sticker_width_mm: sticker_w_mm,
            sticker_height_mm: sticker_h_mm,
            margin_band_png_b64: band_png_b64,
            margin_band_offset_x_mm: band_off_x,
            margin_band_offset_y_mm: band_off_y,
            margin_band_width_mm: band_w,
            margin_band_height_mm: band_h,
        };
    }

    // Dilate each stamp by half the inter-copy margin so that when two stamps
    // are stamped down with no occupancy overlap, their *original* shapes are
    // at least inter_copy_margin_mm apart at the closest point.
    let pad_px = (inter_margin_px + 1) / 2;
    let stamps = build_stamps(&raw_mask, pad_px, req.rotation_step_deg);

    // Build the margin-band PNG once per pack — the un-rotated dilated outline
    // minus the raw outline, cropped tight, encoded for the canvas overlay.
    let (band_png_b64, band_off_x, band_off_y, band_w_mm, band_h_mm) =
        build_margin_band(&raw_mask, pad_px);
    let stride = req.quality.scan_stride().max(1);

    let mut occupancy = GrayImage::new(page_w_px, page_h_px);
    let margin_min_x: i32 = page_margin_px;
    let margin_min_y: i32 = page_margin_px;
    let margin_max_x: i32 = page_w_px as i32 - page_margin_px;
    let margin_max_y: i32 = page_h_px as i32 - page_margin_px;

    let max_count = req.max_count.unwrap_or(u32::MAX);
    let mut placements: Vec<Placement> = Vec::new();
    let mut max_reached = false;

    while (placements.len() as u32) < max_count {
        // For each stamp rotation, find the bottom-left-most valid position.
        // Scan with stride for speed, then refine by stride/4 within the window.
        let best = stamps
            .par_iter()
            .filter_map(|stamp| {
                let mut best_yx: Option<(i32, i32)> = None;
                let mut y = margin_min_y;
                while y + stamp.height as i32 <= margin_max_y {
                    let mut x = margin_min_x;
                    while x + stamp.width as i32 <= margin_max_x {
                        if fits_at(
                            &occupancy,
                            stamp,
                            x,
                            y,
                            margin_min_x,
                            margin_min_y,
                            margin_max_x,
                            margin_max_y,
                        ) {
                            match best_yx {
                                Some((by, bx)) if (y, x) >= (by, bx) => {}
                                _ => best_yx = Some((y, x)),
                            }
                            // Found leftmost on this row; break to next row.
                            break;
                        }
                        x += stride as i32;
                    }
                    if best_yx.is_some() {
                        break;
                    }
                    y += stride as i32;
                }
                best_yx.map(|(y, x)| (stamp, x, y))
            })
            .min_by(|a, b| {
                // Prefer smaller y (lower on page); break ties by smaller x.
                a.2.cmp(&b.2).then_with(|| a.1.cmp(&b.1))
            });

        match best {
            Some((stamp, x, y)) => {
                stamp_onto(&mut occupancy, stamp, x, y);
                // The placement position the renderer needs is the location of
                // the source sticker's (0, 0) corner, i.e. the stamp's anchor.
                let plc_x_px = x + stamp.anchor_x;
                let plc_y_px = y + stamp.anchor_y;
                placements.push(Placement {
                    x_mm: plc_x_px as f32 * 25.4 / PACK_DPI as f32,
                    y_mm: plc_y_px as f32 * 25.4 / PACK_DPI as f32,
                    rotation_deg: stamp.rotation_deg,
                });
                if (placements.len() as u32) >= max_count {
                    max_reached = true;
                    break;
                }
            }
            None => break,
        }
    }

    let occupied = mask_area_px(&occupancy) as f32;
    let total = (page_w_px as f32) * (page_h_px as f32);
    let coverage_pct = if total > 0.0 { occupied / total * 100.0 } else { 0.0 };

    PackResult {
        placements,
        coverage_pct,
        page_width_mm: page_w_mm,
        page_height_mm: page_h_mm,
        elapsed_ms: t0.elapsed().as_millis() as u64,
        max_reached,
        sticker_width_mm: sticker_w_mm,
        sticker_height_mm: sticker_h_mm,
        margin_band_png_b64: band_png_b64,
        margin_band_offset_x_mm: band_off_x,
        margin_band_offset_y_mm: band_off_y,
        margin_band_width_mm: band_w_mm,
        margin_band_height_mm: band_h_mm,
    }
}

/// Returns `(png_b64, offset_x_mm, offset_y_mm, width_mm, height_mm)` for a
/// translucent cyan visualization of the inter-copy margin band — the set of
/// pixels the dilation added to the raw alpha shape. The PNG is in scaled
/// sticker mm (i.e., at the user-selected scale factor), so the canvas can
/// draw it at the same per-mm zoom level as the sticker preview.
fn build_margin_band(
    raw_mask: &Mask,
    pad_px: u32,
) -> (String, f32, f32, f32, f32) {
    if pad_px == 0 {
        return empty_band_payload();
    }
    let dilated = dilate_mask(raw_mask, pad_px);
    let (rw, rh) = dilated.dimensions();
    let mut band = GrayImage::new(rw, rh);
    for y in 0..rh {
        for x in 0..rw {
            let d = dilated.get_pixel(x, y)[0];
            let r = raw_mask.get_pixel(x, y)[0];
            if d > 0 && r == 0 {
                band.put_pixel(x, y, Luma([255]));
            }
        }
    }
    let (cropped, ox, oy) = match crop_to_content(&band) {
        Some(t) => t,
        None => return empty_band_payload(),
    };
    let (cw, ch) = cropped.dimensions();
    // Colorize: translucent cyan where band=1.
    let mut rgba = RgbaImage::new(cw, ch);
    for y in 0..ch {
        for x in 0..cw {
            if cropped.get_pixel(x, y)[0] > 0 {
                rgba.put_pixel(x, y, Rgba([20, 184, 200, 140]));
            }
        }
    }
    let mut buf = Vec::new();
    {
        let mut cursor = Cursor::new(&mut buf);
        DynamicImage::ImageRgba8(rgba)
            .write_to(&mut cursor, ImageFormat::Png)
            .ok();
    }
    let png_b64 = B64.encode(&buf);
    let offset_x_mm = (ox as f32) * 25.4 / PACK_DPI as f32;
    let offset_y_mm = (oy as f32) * 25.4 / PACK_DPI as f32;
    let width_mm = (cw as f32) * 25.4 / PACK_DPI as f32;
    let height_mm = (ch as f32) * 25.4 / PACK_DPI as f32;
    (png_b64, offset_x_mm, offset_y_mm, width_mm, height_mm)
}

fn empty_band_payload() -> (String, f32, f32, f32, f32) {
    (String::new(), 0.0, 0.0, 0.0, 0.0)
}
