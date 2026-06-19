import { useEffect, useRef, useState } from "react";
import type { LoadedSticker, PackResult } from "../types";

interface Props {
  sticker: LoadedSticker | null;
  result: PackResult | null;
  pageMarginMm: number;
  scaleFactor: number;
  showMargin: boolean;
}

/** Persistent view transform: page-in-screen scale + screen-pixel offset. */
interface View {
  scale: number;
  offsetX: number;
  offsetY: number;
}

const MIN_SCALE_MULT = 0.1; // vs. the fit-to-window baseline
const MAX_SCALE_MULT = 40;

/**
 * Draws the page to scale, then draws each placement by rotating the loaded
 * preview thumbnail about the sticker's (0, 0) corner — matching the renderer.
 *
 * Supports pan/zoom: pinch or Ctrl/Cmd+wheel zooms about the cursor, plain
 * wheel pans (trackpad two-finger swipe), left-button drag pans, double-click
 * resets to fit. View is preserved across resizes and re-packs of the same
 * sticker/paper; it resets when the sticker or page dimensions change.
 */
export function PageCanvas({
  sticker,
  result,
  pageMarginMm,
  scaleFactor,
  showMargin,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const stickerImgRef = useRef<HTMLImageElement | null>(null);
  const bandImgRef = useRef<HTMLImageElement | null>(null);
  const viewRef = useRef<View | null>(null);
  const baseScaleRef = useRef<number>(1);
  const draggingRef = useRef(false);
  const lastPointerRef = useRef({ x: 0, y: 0 });
  const [cursor, setCursor] = useState<"grab" | "grabbing">("grab");

  useEffect(() => {
    if (!sticker) {
      stickerImgRef.current = null;
      requestAnimationFrame(draw);
      return;
    }
    const img = new Image();
    img.onload = () => {
      stickerImgRef.current = img;
      requestAnimationFrame(draw);
    };
    img.src = `data:image/png;base64,${sticker.preview_png_b64}`;
  }, [sticker]);

  useEffect(() => {
    if (!result?.margin_band_png_b64) {
      bandImgRef.current = null;
      requestAnimationFrame(draw);
      return;
    }
    const img = new Image();
    img.onload = () => {
      bandImgRef.current = img;
      requestAnimationFrame(draw);
    };
    img.src = `data:image/png;base64,${result.margin_band_png_b64}`;
  }, [result?.margin_band_png_b64]);

  // Reset the view when sticker or page dims change — otherwise zooming in on
  // an A4 then switching to A3 would leave the page tiny in the corner.
  const stickerKey = sticker
    ? `${sticker.width_px}x${sticker.height_px}`
    : "none";
  const pageKey = result
    ? `${result.page_width_mm}x${result.page_height_mm}`
    : "default";
  useEffect(() => {
    viewRef.current = null;
    requestAnimationFrame(draw);
  }, [stickerKey, pageKey]);

  useEffect(() => {
    const onResize = () => draw();
    window.addEventListener("resize", onResize);
    draw();
    return () => window.removeEventListener("resize", onResize);
  }, [result, pageMarginMm, sticker, scaleFactor, showMargin]);

  // Wheel needs passive: false so we can preventDefault to stop the OS page
  // from scrolling/zooming the whole Tauri window during pinch.
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const handler = (e: WheelEvent) => onWheel(e);
    canvas.addEventListener("wheel", handler, { passive: false });
    return () => canvas.removeEventListener("wheel", handler);
  }, []);

  function ensureView(pageW: number, pageH: number, cw: number, ch: number): View {
    const pad = 24;
    const baseScale = Math.min(
      (cw - pad * 2) / pageW,
      (ch - pad * 2) / pageH,
    );
    baseScaleRef.current = baseScale;
    if (!viewRef.current) {
      viewRef.current = {
        scale: baseScale,
        offsetX: (cw - pageW * baseScale) / 2,
        offsetY: (ch - pageH * baseScale) / 2,
      };
    }
    return viewRef.current;
  }

  function onWheel(e: WheelEvent) {
    e.preventDefault();
    const canvas = canvasRef.current;
    const view = viewRef.current;
    if (!canvas || !view) return;
    const rect = canvas.getBoundingClientRect();
    const cx = e.clientX - rect.left;
    const cy = e.clientY - rect.top;

    // Trackpad pinch arrives as wheel + ctrlKey on every browser; Cmd/Ctrl +
    // wheel is the desktop-mouse fallback.
    if (e.ctrlKey || e.metaKey) {
      const factor = Math.exp(-e.deltaY * 0.01);
      const minScale = baseScaleRef.current * MIN_SCALE_MULT;
      const maxScale = baseScaleRef.current * MAX_SCALE_MULT;
      const newScale = clamp(view.scale * factor, minScale, maxScale);
      const f = newScale / view.scale;
      viewRef.current = {
        scale: newScale,
        offsetX: cx - (cx - view.offsetX) * f,
        offsetY: cy - (cy - view.offsetY) * f,
      };
    } else {
      // Plain wheel / two-finger swipe → pan.
      viewRef.current = {
        ...view,
        offsetX: view.offsetX - e.deltaX,
        offsetY: view.offsetY - e.deltaY,
      };
    }
    requestAnimationFrame(draw);
  }

  function onPointerDown(e: React.PointerEvent<HTMLCanvasElement>) {
    if (e.button !== 0) return;
    draggingRef.current = true;
    lastPointerRef.current = { x: e.clientX, y: e.clientY };
    e.currentTarget.setPointerCapture(e.pointerId);
    setCursor("grabbing");
  }

  function onPointerMove(e: React.PointerEvent<HTMLCanvasElement>) {
    if (!draggingRef.current || !viewRef.current) return;
    const dx = e.clientX - lastPointerRef.current.x;
    const dy = e.clientY - lastPointerRef.current.y;
    viewRef.current = {
      ...viewRef.current,
      offsetX: viewRef.current.offsetX + dx,
      offsetY: viewRef.current.offsetY + dy,
    };
    lastPointerRef.current = { x: e.clientX, y: e.clientY };
    requestAnimationFrame(draw);
  }

  function onPointerUp(e: React.PointerEvent<HTMLCanvasElement>) {
    if (!draggingRef.current) return;
    draggingRef.current = false;
    if (e.currentTarget.hasPointerCapture(e.pointerId)) {
      e.currentTarget.releasePointerCapture(e.pointerId);
    }
    setCursor("grab");
  }

  function onDoubleClick() {
    viewRef.current = null;
    requestAnimationFrame(draw);
  }

  function draw() {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;
    const dpr = window.devicePixelRatio || 1;
    const cw = container.clientWidth;
    const ch = container.clientHeight;
    canvas.width = Math.max(1, Math.floor(cw * dpr));
    canvas.height = Math.max(1, Math.floor(ch * dpr));
    canvas.style.width = `${cw}px`;
    canvas.style.height = `${ch}px`;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, cw, ch);

    const pageW = result?.page_width_mm ?? 210;
    const pageH = result?.page_height_mm ?? 297;
    const view = ensureView(pageW, pageH, cw, ch);
    const { scale, offsetX: ox, offsetY: oy } = view;
    const drawW = pageW * scale;
    const drawH = pageH * scale;

    const paper = getComputedStyle(document.documentElement)
      .getPropertyValue("--page-bg")
      .trim() || "#ffffff";
    const edge = getComputedStyle(document.documentElement)
      .getPropertyValue("--page-edge")
      .trim() || "#444";
    const guide = getComputedStyle(document.documentElement)
      .getPropertyValue("--margin-guide")
      .trim() || "#bbb";

    ctx.fillStyle = paper;
    ctx.strokeStyle = edge;
    ctx.lineWidth = 1;
    ctx.fillRect(ox, oy, drawW, drawH);
    ctx.strokeRect(ox + 0.5, oy + 0.5, drawW, drawH);

    if (pageMarginMm > 0) {
      ctx.strokeStyle = guide;
      ctx.setLineDash([4, 4]);
      ctx.lineWidth = 1;
      const mm = pageMarginMm * scale;
      ctx.strokeRect(ox + mm, oy + mm, drawW - mm * 2, drawH - mm * 2);
      ctx.setLineDash([]);
    }

    const stickerImg = stickerImgRef.current;
    if (!stickerImg || !sticker || !result) return;
    const w = sticker.width_mm * scaleFactor * scale;
    const h = sticker.height_mm * scaleFactor * scale;
    const bandImg = bandImgRef.current;
    const bandOffX = (result.margin_band_offset_x_mm ?? 0) * scale;
    const bandOffY = (result.margin_band_offset_y_mm ?? 0) * scale;
    const bandW = (result.margin_band_width_mm ?? 0) * scale;
    const bandH = (result.margin_band_height_mm ?? 0) * scale;

    for (const p of result.placements) {
      const drawX = ox + p.x_mm * scale;
      const drawY = oy + p.y_mm * scale;
      ctx.save();
      ctx.translate(drawX, drawY);
      ctx.rotate((p.rotation_deg * Math.PI) / 180);
      if (showMargin && bandImg && bandW > 0 && bandH > 0) {
        ctx.drawImage(bandImg, bandOffX, bandOffY, bandW, bandH);
      }
      ctx.drawImage(stickerImg, 0, 0, w, h);
      ctx.restore();
    }
  }

  return (
    <div className="canvas-container" ref={containerRef}>
      <canvas
        ref={canvasRef}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerCancel={onPointerUp}
        onDoubleClick={onDoubleClick}
        style={{ cursor, touchAction: "none" }}
      />
      <div className="zoom-hint">
        pinch / ⌘+scroll to zoom · drag to pan · double-click to reset
      </div>
    </div>
  );
}

function clamp(v: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, v));
}
