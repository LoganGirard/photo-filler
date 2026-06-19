import type {
  LoadedSticker,
  Orientation,
  PackRequest,
  PaperSize,
  Quality,
  Theme,
} from "../types";
import { PAPER_PRESETS } from "../types";

interface Props {
  sticker: LoadedSticker | null;
  request: PackRequest;
  packing: boolean;
  exporting: boolean;
  dirty: boolean;
  showMargin: boolean;
  theme: Theme;
  lastResultText: string;
  onChange: (next: PackRequest) => void;
  onPickFile: () => void;
  onPack: () => void;
  onExport: () => void;
  onToggleMargin: (v: boolean) => void;
  onChangeTheme: (t: Theme) => void;
}

const ROTATION_STEPS = [90, 45, 30, 15, 10, 5];
const DPI_OPTIONS = [150, 300, 600];

export function Controls({
  sticker,
  request,
  packing,
  exporting,
  dirty,
  showMargin,
  theme,
  lastResultText,
  onChange,
  onPickFile,
  onPack,
  onExport,
  onToggleMargin,
  onChangeTheme,
}: Props) {
  function setPaper(kind: PaperSize["kind"]) {
    if (kind === "custom") {
      onChange({
        ...request,
        paper: { kind: "custom", width_mm: 200, height_mm: 200 },
      });
    } else {
      onChange({ ...request, paper: { kind } as PaperSize });
    }
  }

  const isCustom = request.paper.kind === "custom";
  const scaledW = sticker ? sticker.width_mm * request.scale_factor : 0;
  const scaledH = sticker ? sticker.height_mm * request.scale_factor : 0;

  return (
    <aside className="controls">
      <header className="controls-header">
        <h2>Photo Filler</h2>
        <select
          className="theme-select"
          value={theme}
          onChange={(e) => onChangeTheme(e.target.value as Theme)}
          title="Color theme"
        >
          <option value="light">☀ Light</option>
          <option value="dark">🌙 Dark</option>
          <option value="pastel">🎨 Pastel artist</option>
        </select>
      </header>

      <section>
        <button className="primary" onClick={onPickFile}>
          {sticker ? "Replace image…" : "Choose PNG…"}
        </button>
        {sticker && (
          <div className="meta">
            <div>
              {sticker.width_px} × {sticker.height_px} px
            </div>
            <div>
              {sticker.width_mm.toFixed(1)} × {sticker.height_mm.toFixed(1)} mm
              {" "}
              @ {sticker.source_dpi} DPI
            </div>
            <div>
              {sticker.transparent_pct < 1 ? (
                <span className="warn">
                  ⚠ No transparency detected ({sticker.transparent_pct.toFixed(1)}%) —
                  the margin will be applied around the full rectangle, not
                  the shape outline.
                </span>
              ) : (
                <span className="ok">
                  Outline inferred from alpha ({sticker.transparent_pct.toFixed(0)}% transparent)
                </span>
              )}
            </div>
          </div>
        )}
      </section>

      <section>
        <label>
          Source DPI (what the input PNG represents)
          <input
            type="number"
            min={50}
            max={2400}
            value={request.dpi}
            onChange={(e) =>
              onChange({ ...request, dpi: Number(e.target.value) || 300 })
            }
          />
        </label>
      </section>

      <section>
        <label>
          Paper size
          <select
            value={request.paper.kind}
            onChange={(e) => setPaper(e.target.value as PaperSize["kind"])}
          >
            {PAPER_PRESETS.map((p) => (
              <option key={p.value.kind} value={p.value.kind}>
                {p.label}
              </option>
            ))}
            <option value="custom">Custom…</option>
          </select>
        </label>

        {isCustom && request.paper.kind === "custom" && (
          <div className="row">
            <label>
              W (mm)
              <input
                type="number"
                min={10}
                value={request.paper.width_mm}
                onChange={(e) =>
                  onChange({
                    ...request,
                    paper: {
                      kind: "custom",
                      width_mm: Number(e.target.value) || 0,
                      height_mm:
                        request.paper.kind === "custom"
                          ? request.paper.height_mm
                          : 0,
                    },
                  })
                }
              />
            </label>
            <label>
              H (mm)
              <input
                type="number"
                min={10}
                value={request.paper.height_mm}
                onChange={(e) =>
                  onChange({
                    ...request,
                    paper: {
                      kind: "custom",
                      width_mm:
                        request.paper.kind === "custom"
                          ? request.paper.width_mm
                          : 0,
                      height_mm: Number(e.target.value) || 0,
                    },
                  })
                }
              />
            </label>
          </div>
        )}

        <label>
          Orientation
          <select
            value={request.orientation}
            onChange={(e) =>
              onChange({
                ...request,
                orientation: e.target.value as Orientation,
              })
            }
          >
            <option value="portrait">Portrait</option>
            <option value="landscape">Landscape</option>
          </select>
        </label>

        <label>
          Output DPI
          <select
            value={request.dpi}
            onChange={(e) =>
              onChange({ ...request, dpi: Number(e.target.value) })
            }
          >
            {DPI_OPTIONS.map((d) => (
              <option key={d} value={d}>
                {d}
              </option>
            ))}
          </select>
        </label>
      </section>

      <section>
        <label>
          Sticker scale: {(request.scale_factor * 100).toFixed(0)}%
          {sticker && (
            <span className="dim"> — {scaledW.toFixed(1)} × {scaledH.toFixed(1)} mm</span>
          )}
          <input
            type="range"
            min={0.25}
            max={3}
            step={0.05}
            value={request.scale_factor}
            onChange={(e) =>
              onChange({
                ...request,
                scale_factor: Number(e.target.value),
              })
            }
          />
        </label>

        <label>
          Page margin: {request.page_margin_mm.toFixed(1)} mm
          <input
            type="range"
            min={0}
            max={25}
            step={0.5}
            value={request.page_margin_mm}
            onChange={(e) =>
              onChange({
                ...request,
                page_margin_mm: Number(e.target.value),
              })
            }
          />
        </label>

        <label>
          Space between stickers: {request.inter_copy_margin_mm.toFixed(1)} mm
          <input
            type="range"
            min={0}
            max={15}
            step={0.5}
            value={request.inter_copy_margin_mm}
            onChange={(e) =>
              onChange({
                ...request,
                inter_copy_margin_mm: Number(e.target.value),
              })
            }
          />
        </label>
      </section>

      <section>
        <label>
          Rotation step (°) — smaller = better packing, slower
          <select
            value={request.rotation_step_deg}
            onChange={(e) =>
              onChange({
                ...request,
                rotation_step_deg: Number(e.target.value),
              })
            }
          >
            {ROTATION_STEPS.map((s) => (
              <option key={s} value={s}>
                {s}°
              </option>
            ))}
          </select>
        </label>

        <label>
          Quality
          <select
            value={request.quality}
            onChange={(e) =>
              onChange({ ...request, quality: e.target.value as Quality })
            }
          >
            <option value="fast">Fast</option>
            <option value="balanced">Balanced</option>
            <option value="thorough">Thorough</option>
          </select>
        </label>

        <label className="row">
          <input
            type="checkbox"
            checked={request.max_count === null}
            onChange={(e) =>
              onChange({
                ...request,
                max_count: e.target.checked ? null : 12,
              })
            }
          />
          Fill page (max copies)
        </label>

        {request.max_count !== null && (
          <label>
            Copy count
            <input
              type="number"
              min={1}
              value={request.max_count}
              onChange={(e) =>
                onChange({
                  ...request,
                  max_count: Math.max(1, Number(e.target.value) || 1),
                })
              }
            />
          </label>
        )}

        <label className="row">
          <input
            type="checkbox"
            checked={showMargin}
            onChange={(e) => onToggleMargin(e.target.checked)}
          />
          Show enforced margin around each sticker
        </label>
      </section>

      <section className="actions">
        <button
          className="primary"
          disabled={!sticker || packing || !dirty}
          onClick={onPack}
          title={
            !sticker
              ? "Load a PNG first"
              : !dirty
                ? "Up to date — change a setting to repack"
                : "Pack now"
          }
        >
          {packing ? "Packing…" : dirty ? "Pack" : "Up to date"}
        </button>
        <button disabled={!sticker || exporting} onClick={onExport}>
          {exporting ? "Saving…" : "Export PNG…"}
        </button>
      </section>

      {lastResultText && <div className="result">{lastResultText}</div>}
    </aside>
  );
}
