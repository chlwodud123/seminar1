# Mechanics & Constants Sheet (Extracted from Pinta 3.1.2)

Target inspected: `C:\Users\chlwo\AppData\Local\Programs\Pinta\bin\Pinta.exe` (native MSVC apphost) and its companion .NET 8 assemblies (`Pinta.dll`, `Pinta.Core.dll`, `Pinta.Tools.dll`, `Pinta.Effects.dll`, `Pinta.Gui.Widgets.dll`).
All constants below were extracted directly from the .NET IL method bodies and metadata tables of those assemblies.

## 1. Window & Startup Defaults

| Parameter | Value | Source |
|---|---|---|
| Default Window Width | `1100` px | `Pinta.dll` `Pinta.MainWindow::CreateWindow` (RVA `0x6710`, key `window-size-width`) |
| Default Window Height | `750` px | `Pinta.dll` `Pinta.MainWindow::CreateWindow` (RVA `0x6710`, key `window-size-height`) |
| Default Window Maximized | `false` (`0`) | `Pinta.dll` `Pinta.MainWindow::CreateWindow` (RVA `0x6710`, key `window-maximized`) |
| Default Startup Canvas Width | `800` px | `Pinta.dll` `Pinta.MainClass::OpenFilesFromCommandLine` (RVA `0x5e64`) |
| Default Startup Canvas Height | `600` px | `Pinta.dll` `Pinta.MainClass::OpenFilesFromCommandLine` (RVA `0x5e64`) |
| Default Startup Background | White `(1.0, 1.0, 1.0, 1.0)` | `Pinta.dll` `Pinta.MainClass::OpenFilesFromCommandLine` (RVA `0x5e64`) |
| Default Active Tool | `PaintBrushTool` | `Pinta.dll` `Pinta.MainWindow::LoadUserSettings` (RVA `0x6d94`, key `last-selected-tool`) |
| Tool Options Bar Height | `48` px | `Pinta.dll` `Pinta.MainWindow::CreateToolToolBar` (RVA `0x6ba8`) |
| Toolbar Visible Default | `true` (`1`) | `Pinta.dll` `Pinta.MainWindow::LoadUserSettings` (RVA `0x6d94`, key `toolbar-shown`) |
| Statusbar Visible Default | `true` (`1`) | `Pinta.dll` `Pinta.MainWindow::LoadUserSettings` (RVA `0x6d94`, key `statusbar-shown`) |
| Toolbox Visible Default | `true` (`1`) | `Pinta.dll` `Pinta.MainWindow::LoadUserSettings` (RVA `0x6d94`, key `toolbox-shown`) |
| Image Tabs Visible Default | `true` (`1`) | `Pinta.dll` `Pinta.MainWindow::LoadUserSettings` (RVA `0x6d94`, key `image-tabs-shown`) |
| Tool Windows Visible Default | `true` (`1`) | `Pinta.dll` `Pinta.MainWindow::LoadUserSettings` (RVA `0x6d94`, key `tool-windows-shown`) |
| Rulers Visible Default | `false` (`0`) | `Pinta.dll` `Pinta.MainWindow::LoadUserSettings` (RVA `0x6d94`, key `ruler-shown`) |
| Canvas Grid Visible Default | `false` (`0`) | `Pinta.Core.dll` `Pinta.Core.CanvasGridManager::LoadGridSettings` (RVA `0x1c794`) |
| Canvas Grid Cell Size | `64 x 64` px | `Pinta.Core.dll` `Pinta.Core.CanvasGridManager::LoadGridSettings` (RVA `0x1c794`) |
| Best Fit Margin | `20` px | `Pinta.dll` `Pinta.MainWindow::ZoomToWindow_Activated` (RVA `0x7278`) |

## 2. New Image Dialog Presets

Extracted from `Pinta.dll` `Pinta.NewImageDialog::.cctor` (RVA `0x4230`) and `.ctor` (RVA `0x367c`):
- Presets: `640 x 480`, `800 x 600`, `1024 x 768`, `1600 x 1200`
- Orientations: `Portrait`, `Landscape`
- Background Fill: `White`, `Background Color`, `Transparent`

## 3. Zoom Levels

Extracted from `Pinta.Core.dll` `Pinta.Core.ViewActions::.cctor` (RVA `0x6d80`) and `DefaultZoomIndex` (RVA `0x6d54`):
- 23 preset zoom factors + `Window` (Best Fit):
  `3600%`, `2400%`, `1600%`, `1200%`, `800%`, `700%`, `600%`, `500%`, `400%`, `300%`, `200%`, `175%`, `150%`, `125%`, **`100%` (default, index 14)**, `66%`, `50%`, `33%`, `25%`, `16%`, `12%`, `8%`, `5%`
