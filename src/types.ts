export type Orientation = "portrait" | "landscape";

export type PaperSize =
  | { kind: "a4" }
  | { kind: "a3" }
  | { kind: "us_letter" }
  | { kind: "us_legal" }
  | { kind: "custom"; width_mm: number; height_mm: number };

export type Quality = "fast" | "balanced" | "thorough";

export interface PackRequest {
  paper: PaperSize;
  orientation: Orientation;
  dpi: number;
  page_margin_mm: number;
  inter_copy_margin_mm: number;
  max_count: number | null;
  rotation_step_deg: number;
  quality: Quality;
  scale_factor: number;
}

export interface Placement {
  x_mm: number;
  y_mm: number;
  rotation_deg: number;
}

export interface PackResult {
  placements: Placement[];
  coverage_pct: number;
  page_width_mm: number;
  page_height_mm: number;
  elapsed_ms: number;
  max_reached: boolean;
  sticker_width_mm: number;
  sticker_height_mm: number;
  margin_band_png_b64: string;
  margin_band_offset_x_mm: number;
  margin_band_offset_y_mm: number;
  margin_band_width_mm: number;
  margin_band_height_mm: number;
}

export type Theme = "light" | "dark" | "pastel";

export interface LoadedSticker {
  width_mm: number;
  height_mm: number;
  width_px: number;
  height_px: number;
  source_dpi: number;
  preview_png_b64: string;
  transparent_pct: number;
}

export interface ExportRequest {
  path: string;
  paper: PaperSize;
  orientation: Orientation;
  dpi: number;
  scale_factor: number;
  placements: Placement[];
}

export const PAPER_PRESETS: { label: string; value: PaperSize }[] = [
  { label: "A4 (210 × 297 mm)", value: { kind: "a4" } },
  { label: "A3 (297 × 420 mm)", value: { kind: "a3" } },
  { label: "US Letter (8.5 × 11 in)", value: { kind: "us_letter" } },
  { label: "US Legal (8.5 × 14 in)", value: { kind: "us_legal" } },
];

export function paperLabel(p: PaperSize): string {
  const preset = PAPER_PRESETS.find((x) => x.value.kind === p.kind);
  if (preset) return preset.label;
  if (p.kind === "custom") return `Custom (${p.width_mm} × ${p.height_mm} mm)`;
  return p.kind;
}
