use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba, RgbaImage};
use std::path::Path;

use crate::page::{mm_to_px, Orientation, PaperSize};
use crate::pack::{PackResult, Placement};
use crate::shape::Sticker;

/// Composite all placements onto a white page at `dpi`. Returns the final RGBA image.
pub fn render_page(
    sticker: &Sticker,
    paper: &PaperSize,
    orientation: Orientation,
    dpi: u32,
    scale_factor: f32,
    placements: &[Placement],
) -> RgbaImage {
    let (page_w_mm, page_h_mm) = paper.dimensions_mm(orientation);
    let page_w_px = mm_to_px(page_w_mm, dpi);
    let page_h_px = mm_to_px(page_h_mm, dpi);

    let mut canvas: RgbaImage = ImageBuffer::from_pixel(page_w_px, page_h_px, Rgba([255, 255, 255, 255]));
    let stamp_src = sticker.resampled_scaled(dpi, scale_factor);

    for p in placements {
        let rotated = rotate_rgba(&stamp_src, p.rotation_deg);
        // The packer stored the placement as the location of the source's (0,0)
        // corner. The rotated image's centre is offset from the original (0,0)
        // by (-min_x, -min_y) computed during rotation; rotate_rgba returns the
        // anchor so we can subtract it.
        let (rotated_img, anchor_x, anchor_y) = rotated;
        let dst_x = mm_to_px(p.x_mm, dpi) as i32 - anchor_x;
        let dst_y = mm_to_px(p.y_mm, dpi) as i32 - anchor_y;
        composite_at(&mut canvas, &rotated_img, dst_x, dst_y);
    }

    canvas
}

pub fn save_png(img: &RgbaImage, path: &Path) -> Result<(), String> {
    img.save(path).map_err(|e| format!("save png: {e}"))
}

/// Render a downscaled preview PNG (max edge `max_edge_px`) and return PNG bytes.
pub fn preview_png_bytes(img: &RgbaImage, max_edge_px: u32) -> Result<Vec<u8>, String> {
    let (w, h) = img.dimensions();
    let scale = (max_edge_px as f32 / w.max(h) as f32).min(1.0);
    let tw = (w as f32 * scale).round().max(1.0) as u32;
    let th = (h as f32 * scale).round().max(1.0) as u32;
    let resized = if scale < 1.0 {
        DynamicImage::ImageRgba8(img.clone())
            .resize_exact(tw, th, image::imageops::FilterType::Triangle)
            .to_rgba8()
    } else {
        img.clone()
    };
    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    DynamicImage::ImageRgba8(resized)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("encode png: {e}"))?;
    Ok(buf)
}

/// Rotate an RGBA image by `angle_deg` about its centre using bilinear sampling.
/// Returns (rotated_image, anchor_x, anchor_y) where (anchor_x, anchor_y) is the
/// pixel within the rotated image that corresponds to the source (0, 0) corner.
fn rotate_rgba(img: &RgbaImage, angle_deg: f32) -> (RgbaImage, i32, i32) {
    if angle_deg.abs() < f32::EPSILON {
        return (img.clone(), 0, 0);
    }
    let (w, h) = img.dimensions();
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
    let mut out = RgbaImage::new(out_w.max(1), out_h.max(1));

    for oy in 0..out_h {
        for ox in 0..out_w {
            let dx = ox as f32 + min_x;
            let dy = oy as f32 + min_y;
            let sx = dx * cos_t + dy * sin_t;
            let sy = -dx * sin_t + dy * cos_t;
            if sx >= 0.0 && sy >= 0.0 && sx < w as f32 - 1.0 && sy < h as f32 - 1.0 {
                let p = sample_bilinear(img, sx, sy);
                out.put_pixel(ox, oy, p);
            }
        }
    }
    let anchor_x = (-min_x).round() as i32;
    let anchor_y = (-min_y).round() as i32;
    (out, anchor_x, anchor_y)
}

fn sample_bilinear(img: &RgbaImage, x: f32, y: f32) -> Rgba<u8> {
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(img.width() - 1);
    let y1 = (y0 + 1).min(img.height() - 1);
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let p00 = img.get_pixel(x0, y0).0;
    let p10 = img.get_pixel(x1, y0).0;
    let p01 = img.get_pixel(x0, y1).0;
    let p11 = img.get_pixel(x1, y1).0;
    let mut out = [0u8; 4];
    for c in 0..4 {
        let v = (p00[c] as f32) * (1.0 - fx) * (1.0 - fy)
            + (p10[c] as f32) * fx * (1.0 - fy)
            + (p01[c] as f32) * (1.0 - fx) * fy
            + (p11[c] as f32) * fx * fy;
        out[c] = v.round().clamp(0.0, 255.0) as u8;
    }
    Rgba(out)
}

fn composite_at(canvas: &mut RgbaImage, stamp: &RgbaImage, dst_x: i32, dst_y: i32) {
    let (cw, ch) = canvas.dimensions();
    let (sw, sh) = stamp.dimensions();
    for sy in 0..sh {
        let cy = dst_y + sy as i32;
        if cy < 0 || cy as u32 >= ch {
            continue;
        }
        for sx in 0..sw {
            let cx = dst_x + sx as i32;
            if cx < 0 || cx as u32 >= cw {
                continue;
            }
            let src = stamp.get_pixel(sx, sy).0;
            if src[3] == 0 {
                continue;
            }
            let dst = canvas.get_pixel(cx as u32, cy as u32).0;
            let sa = src[3] as f32 / 255.0;
            let da = dst[3] as f32 / 255.0;
            let out_a = sa + da * (1.0 - sa);
            let blend = |s: u8, d: u8| {
                if out_a <= 0.0 {
                    0u8
                } else {
                    let v = (s as f32 * sa + d as f32 * da * (1.0 - sa)) / out_a;
                    v.round().clamp(0.0, 255.0) as u8
                }
            };
            let out_px = Rgba([
                blend(src[0], dst[0]),
                blend(src[1], dst[1]),
                blend(src[2], dst[2]),
                (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
            ]);
            canvas.put_pixel(cx as u32, cy as u32, out_px);
        }
    }
}

// Helper used by the unused-result render path (kept for clarity).
#[allow(dead_code)]
pub fn _summary(result: &PackResult) -> String {
    format!(
        "{} copies, {:.1}% coverage in {} ms",
        result.placements.len(),
        result.coverage_pct,
        result.elapsed_ms
    )
}