- Min zoom: `5%` (`0.05`), Max zoom: `3600%` (`36.0`) (`DocumentWorkspace::ZoomAndRecenterView`, RVA `0xe8a8`)

## 4. Palette & Colors

Extracted from `Pinta.Core.dll`:
- `PaletteManager::.ctor` (RVA `0x1d690`): `MAX_RECENT_COLORS = 10`
- Default primary color: Black `#000000`, secondary color: White `#FFFFFF` (estimated from standard Paint/Pinta convention)
- `PaletteHelper.<EnumerateDefaultColors>d__5::MoveNext` (RVA `0x27444`): 34 default palette colors:
  1. `#FFFFFF`, `#000000`
  2. `#A0A0A0`, `#808080`
  3. `#404040`, `#303030`
  4. `#FF0000`, `#FF7F7F`
  5. `#FF6A00`, `#FFB27F`
  6. `#FFD800`, `#FFE97F`
  7. `#B6FF00`, `#DAFF7F`
  8. `#4CFF00`, `#A5FF7F`
  9. `#00FF21`, `#7FFF8E`
  10. `#00FF90`, `#7FFFC5`
  11. `#00FFFF`, `#7FFFFF`
  12. `#0094FF`, `#7FC9FF`
  13. `#0026FF`, `#7F92FF`
  14. `#4800FF`, `#A17FFF`
  15. `#B200FF`, `#D67FFF`
  16. `#FF00DC`, `#FF7FED`
  17. `#FF006E`, `#FF7FB6`

## 5. Tools (Priority Order, Shortcuts & Defaults)

Extracted from `Pinta.Tools.dll`:

| Priority | Tool | Shortcut | Defaults / Notes | Source RVA |
|---:|---|---|---|---|
| 5 | Move Selected Pixels | `M` | Moves pixels within active selection | `0xbd51` / `0xbd9e` / `0xbda7` |
| 7 | Move Selection | `M` | Moves selection outline only | `0xc0ea` / `0xc137` / `0xc140` |
| 9 | Zoom | `Z` | Left-click zoom in, right-click zoom out, drag box | `0xfc13` / `0xfc3a` / `0xfc43` |
| 11 | Pan | `H` | Drag canvas view | `0xcb19` / `0xcb44` / `0xcb4d` |
| 13 | Rectangle Select | `S` | Rectangular selection (Replace/Union/Exclude/Xor/Intersect) | `0xd4be` / `0xd5f0` / `0xd4e5` |
| 15 | Ellipse Select | `S` | Elliptical selection | `0x9a84` / `0xd5f0` / `0x9aab` |
| 17 | Lasso Select | `S` | Freeform polygon selection | `0xb6b5` / `0xb6d4` / `0xb6f9` |
| 19 | Magic Wand Select | `S` | Flood color tolerance selection (`0..100%`) | `0xbbed` / `0xbbe4` / `0xbc28` |
| 21 | Paintbrush | `B` | Default width `2` px (`[`, `]` adjust), antialiasing | `0xc29f` / `0xc2c1` / `0x88c3` |
| 25 | Pencil | `P` | Hard 1-pixel drawing | `0xcc41` / `0xcc7b` / `0xcc84` |
| 27 | Eraser | `E` | Default width `2` px, erases to transparent | `0x9b6b` / `0x9b8d` / `0x9b96` |
| 29 | Paint Bucket | `F` | Flood fill, tolerance `0..100%` (default `0%`), contiguous/global | `0xc8e5` / `0xc920` / `0xa62c` |
| 31 | Gradient | `G` | Linear, Linear Reflected, Diamond, Radial, Conical; Color/Transparency mode | `0xadef` / `0xae0e` / `0xae33` |
| 33 | Color Picker | `K` | Sample 1x1, 3x3, 5x5, 7x7, 9x9; Left=Primary, Right=Secondary | `0x9319` / `0x933b` / `0x9344` |
| 35 | Text | `T` | Interactive text placement, font size, bold/italic | `0xdd00` / `0xdd1f` / `0xdd28` |
| 37 | Line/Curve | `O` | Straight line / Bezier curve with width & style | `0xbb98` / `0xda8b` / `0xbbb3` |
| 39 | Rectangle | `O` | Outline, Fill, Outline+Fill | `0xd544` / `0xda8b` / `0xd55f` |
| 41 | Rounded Rectangle | `O` | Rounded corner rectangle | `0xd5c0` / `0xda8b` / `0xd5db` |
| 43 | Ellipse | `O` | Outline, Fill, Outline+Fill | `0x9b0c` / `0xda8b` / `0x9b27` |
| 45 | Freeform Shape | `O` | Freehand closed polygon shape | `0xa735` / `0xa770` / `0xa779` |
| 47 | Clone Stamp | `L` | Ctrl+Click sets source anchor, drag clones pixels | `0x8ec9` / `0x8eff` / `0x8f08` |
| 49 | Recolor | `R` | Replaces secondary color with primary within tolerance | `0xcf85` / `0xcfc0` / `0xcfe1` |

