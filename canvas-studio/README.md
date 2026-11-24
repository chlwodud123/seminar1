# Canvas Studio

Desktop & Web layered raster image editor built with Rust and `eframe`/`egui` (cloned via Craft Anything).

## Live Web App
- https://chlwodud123.github.io/seminar1/canvas-studio/

## Features
- Multi-document tabs (`800 x 600` default canvas, presets `640x480`, `800x600`, `1024x768`, `1600x1200`)
- 22 tools: Move Selected Pixels, Move Selection, Zoom, Pan, Rectangle/Ellipse/Lasso/Magic Wand Select, Paintbrush, Pencil, Eraser, Paint Bucket, Gradient (5 styles), Color Picker, Text, Line/Curve, Rectangle, Rounded Rectangle, Ellipse, Freeform Shape, Clone Stamp, Recolor
- Multi-layer editing with thumbnails, visibility toggle, opacity, and 14 blend modes
- Full History panel (Undo/Redo and click-to-jump)
- 34-color default palette, 10 recent colors, primary/secondary color picker
- Adjustments and Effects with real-time preview
- Open & save PNG, JPEG, BMP, and GIF images (supports drag-and-drop and download in browser)

## Build & Run (Desktop)
```bash
cargo build --release
cargo run
```

## Build (WebAssembly)
```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-dir pkg --target web --no-typescript target/wasm32-unknown-unknown/release/canvas_studio.wasm
```

## Headless Screenshot Mode (Desktop)
```bash
cargo run -- --screenshot shots/now.png
```
