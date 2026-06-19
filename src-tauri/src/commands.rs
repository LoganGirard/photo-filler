use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::pack::{pack as run_pack, PackRequest, PackResult};
use crate::render::{preview_png_bytes, render_page, save_png};
use crate::shape::Sticker;

/// Per-session state: the currently loaded sticker. Tauri 2 manages this via .manage().
#[derive(Default)]
pub struct AppState {
    pub sticker: Mutex<Option<Sticker>>,
}

#[derive(Debug, Serialize)]
pub struct LoadedSticker {
    pub width_mm: f32,
    pub height_mm: f32,
    pub width_px: u32,
    pub height_px: u32,
    pub source_dpi: u32,
    /// PNG-encoded thumbnail, base64-encoded, for the preview canvas to draw rotated copies.
    pub preview_png_b64: String,
    /// Fraction of pixels with alpha < 16. If this is ~0, the image has no usable
    /// transparency, the "outline" is the full bounding box, and the inter-copy
    /// margin will be applied around the rectangle, not the shape.
    pub transparent_pct: f32,
}

#[tauri::command]
pub fn load_image_path(
    path: String,
    source_dpi: u32,
    state: tauri::State<AppState>,
) -> Result<LoadedSticker, String> {
    let sticker = Sticker::load_from_path(std::path::Path::new(&path), source_dpi)?;
    finalize_load(sticker, state)
}

#[tauri::command]
pub fn load_image_bytes(
    bytes_b64: String,
    source_dpi: u32,
    state: tauri::State<AppState>,
) -> Result<LoadedSticker, String> {
    let bytes = B64
        .decode(bytes_b64.as_bytes())
        .map_err(|e| format!("decode base64: {e}"))?;
    let sticker = Sticker::load_from_bytes(&bytes, source_dpi)?;
    finalize_load(sticker, state)
}

fn finalize_load(sticker: Sticker, state: tauri::State<AppState>) -> Result<LoadedSticker, String> {
    let (w_px, h_px) = sticker.source.dimensions();
    let width_mm = sticker.width_mm();
    let height_mm = sticker.height_mm();
    let source_dpi = sticker.source_dpi;
    let total = (w_px as u64) * (h_px as u64);
    let mut transparent = 0u64;
    for p in sticker.source.pixels() {
        if p[3] < 16 {
            transparent += 1;
        }
    }
    let transparent_pct = if total > 0 {
        (transparent as f32) / (total as f32) * 100.0
    } else {
        0.0
    };
    let thumb_bytes = preview_png_bytes(&sticker.source, 512)?;
    let preview_png_b64 = B64.encode(thumb_bytes);
    *state.sticker.lock().unwrap() = Some(sticker);
    Ok(LoadedSticker {
        width_mm,
        height_mm,
        width_px: w_px,
        height_px: h_px,
        source_dpi,
        preview_png_b64,
        transparent_pct,
    })
}

#[tauri::command]
pub fn pack(req: PackRequest, state: tauri::State<AppState>) -> Result<PackResult, String> {
    let guard = state.sticker.lock().unwrap();
    let sticker = guard.as_ref().ok_or_else(|| "no image loaded".to_string())?;
    Ok(run_pack(sticker, &req))
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub path: String,
    pub paper: crate::page::PaperSize,
    pub orientation: crate::page::Orientation,
    pub dpi: u32,
    #[serde(default = "default_scale")]
    pub scale_factor: f32,
    pub placements: Vec<crate::pack::Placement>,
}

fn default_scale() -> f32 {
    1.0
}

#[tauri::command]
pub fn export_png(req: ExportRequest, state: tauri::State<AppState>) -> Result<String, String> {
    let guard = state.sticker.lock().unwrap();
    let sticker = guard.as_ref().ok_or_else(|| "no image loaded".to_string())?;
    let canvas = render_page(
        sticker,
        &req.paper,
        req.orientation,
        req.dpi,
        req.scale_factor,
        &req.placements,
    );
    let out_path = PathBuf::from(&req.path);
    save_png(&canvas, &out_path)?;
    Ok(req.path)
}
