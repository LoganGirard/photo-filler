import { invoke } from "@tauri-apps/api/core";
import type {
  ExportRequest,
  LoadedSticker,
  PackRequest,
  PackResult,
} from "./types";

function bytesToBase64(bytes: Uint8Array): string {
  let binary = "";
  const chunk = 0x8000;
  for (let i = 0; i < bytes.length; i += chunk) {
    binary += String.fromCharCode.apply(
      null,
      Array.from(bytes.subarray(i, i + chunk)),
    );
  }
  return btoa(binary);
}

export async function loadImageFromFile(
  file: File,
  sourceDpi: number,
): Promise<LoadedSticker> {
  const buf = new Uint8Array(await file.arrayBuffer());
  const b64 = bytesToBase64(buf);
  return invoke<LoadedSticker>("load_image_bytes", {
    bytesB64: b64,
    sourceDpi,
  });
}

export async function runPack(req: PackRequest): Promise<PackResult> {
  return invoke<PackResult>("pack", { req });
}

export async function exportPng(req: ExportRequest): Promise<string> {
  return invoke<string>("export_png", { req });
}
