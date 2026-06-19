import { useCallback, useEffect, useRef, useState } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { Controls } from "./components/Controls";
import { FileDrop } from "./components/FileDrop";
import { PageCanvas } from "./components/PageCanvas";
import { exportPng, loadImageFromFile, runPack } from "./api";
import type { LoadedSticker, PackRequest, PackResult, Theme } from "./types";
import "./App.css";

const DEFAULT_REQUEST: PackRequest = {
  paper: { kind: "a4" },
  orientation: "portrait",
  dpi: 300,
  page_margin_mm: 5,
  inter_copy_margin_mm: 2,
  max_count: null,
  rotation_step_deg: 15,
  quality: "balanced",
  scale_factor: 1.0,
};

const THEME_STORAGE_KEY = "photo-filler.theme";
const DEFAULT_THEME: Theme = "light";
const SCALE_DEBOUNCE_MS = 350;
const MIN_SPINNER_MS = 250;

function App() {
  const [sticker, setSticker] = useState<LoadedSticker | null>(null);
  const [request, setRequest] = useState<PackRequest>(DEFAULT_REQUEST);
  const [result, setResult] = useState<PackResult | null>(null);
  const [lastPackedRequest, setLastPackedRequest] = useState<PackRequest | null>(null);
  const [packing, setPacking] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [showMargin, setShowMargin] = useState(false);
  const [theme, setTheme] = useState<Theme>(() => {
    const stored = localStorage.getItem(THEME_STORAGE_KEY) as Theme | null;
    return stored ?? DEFAULT_THEME;
  });
  const [message, setMessage] = useState<string>("");
  const fileInputRef = useRef<HTMLInputElement | null>(null);
  const packInFlightRef = useRef(false);

  useEffect(() => {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem(THEME_STORAGE_KEY, theme);
  }, [theme]);

  const dirty =
    !!sticker &&
    (!lastPackedRequest ||
      JSON.stringify(lastPackedRequest) !== JSON.stringify(request));

  const handlePack = useCallback(
    async (req: PackRequest) => {
      if (!sticker || packInFlightRef.current) return;
      packInFlightRef.current = true;
      setPacking(true);
      setMessage("");
      const t0 = performance.now();
      try {
        const r = await runPack(req);
        // Keep the spinner visible for at least MIN_SPINNER_MS so fast packs are
        // still perceptible — without it, sub-frame packs flash invisibly.
        const elapsed = performance.now() - t0;
        if (elapsed < MIN_SPINNER_MS) {
          await new Promise((resolve) =>
            setTimeout(resolve, MIN_SPINNER_MS - elapsed),
          );
        }
        setResult(r);
        setLastPackedRequest(req);
        setMessage(
          `Fit ${r.placements.length}${r.max_reached ? " (capped)" : ""} copies — ${r.coverage_pct.toFixed(1)}% coverage in ${r.elapsed_ms} ms`,
        );
      } catch (e) {
        setMessage(`Pack failed: ${String(e)}`);
      } finally {
        packInFlightRef.current = false;
        setPacking(false);
      }
    },
    [sticker],
  );

  // Auto-repack on scale change, debounced so dragging the slider doesn't
  // queue a pack per pixel. Only fires when we have a prior pack to update
  // (otherwise the user hasn't asked for any pack yet).
  useEffect(() => {
    if (!sticker || !lastPackedRequest) return;
    if (request.scale_factor === lastPackedRequest.scale_factor) return;
    const id = setTimeout(() => {
      handlePack(request);
    }, SCALE_DEBOUNCE_MS);
    return () => clearTimeout(id);
  }, [request.scale_factor, sticker, lastPackedRequest, handlePack, request]);

  async function handleFile(file: File) {
    setMessage("");
    setResult(null);
    setLastPackedRequest(null);
    try {
      const loaded = await loadImageFromFile(file, request.dpi);
      setSticker(loaded);
    } catch (e) {
      setMessage(`Load failed: ${String(e)}`);
    }
  }

  async function handleExport() {
    if (!sticker || !result) {
      setMessage("Pack first, then export.");
      return;
    }
    setExporting(true);
    try {
      const path = await save({
        title: "Save sticker sheet",
        filters: [{ name: "PNG", extensions: ["png"] }],
        defaultPath: "sticker-sheet.png",
      });
      if (!path) {
        setExporting(false);
        return;
      }
      const saved = await exportPng({
        path,
        paper: request.paper,
        orientation: request.orientation,
        dpi: request.dpi,
        scale_factor: request.scale_factor,
        placements: result.placements,
      });
      setMessage(`Saved: ${saved}`);
    } catch (e) {
      setMessage(`Export failed: ${String(e)}`);
    } finally {
      setExporting(false);
    }
  }

  return (
    <FileDrop ref={fileInputRef} onFile={handleFile}>
      <div className="app">
        <Controls
          sticker={sticker}
          request={request}
          packing={packing}
          exporting={exporting}
          dirty={dirty}
          showMargin={showMargin}
          theme={theme}
          lastResultText={message}
          onChange={setRequest}
          onPickFile={() => fileInputRef.current?.click()}
          onPack={() => handlePack(request)}
          onExport={handleExport}
          onToggleMargin={setShowMargin}
          onChangeTheme={setTheme}
        />
        <main className="preview">
          {sticker ? (
            <PageCanvas
              sticker={sticker}
              result={result}
              pageMarginMm={request.page_margin_mm}
              scaleFactor={request.scale_factor}
              showMargin={showMargin}
            />
          ) : (
            <div className="empty">
              <p>Drop a transparent PNG here, or click <strong>Choose PNG…</strong></p>
            </div>
          )}
          {packing && (
            <div className="spinner-overlay" aria-live="polite">
              <div className="spinner" />
              <div className="spinner-label">Packing…</div>
            </div>
          )}
        </main>
      </div>
    </FileDrop>
  );
}

export default App;