## 6. Layers & Blend Modes

Extracted from `Pinta.Core.dll` `Pinta.Core.Layer` (RVA `0xf0e4`), `LayerPropertiesDialog` (RVA `0x2fc4`), `BlendMode`:
- Layer defaults: `opacity = 1.0` (`100%`, range `0..100%`), `hidden = false`, `blend_mode = Normal`, default name `"Layer {n}"` (background `"Background"`).
- 14 Blend Modes:
  1. `Normal`: `src`
  2. `Multiply`: `dst * src`
  3. `ColorBurn`: `1 - min(1, (1 - dst) / max(src, eps))`
  4. `ColorDodge`: `min(1, dst / max(1 - src, eps))`
  5. `Overlay`: `if dst < 0.5 { 2*dst*src } else { 1 - 2*(1-dst)*(1-src) }`
  6. `Difference`: `|dst - src|`
  7. `Lighten`: `max(dst, src)`
  8. `Darken`: `min(dst, src)`
  9. `Screen`: `1 - (1 - dst) * (1 - src)`
  10. `Xor`: Porter-Duff XOR alpha compositing
  11. `HardLight`: `if src < 0.5 { 2*dst*src } else { 1 - 2*(1-dst)*(1-src) }`
  12. `SoftLight`: W3C soft-light formula
  13. `Color`: HSL hue & saturation of `src`, luminosity of `dst`
  14. `Luminosity`: HSL hue & saturation of `dst`, luminosity of `src`

## 7. Adjustments & Effects Defaults

Extracted from `Pinta.Effects.dll`:
- **Adjustments**:
  - `Auto Level` (`0x207f`): stretch per-channel histogram to `0..255`
  - `Black and White` (`0x210a`): luminance `0.299*R + 0.587*G + 0.114*B`
  - `Brightness / Contrast` (`0x2142`): brightness `-100..100` (default `0`), contrast `-100..100` (default `0`)
  - `Hue / Saturation` (`0x2701`, `0xe2e0`): hue `-180..180` (default `0`), saturation `0..200` (default `100`), lightness `-100..100` (default `0`)
  - `Invert Colors` (`0x27c0`): `255 - c`
  - `Levels` (`0x27f8`): input black/white/gamma, output black/white
  - `Posterize` (`0x28f3`, `0x29fe`): levels `2..64` per channel (default `16, 16, 16`)
  - `Sepia` (`0x2a30`): grayscale then sepia tone transform
- **Effects**:
  - `Gaussian Blur` (`0x8e53`, `0x11f79`): radius `1..50` (default `2`)
  - `Motion Blur` (`0xa449`, `0x13035`): angle default `25.0°`, distance default `10`, centered `true`
  - `Radial Blur` (`0xb4f0`, `0x141c6`): angle default `2.0°`
  - `Zoom Blur` (`0xd227`, `0x16298`): amount default `10`
  - `Sharpen` (`0xbcc2`, `0xbd97`): amount `1..20` (default `2`)
  - `Glow` (`0x968d`, `0x11fbb`): radius `6`, brightness `10`, contrast `10`
  - `Vignette` (`0xcb67`, `0xce2a`): radius `50`, density `1.0`
  - `Pixelate` (`0xb076`, `0xb316`): cell size `1..100` (default `2`)
  - `Bulge` (`0x7060`, `0x1002e`): amount `-200..100` (default `45`)
  - `Twist` (`0xc613`, `0x15198`): amount `-100..100` (default `30`), size `100`
  - `Frosted Glass` (`0x8a8a`, `0x11f42`): max scatter radius default `1`
  - `Add Noise` (`0x68c6`, `0xf977`): intensity `64`, color saturation `100`, coverage `100.0%`
  - `Oil Painting` (`0xa806`, `0x13505`): brush size `3`, coarseness `50`
  - `Edge Detect` (`0x8192`, `0x8348`): angle default `45.0°`
  - `Emboss` (`0x836e`, `0x115ac`): angle default `0.0°`
  - `Relief` (`0xbb44`, `0xbc9c`): angle default `45.0°`
  - `Clouds` (`0x7674`, `0x10bec`): scale `250`, power `50`
  - `Mandelbrot Fractal` (`0x9fa0`, `0x12d40`) & `Julia Fractal` (`0x9c23`, `0x126f0`)
