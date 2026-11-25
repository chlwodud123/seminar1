# Beach Blob Volley (Blobby)

Desktop & Web arcade beach volleyball game built with Rust and `eframe`/`egui` (cloned via Craft Anything).

## Live Web App
- https://chlwodud123.github.io/seminar1/blobby/

## Features
- 1P vs AI (Weak / Medium / Strong), 2-Player Local, and AI vs AI Demo modes
- Authentic dual-sphere blob-ball physics, Verlet integration, net top/shaft collisions, and 75 Hz fixed-timestep simulation
- Default (Rally Point) and Classic (Side-Out) rulesets, customizable score-to-win (`5`, `11`, `15`, `21`), and 5 game speed presets (`40%`–`150%`)
- Court themes (Sunny Beach / Sunset Cove), custom blob colors & morphing, shadow and FPS toggles

## Controls
- **1P vs AI**: `A` / `D` or `Left` / `Right` to move, `W` / `Up` / `Space` to jump
- **2-Player Local**: Left Player `A` / `D` / `W`, Right Player `Left` / `Right` / `Up`
- **Pause / Menu**: `Esc` or `P`

## Build & Run (Desktop)
```bash
cargo build --release
cargo run
```

## Build (WebAssembly)
```bash
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
wasm-bindgen --out-dir pkg --target web --no-typescript target/wasm32-unknown-unknown/release/beach_blob_volley.wasm
```

## Headless Screenshot Mode (Desktop)
```bash
cargo run -- --screenshot shots/now.png
```
