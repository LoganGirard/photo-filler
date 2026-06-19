# Photo Filler

A cross-platform desktop app (macOS & Windows, via Tauri) that takes a single
transparent-background PNG sticker and packs as many rotated copies as possible
onto a printable page (A4, US Letter, custom, etc.) for sticker-sheet cutting.

## Run locally

```bash
npm install
npm run tauri dev
```

The first `tauri dev` will build the Rust backend (~5–10 minutes initially).

## Build a release bundle

```bash
npm run tauri build
```

Produces a `.app` / `.dmg` on macOS and `.msi` / `.exe` on Windows under
`src-tauri/target/release/bundle/`.

## How it works

The packer is a raster bottom-left-fill over a configurable set of rotation
angles. Each candidate rotation is rasterized into a binary collision mask
(dilated by half the inter-copy margin) and slid across an occupancy grid at
an internal resolution of 40 DPI. The first valid top-left position on the
top-most row wins per rotation; the overall best across rotations is chosen.
Placements are stored in millimetres so the final compositor can render at the
user's chosen output DPI without re-running the search.

## Project layout

| Path | Purpose |
|---|---|
| `src-tauri/src/page.rs` | Paper sizes, orientation, mm↔px |
| `src-tauri/src/shape.rs` | PNG decode, alpha mask, dilation |
| `src-tauri/src/pack.rs` | Raster bottom-left-fill packer |
| `src-tauri/src/render.rs` | Rotate + alpha-composite final PNG |
| `src-tauri/src/commands.rs` | Tauri command handlers |
| `src/App.tsx` | Two-pane app shell |
| `src/components/Controls.tsx` | Left-pane form |
| `src/components/PageCanvas.tsx` | Live preview canvas |
| `src/components/FileDrop.tsx` | Drag-and-drop wrapper |
