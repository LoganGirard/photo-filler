use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Orientation {
    Portrait,
    Landscape,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PaperSize {
    A4,
    A3,
    UsLetter,
    UsLegal,
    Custom { width_mm: f32, height_mm: f32 },
}

impl PaperSize {
    pub fn dimensions_mm(&self, orientation: Orientation) -> (f32, f32) {
        let (w, h) = match self {
            PaperSize::A4 => (210.0, 297.0),
            PaperSize::A3 => (297.0, 420.0),
            PaperSize::UsLetter => (215.9, 279.4),
            PaperSize::UsLegal => (215.9, 355.6),
            PaperSize::Custom { width_mm, height_mm } => (*width_mm, *height_mm),
        };
        match orientation {
            Orientation::Portrait => (w, h),
            Orientation::Landscape => (h, w),
        }
    }
}

pub fn mm_to_px(mm: f32, dpi: u32) -> u32 {
    (mm * dpi as f32 / 25.4).round().max(1.0) as u32
}

pub fn px_to_mm(px: u32, dpi: u32) -> f32 {
    px as f32 * 25.4 / dpi as f32
}
