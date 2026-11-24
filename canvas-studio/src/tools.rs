use std::collections::VecDeque;

use crate::blend::{composite_pixel, paint_over, BlendMode};
use crate::canvas::{Document, SelectionCombineMode, SelectionMask};

/// All 22 Pinta tools in exact priority order extracted from `Pinta.Tools.dll`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToolKind {
    MoveSelectedPixels, // M, priority 5
    MoveSelection,      // M, priority 7
    Zoom,               // Z, priority 9
    Pan,                // H, priority 11
    RectangleSelect,    // S, priority 13
    EllipseSelect,      // S, priority 15
    LassoSelect,        // S, priority 17
    MagicWand,          // S, priority 19
    Paintbrush,         // B, priority 21 (default)
    Pencil,             // P, priority 25
    Eraser,             // E, priority 27
    PaintBucket,        // F, priority 29
    Gradient,           // G, priority 31
    ColorPicker,        // K, priority 33
    Text,               // T, priority 35
    LineCurve,          // O, priority 37
    Rectangle,          // O, priority 39
    RoundedRectangle,   // O, priority 41
    Ellipse,            // O, priority 43
    FreeformShape,      // O, priority 45
    CloneStamp,         // L, priority 47
    Recolor,            // R, priority 49
}

impl ToolKind {
    pub const ALL: [ToolKind; 22] = [
        ToolKind::MoveSelectedPixels,
        ToolKind::MoveSelection,
        ToolKind::Zoom,
        ToolKind::Pan,
        ToolKind::RectangleSelect,
        ToolKind::EllipseSelect,
        ToolKind::LassoSelect,
        ToolKind::MagicWand,
        ToolKind::Paintbrush,
        ToolKind::Pencil,
        ToolKind::Eraser,
        ToolKind::PaintBucket,
        ToolKind::Gradient,
        ToolKind::ColorPicker,
        ToolKind::Text,
        ToolKind::LineCurve,
        ToolKind::Rectangle,
        ToolKind::RoundedRectangle,
        ToolKind::Ellipse,
        ToolKind::FreeformShape,
        ToolKind::CloneStamp,
        ToolKind::Recolor,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ToolKind::MoveSelectedPixels => "Move Selected Pixels",
            ToolKind::MoveSelection => "Move Selection",
            ToolKind::Zoom => "Zoom",
            ToolKind::Pan => "Pan",
            ToolKind::RectangleSelect => "Rectangle Select",
            ToolKind::EllipseSelect => "Ellipse Select",
            ToolKind::LassoSelect => "Lasso Select",
            ToolKind::MagicWand => "Magic Wand Select",
            ToolKind::Paintbrush => "Paintbrush",
            ToolKind::Pencil => "Pencil",
            ToolKind::Eraser => "Eraser",
            ToolKind::PaintBucket => "Paint Bucket",
            ToolKind::Gradient => "Gradient",
            ToolKind::ColorPicker => "Color Picker",
            ToolKind::Text => "Text",
            ToolKind::LineCurve => "Line / Curve",
            ToolKind::Rectangle => "Rectangle",
            ToolKind::RoundedRectangle => "Rounded Rectangle",
            ToolKind::Ellipse => "Ellipse",
            ToolKind::FreeformShape => "Freeform Shape",
            ToolKind::CloneStamp => "Clone Stamp",
            ToolKind::Recolor => "Recolor",
        }
    }

    pub fn shortcut(self) -> char {
        match self {
            ToolKind::MoveSelectedPixels | ToolKind::MoveSelection => 'M',
            ToolKind::Zoom => 'Z',
            ToolKind::Pan => 'H',
            ToolKind::RectangleSelect
            | ToolKind::EllipseSelect
            | ToolKind::LassoSelect
            | ToolKind::MagicWand => 'S',
            ToolKind::Paintbrush => 'B',
            ToolKind::Pencil => 'P',
            ToolKind::Eraser => 'E',
            ToolKind::PaintBucket => 'F',
            ToolKind::Gradient => 'G',
            ToolKind::ColorPicker => 'K',
            ToolKind::Text => 'T',
            ToolKind::LineCurve
            | ToolKind::Rectangle
            | ToolKind::RoundedRectangle
            | ToolKind::Ellipse
            | ToolKind::FreeformShape => 'O',
            ToolKind::CloneStamp => 'L',
            ToolKind::Recolor => 'R',
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            ToolKind::MoveSelectedPixels => "Drag to move selected pixels (or active layer).",
            ToolKind::MoveSelection => "Drag to reposition the selection outline.",
            ToolKind::Zoom => "Left-click to zoom in, right-click to zoom out.",
            ToolKind::Pan => "Drag to pan the canvas view (Spacebar pans in any tool).",
            ToolKind::RectangleSelect => "Drag to select a rectangle. Hold Shift for a square.",
            ToolKind::EllipseSelect => "Drag to select an ellipse. Hold Shift for a circle.",
            ToolKind::LassoSelect => "Drag freehand to outline a selection region.",
            ToolKind::MagicWand => "Click to select contiguous pixels of similar color.",
            ToolKind::Paintbrush => "Left-click draws primary color, right-click draws secondary. [ ] resize brush.",
            ToolKind::Pencil => "Draw crisp 1-pixel strokes. Left=primary, right=secondary.",
            ToolKind::Eraser => "Drag to erase pixels to transparent. [ ] resize eraser.",
            ToolKind::PaintBucket => "Click to fill region with primary (left) or secondary (right) color.",
            ToolKind::Gradient => "Drag to draw a smooth gradient between primary and secondary colors.",
            ToolKind::ColorPicker => "Left-click picks primary color, right-click picks secondary color.",
            ToolKind::Text => "Click on canvas to place text, type, then press Enter or Commit.",
            ToolKind::LineCurve => "Drag to draw a straight line. Hold Shift to snap 15° angles.",
            ToolKind::Rectangle => "Drag to draw a rectangle. Hold Shift for a square.",
            ToolKind::RoundedRectangle => "Drag to draw a rounded rectangle. Hold Shift for a square.",
            ToolKind::Ellipse => "Drag to draw an ellipse. Hold Shift for a circle.",
            ToolKind::FreeformShape => "Drag to draw a closed freeform shape.",
            ToolKind::CloneStamp => "Ctrl+Click to set clone source, then drag to stamp pixels.",
            ToolKind::Recolor => "Drag to replace secondary color with primary color within tolerance.",
        }
    }

    pub fn uses_brush_width(self) -> bool {
        matches!(
            self,
            ToolKind::Paintbrush
                | ToolKind::Eraser
                | ToolKind::LineCurve
                | ToolKind::Rectangle
                | ToolKind::RoundedRectangle
                | ToolKind::Ellipse
                | ToolKind::FreeformShape
                | ToolKind::CloneStamp
                | ToolKind::Recolor
        )
    }

    pub fn uses_fill_style(self) -> bool {
        matches!(
            self,
            ToolKind::Rectangle
                | ToolKind::RoundedRectangle
                | ToolKind::Ellipse
                | ToolKind::FreeformShape
        )
    }

    pub fn uses_tolerance(self) -> bool {
        matches!(
            self,
            ToolKind::PaintBucket | ToolKind::MagicWand | ToolKind::Recolor
        )
    }

    pub fn uses_selection_mode(self) -> bool {
        matches!(
            self,
            ToolKind::RectangleSelect
                | ToolKind::EllipseSelect
                | ToolKind::LassoSelect
                | ToolKind::MagicWand
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShapeFillStyle {
    Outline,
    Fill,
    OutlineAndFill,
}

impl ShapeFillStyle {
    pub const ALL: [ShapeFillStyle; 3] = [
        ShapeFillStyle::Outline,
        ShapeFillStyle::Fill,
        ShapeFillStyle::OutlineAndFill,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ShapeFillStyle::Outline => "Outline",
            ShapeFillStyle::Fill => "Fill",
            ShapeFillStyle::OutlineAndFill => "Outline & Fill",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientKind {
    Linear,
    LinearReflected,
    Diamond,
    Radial,
    Conical,
}

impl GradientKind {
    pub const ALL: [GradientKind; 5] = [
        GradientKind::Linear,
        GradientKind::LinearReflected,
        GradientKind::Diamond,
        GradientKind::Radial,
        GradientKind::Conical,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GradientKind::Linear => "Linear",
            GradientKind::LinearReflected => "Linear Reflected",
            GradientKind::Diamond => "Diamond",
            GradientKind::Radial => "Radial",
            GradientKind::Conical => "Conical",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientColorMode {
    Color,
    Transparency,
}

#[derive(Clone, Debug)]
pub struct ToolSettings {
    /// Default brush width: 2 px (`Pinta.Tools.BaseBrushTool::get_BrushWidth` RVA 0x88c3).
    pub brush_width: i32,
    pub antialiasing: bool,
    pub overwrite_blend: bool,
    /// Flood / Magic Wand / Recolor tolerance 0..=100% (`FloodTool::get_ToleranceSlider` RVA 0xa62c).
    pub tolerance: i32,
    pub flood_contiguous: bool,
    pub fill_style: ShapeFillStyle,
    pub corner_radius: i32,
    pub gradient_kind: GradientKind,
    pub gradient_mode: GradientColorMode,
    pub picker_sample_size: i32,
    pub picker_sample_layer_only: bool,
    pub selection_mode: SelectionCombineMode,
    pub text_size: f32,
    pub text_bold: bool,
}

impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            brush_width: 2,
            antialiasing: true,
            overwrite_blend: false,
            tolerance: 0,
            flood_contiguous: true,
            fill_style: ShapeFillStyle::Outline,
            corner_radius: 16,
            gradient_kind: GradientKind::Linear,
            gradient_mode: GradientColorMode::Color,
            picker_sample_size: 1,
            picker_sample_layer_only: false,
            selection_mode: SelectionCombineMode::Replace,
            text_size: 24.0,
            text_bold: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FloatingMoveState {
    pub base_pixels: Vec<u8>,
    pub lifted_pixels: Vec<u8>,
    pub orig_selection: Option<SelectionMask>,
    pub start_pt: [i32; 2],
    pub offset: [i32; 2],
}

#[derive(Clone, Debug)]
pub struct ActiveTextState {
    pub pos: [f32; 2],
    pub text: String,
    pub color: [u8; 4],
}

#[derive(Default)]
pub struct ToolRuntime {
    pub dragging: bool,
    pub secondary_drag: bool,
    pub start_pt: [f32; 2],
    pub last_pt: [f32; 2],
    pub cur_pt: [f32; 2],
    pub poly_points: Vec<(f32, f32)>,
    pub clone_source: Option<[i32; 2]>,
    pub clone_drag_offset: Option<[i32; 2]>,
    pub move_state: Option<FloatingMoveState>,
    pub orig_selection_for_move: Option<SelectionMask>,
    pub active_text: Option<ActiveTextState>,
}

/// Draw a circular brush stamp at `(cx, cy)` onto `pixels` respecting selection mask.
pub fn stamp_circle(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    cx: f32,
    cy: f32,
    diameter: f32,
    color: [u8; 4],
    antialiasing: bool,
    overwrite: bool,
    erase: bool,
) {
    let radius = (diameter * 0.5).max(0.5);
    let pad = if antialiasing { 1.5 } else { 0.5 };
    let x0 = ((cx - radius - pad).floor() as i32).max(0) as u32;
    let x1 = ((cx + radius + pad).ceil() as i32).min(width as i32 - 1);
    let y0 = ((cy - radius - pad).floor() as i32).max(0) as u32;
    let y1 = ((cy + radius + pad).ceil() as i32).min(height as i32 - 1);
    if x1 < x0 as i32 || y1 < y0 as i32 {
        return;
    }

    for y in y0..=y1 as u32 {
        let dy = (y as f32 + 0.5) - cy;
        let row = (y * width) as usize;
        for x in x0..=x1 as u32 {
            if let Some(sel) = selection {
                if !sel.contains(x, y) {
                    continue;
                }
            }
            let dx = (x as f32 + 0.5) - cx;
            let dist = (dx * dx + dy * dy).sqrt();
            let cov = if antialiasing {
                (radius + 0.5 - dist).clamp(0.0, 1.0)
            } else if dist <= radius {
                1.0
            } else {
                0.0
            };
            if cov <= 0.0 {
                continue;
            }
            let idx = (row + x as usize) * 4;
            if erase {
                let a = pixels[idx + 3] as f32 * (1.0 - cov);
                pixels[idx + 3] = a.round().clamp(0.0, 255.0) as u8;
                if pixels[idx + 3] == 0 {
                    pixels[idx] = 0;
                    pixels[idx + 1] = 0;
                    pixels[idx + 2] = 0;
                }
            } else {
                let dst = [
                    pixels[idx],
                    pixels[idx + 1],
                    pixels[idx + 2],
                    pixels[idx + 3],
                ];
                let mut src = color;
                src[3] = ((color[3] as f32) * cov).round().clamp(0.0, 255.0) as u8;
                let out = paint_over(dst, src, overwrite && cov >= 0.99);
                pixels[idx..idx + 4].copy_from_slice(&out);
            }
        }
    }
}

/// Interpolate a smooth brush stroke between `(x0, y0)` and `(x1, y1)`.
pub fn stroke_segment(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    p0: [f32; 2],
    p1: [f32; 2],
    diameter: f32,
    color: [u8; 4],
    antialiasing: bool,
    overwrite: bool,
    erase: bool,
) {
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let dist = (dx * dx + dy * dy).sqrt();
    let step = (diameter * 0.25).clamp(0.5, 4.0);
    let steps = (dist / step).ceil().max(1.0) as i32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let x = p0[0] + dx * t;
        let y = p0[1] + dy * t;
        stamp_circle(
            pixels,
            width,
            height,
            selection,
            x,
            y,
            diameter,
            color,
            antialiasing,
            overwrite,
            erase,
        );
    }
}

/// Crisp 1-pixel Bresenham line for the Pencil tool.
pub fn draw_pencil_line(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    color: [u8; 4],
    overwrite: bool,
) {
    let mut x = x0;
    let mut y = y0;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        if x >= 0 && y >= 0 && (x as u32) < width && (y as u32) < height {
            let ux = x as u32;
            let uy = y as u32;
            if selection.map_or(true, |s| s.contains(ux, uy)) {
                let idx = ((uy * width + ux) * 4) as usize;
                let dst = [
                    pixels[idx],
                    pixels[idx + 1],
                    pixels[idx + 2],
                    pixels[idx + 3],
                ];
                let out = paint_over(dst, color, overwrite);
                pixels[idx..idx + 4].copy_from_slice(&out);
            }
        }
        if x == x1 && y == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

#[inline]
fn color_within_tolerance(a: [u8; 4], b: [u8; 4], tol_pct: i32) -> bool {
    let dr = (a[0] as i32 - b[0] as i32).abs();
    let dg = (a[1] as i32 - b[1] as i32).abs();
    let db = (a[2] as i32 - b[2] as i32).abs();
    let da = (a[3] as i32 - b[3] as i32).abs();
    let max_diff = dr.max(dg).max(db).max(da);
    let threshold = (tol_pct.clamp(0, 100) * 255) / 100;
    max_diff <= threshold
}

/// Paint Bucket flood fill (contiguous BFS or global).
pub fn flood_fill(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    sx: u32,
    sy: u32,
    fill_color: [u8; 4],
    tolerance: i32,
    contiguous: bool,
    overwrite: bool,
) {
    if sx >= width || sy >= height {
        return;
    }
    if let Some(sel) = selection {
        if !sel.contains(sx, sy) {
            return;
        }
    }
    let start_idx = ((sy * width + sx) * 4) as usize;
    let target = [
        pixels[start_idx],
        pixels[start_idx + 1],
        pixels[start_idx + 2],
        pixels[start_idx + 3],
    ];
    if target == fill_color && tolerance == 0 {
        return;
    }

    if !contiguous {
        for y in 0..height {
            for x in 0..width {
                if selection.map_or(true, |s| s.contains(x, y)) {
                    let idx = ((y * width + x) * 4) as usize;
                    let cur = [
                        pixels[idx],
                        pixels[idx + 1],
                        pixels[idx + 2],
                        pixels[idx + 3],
                    ];
                    if color_within_tolerance(cur, target, tolerance) {
                        let out = paint_over(cur, fill_color, overwrite);
                        pixels[idx..idx + 4].copy_from_slice(&out);
                    }
                }
            }
        }
        return;
    }

    let mut visited = vec![false; (width * height) as usize];
    let mut q = VecDeque::new();
    q.push_back((sx, sy));
    visited[(sy * width + sx) as usize] = true;

    while let Some((x, y)) = q.pop_front() {
        let idx = ((y * width + x) * 4) as usize;
        let cur = [
            pixels[idx],
            pixels[idx + 1],
            pixels[idx + 2],
            pixels[idx + 3],
        ];
        let out = paint_over(cur, fill_color, overwrite);
        pixels[idx..idx + 4].copy_from_slice(&out);

        let neighbors = [
            (x.wrapping_sub(1), y, x > 0),
            (x + 1, y, x + 1 < width),
            (x, y.wrapping_sub(1), y > 0),
            (x, y + 1, y + 1 < height),
        ];
        for (nx, ny, ok) in neighbors {
            if !ok {
                continue;
            }
            let vi = (ny * width + nx) as usize;
            if visited[vi] {
                continue;
            }
            if let Some(sel) = selection {
                if !sel.contains(nx, ny) {
                    continue;
                }
            }
            let ni = vi * 4;
            let nc = [pixels[ni], pixels[ni + 1], pixels[ni + 2], pixels[ni + 3]];
            if color_within_tolerance(nc, target, tolerance) {
                visited[vi] = true;
                q.push_back((nx, ny));
            }
        }
    }
}

/// Magic Wand selection mask (contiguous BFS or global).
pub fn magic_wand_mask(
    pixels: &[u8],
    width: u32,
    height: u32,
    sx: u32,
    sy: u32,
    tolerance: i32,
    contiguous: bool,
) -> SelectionMask {
    let mut mask = SelectionMask::empty(width, height);
    if sx >= width || sy >= height {
        return mask;
    }
    let start_idx = ((sy * width + sx) * 4) as usize;
    let target = [
        pixels[start_idx],
        pixels[start_idx + 1],
        pixels[start_idx + 2],
        pixels[start_idx + 3],
    ];

    if !contiguous {
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                let cur = [
                    pixels[idx],
                    pixels[idx + 1],
                    pixels[idx + 2],
                    pixels[idx + 3],
                ];
                if color_within_tolerance(cur, target, tolerance) {
                    mask.mask[(y * width + x) as usize] = 255;
                }
            }
        }
        return mask;
    }

    let mut visited = vec![false; (width * height) as usize];
    let mut q = VecDeque::new();
    q.push_back((sx, sy));
    visited[(sy * width + sx) as usize] = true;
    while let Some((x, y)) = q.pop_front() {
        mask.mask[(y * width + x) as usize] = 255;
        let neighbors = [
            (x.wrapping_sub(1), y, x > 0),
            (x + 1, y, x + 1 < width),
            (x, y.wrapping_sub(1), y > 0),
            (x, y + 1, y + 1 < height),
        ];
        for (nx, ny, ok) in neighbors {
            if !ok {
                continue;
            }
            let vi = (ny * width + nx) as usize;
            if visited[vi] {
                continue;
            }
            let ni = vi * 4;
            let nc = [pixels[ni], pixels[ni + 1], pixels[ni + 2], pixels[ni + 3]];
            if color_within_tolerance(nc, target, tolerance) {
                visited[vi] = true;
                q.push_back((nx, ny));
            }
        }
    }
    mask
}

/// Render a gradient onto `pixels` respecting selection mask.
pub fn render_gradient(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    p0: [f32; 2],
    p1: [f32; 2],
    c0: [u8; 4],
    c1: [u8; 4],
    kind: GradientKind,
    mode: GradientColorMode,
) {
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let len2 = (dx * dx + dy * dy).max(1e-4);
    let len = len2.sqrt();
    let base_angle = dy.atan2(dx);

    for y in 0..height {
        let fy = y as f32 + 0.5;
        for x in 0..width {
            if let Some(sel) = selection {
                if !sel.contains(x, y) {
                    continue;
                }
            }
            let fx = x as f32 + 0.5;
            let vx = fx - p0[0];
            let vy = fy - p0[1];
            let t = match kind {
                GradientKind::Linear => ((vx * dx + vy * dy) / len2).clamp(0.0, 1.0),
                GradientKind::LinearReflected => {
                    ((vx * dx + vy * dy) / len2).abs().clamp(0.0, 1.0)
                }
                GradientKind::Radial => ((vx * vx + vy * vy).sqrt() / len).clamp(0.0, 1.0),
                GradientKind::Diamond => {
                    let proj = (vx * dx + vy * dy).abs() / len2;
                    let perp = (-vx * dy + vy * dx).abs() / len2;
                    (proj + perp).clamp(0.0, 1.0)
                }
                GradientKind::Conical => {
                    let ang = vy.atan2(vx) - base_angle;
                    let mut norm = ang / std::f32::consts::PI;
                    while norm < -1.0 {
                        norm += 2.0;
                    }
                    while norm > 1.0 {
                        norm -= 2.0;
                    }
                    norm.abs().clamp(0.0, 1.0)
                }
            };

            let idx = ((y * width + x) * 4) as usize;
            match mode {
                GradientColorMode::Color => {
                    let src = [
                        (c0[0] as f32 * (1.0 - t) + c1[0] as f32 * t + 0.5) as u8,
                        (c0[1] as f32 * (1.0 - t) + c1[1] as f32 * t + 0.5) as u8,
                        (c0[2] as f32 * (1.0 - t) + c1[2] as f32 * t + 0.5) as u8,
                        (c0[3] as f32 * (1.0 - t) + c1[3] as f32 * t + 0.5) as u8,
                    ];
                    let dst = [
                        pixels[idx],
                        pixels[idx + 1],
                        pixels[idx + 2],
                        pixels[idx + 3],
                    ];
                    let out = paint_over(dst, src, false);
                    pixels[idx..idx + 4].copy_from_slice(&out);
                }
                GradientColorMode::Transparency => {
                    let a = pixels[idx + 3] as f32 * (1.0 - t);
                    pixels[idx + 3] = a.round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

/// Sample average color in an `n x n` box around `(cx, cy)`.
pub fn sample_color(pixels: &[u8], width: u32, height: u32, cx: i32, cy: i32, size: i32) -> [u8; 4] {
    let half = (size / 2).max(0);
    let mut r = 0u64;
    let mut g = 0u64;
    let mut b = 0u64;
    let mut a = 0u64;
    let mut count = 0u64;
    for dy in -half..=half {
        let y = cy + dy;
        if y < 0 || y >= height as i32 {
            continue;
        }
        for dx in -half..=half {
            let x = cx + dx;
            if x < 0 || x >= width as i32 {
                continue;
            }
            let idx = ((y as u32 * width + x as u32) * 4) as usize;
            r += pixels[idx] as u64;
            g += pixels[idx + 1] as u64;
            b += pixels[idx + 2] as u64;
            a += pixels[idx + 3] as u64;
            count += 1;
        }
    }
    if count == 0 {
        return [0, 0, 0, 255];
    }
    [
        (r / count) as u8,
        (g / count) as u8,
        (b / count) as u8,
        (a / count) as u8,
    ]
}

/// Draw a shape (Rectangle, RoundedRectangle, Ellipse, Line) into `pixels`.
pub fn draw_shape(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    tool: ToolKind,
    p0: [f32; 2],
    p1: [f32; 2],
    stroke_color: [u8; 4],
    fill_color: [u8; 4],
    settings: &ToolSettings,
    shift_held: bool,
) {
    let mut p1 = p1;
    if shift_held {
        if tool == ToolKind::LineCurve {
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            let len = (dx * dx + dy * dy).sqrt();
            let ang = dy.atan2(dx);
            let snap = (ang / (std::f32::consts::PI / 12.0)).round() * (std::f32::consts::PI / 12.0);
            p1 = [p0[0] + len * snap.cos(), p0[1] + len * snap.sin()];
        } else {
            let dx = p1[0] - p0[0];
            let dy = p1[1] - p0[1];
            let s = dx.abs().max(dy.abs());
            p1 = [
                p0[0] + s * if dx >= 0.0 { 1.0 } else { -1.0 },
                p0[1] + s * if dy >= 0.0 { 1.0 } else { -1.0 },
            ];
        }
    }

    let bw = settings.brush_width.max(1) as f32;
    if tool == ToolKind::LineCurve {
        stroke_segment(
            pixels,
            width,
            height,
            selection,
            p0,
            p1,
            bw,
            stroke_color,
            settings.antialiasing,
            settings.overwrite_blend,
            false,
        );
        return;
    }

    let min_x = p0[0].min(p1[0]);
    let max_x = p0[0].max(p1[0]);
    let min_y = p0[1].min(p1[1]);
    let max_y = p0[1].max(p1[1]);
    let do_fill = matches!(
        settings.fill_style,
        ShapeFillStyle::Fill | ShapeFillStyle::OutlineAndFill
    );
    let do_outline = matches!(
        settings.fill_style,
        ShapeFillStyle::Outline | ShapeFillStyle::OutlineAndFill
    );
    let fill_col = if settings.fill_style == ShapeFillStyle::Fill {
        stroke_color
    } else {
        fill_color
    };

    let y_start = ((min_y - bw - 2.0).floor() as i32).max(0) as u32;
    let y_end = ((max_y + bw + 2.0).ceil() as i32).min(height as i32 - 1);
    let x_start = ((min_x - bw - 2.0).floor() as i32).max(0) as u32;
    let x_end = ((max_x + bw + 2.0).ceil() as i32).min(width as i32 - 1);
    if y_end < y_start as i32 || x_end < x_start as i32 {
        return;
    }

    let cx = (min_x + max_x) * 0.5;
    let cy = (min_y + max_y) * 0.5;
    let hx = ((max_x - min_x) * 0.5).max(0.5);
    let hy = ((max_y - min_y) * 0.5).max(0.5);
    let r = if tool == ToolKind::RoundedRectangle {
        (settings.corner_radius as f32).min(hx).min(hy).max(0.0)
    } else {
        0.0
    };

    for y in y_start..=y_end as u32 {
        let fy = y as f32 + 0.5;
        for x in x_start..=x_end as u32 {
            if let Some(sel) = selection {
                if !sel.contains(x, y) {
                    continue;
                }
            }
            let fx = x as f32 + 0.5;
            let sdf = match tool {
                ToolKind::Rectangle | ToolKind::RoundedRectangle => {
                    let qx = (fx - cx).abs() - (hx - r);
                    let qy = (fy - cy).abs() - (hy - r);
                    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
                    let inside = qx.max(qy).min(0.0);
                    outside + inside - r
                }
                ToolKind::Ellipse => {
                    let nx = (fx - cx) / hx;
                    let ny = (fy - cy) / hy;
                    let k0 = (nx * nx + ny * ny).sqrt();
                    if k0 < 1e-6 {
                        -hx.min(hy)
                    } else {
                        let gx = nx / hx;
                        let gy = ny / hy;
                        let grad = (gx * gx + gy * gy).sqrt().max(1e-6);
                        (k0 - 1.0) / grad
                    }
                }
                _ => 1e9,
            };

            let idx = ((y * width + x) * 4) as usize;
            if do_fill {
                let cov = if settings.antialiasing {
                    (0.5 - sdf).clamp(0.0, 1.0)
                } else if sdf <= 0.0 {
                    1.0
                } else {
                    0.0
                };
                if cov > 0.0 {
                    let dst = [
                        pixels[idx],
                        pixels[idx + 1],
                        pixels[idx + 2],
                        pixels[idx + 3],
                    ];
                    let mut c = fill_col;
                    c[3] = (c[3] as f32 * cov).round().clamp(0.0, 255.0) as u8;
                    let out = paint_over(dst, c, settings.overwrite_blend && cov >= 0.99);
                    pixels[idx..idx + 4].copy_from_slice(&out);
                }
            }
            if do_outline {
                let half_bw = bw * 0.5;
                let dist_edge = sdf.abs() - half_bw;
                let cov = if settings.antialiasing {
                    (0.5 - dist_edge).clamp(0.0, 1.0)
                } else if dist_edge <= 0.0 {
                    1.0
                } else {
                    0.0
                };
                if cov > 0.0 {
                    let dst = [
                        pixels[idx],
                        pixels[idx + 1],
                        pixels[idx + 2],
                        pixels[idx + 3],
                    ];
                    let mut c = stroke_color;
                    c[3] = (c[3] as f32 * cov).round().clamp(0.0, 255.0) as u8;
                    let out = paint_over(dst, c, settings.overwrite_blend && cov >= 0.99);
                    pixels[idx..idx + 4].copy_from_slice(&out);
                }
            }
        }
    }
}

/// Draw a closed freeform polygon shape into `pixels`.
pub fn draw_freeform_shape(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    pts: &[(f32, f32)],
    stroke_color: [u8; 4],
    fill_color: [u8; 4],
    settings: &ToolSettings,
) {
    if pts.len() < 2 {
        return;
    }
    let do_fill = matches!(
        settings.fill_style,
        ShapeFillStyle::Fill | ShapeFillStyle::OutlineAndFill
    );
    let do_outline = matches!(
        settings.fill_style,
        ShapeFillStyle::Outline | ShapeFillStyle::OutlineAndFill
    );
    let fill_col = if settings.fill_style == ShapeFillStyle::Fill {
        stroke_color
    } else {
        fill_color
    };

    if do_fill && pts.len() >= 3 {
        let poly_mask = SelectionMask::from_polygon(width, height, pts);
        for y in 0..height {
            for x in 0..width {
                if poly_mask.contains(x, y) && selection.map_or(true, |s| s.contains(x, y)) {
                    let idx = ((y * width + x) * 4) as usize;
                    let dst = [
                        pixels[idx],
                        pixels[idx + 1],
                        pixels[idx + 2],
                        pixels[idx + 3],
                    ];
                    let out = paint_over(dst, fill_col, settings.overwrite_blend);
                    pixels[idx..idx + 4].copy_from_slice(&out);
                }
            }
        }
    }

    if do_outline {
        let bw = settings.brush_width.max(1) as f32;
        for i in 0..pts.len() {
            let a = pts[i];
            let b = pts[(i + 1) % pts.len()];
            stroke_segment(
                pixels,
                width,
                height,
                selection,
                [a.0, a.1],
                [b.0, b.1],
                bw,
                stroke_color,
                settings.antialiasing,
                settings.overwrite_blend,
                false,
            );
        }
    }
}

/// Clone Stamp stroke segment.
pub fn clone_stamp_segment(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    p0: [f32; 2],
    p1: [f32; 2],
    offset: [i32; 2],
    diameter: f32,
) {
    let radius = (diameter * 0.5).max(1.0);
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let dist = (dx * dx + dy * dy).sqrt();
    let step = (diameter * 0.25).clamp(1.0, 4.0);
    let steps = (dist / step).ceil().max(1.0) as i32;
    let src_snapshot = pixels.to_vec();

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = p0[0] + dx * t;
        let cy = p0[1] + dy * t;
        let x0 = ((cx - radius).floor() as i32).max(0) as u32;
        let x1 = ((cx + radius).ceil() as i32).min(width as i32 - 1);
        let y0 = ((cy - radius).floor() as i32).max(0) as u32;
        let y1 = ((cy + radius).ceil() as i32).min(height as i32 - 1);
        if x1 < x0 as i32 || y1 < y0 as i32 {
            continue;
        }
        for y in y0..=y1 as u32 {
            let sy = y as i32 + offset[1];
            if sy < 0 || sy >= height as i32 {
                continue;
            }
            let dy_p = (y as f32 + 0.5) - cy;
            for x in x0..=x1 as u32 {
                let sx = x as i32 + offset[0];
                if sx < 0 || sx >= width as i32 {
                    continue;
                }
                if let Some(sel) = selection {
                    if !sel.contains(x, y) {
                        continue;
                    }
                }
                let dx_p = (x as f32 + 0.5) - cx;
                if dx_p * dx_p + dy_p * dy_p <= radius * radius {
                    let s_idx = ((sy as u32 * width + sx as u32) * 4) as usize;
                    let d_idx = ((y * width + x) * 4) as usize;
                    pixels[d_idx..d_idx + 4].copy_from_slice(&src_snapshot[s_idx..s_idx + 4]);
                }
            }
        }
    }
}

/// Recolor brush: replaces `target_color` (secondary) with `new_color` (primary) within `tolerance`.
pub fn recolor_segment(
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    p0: [f32; 2],
    p1: [f32; 2],
    diameter: f32,
    new_color: [u8; 4],
    target_color: [u8; 4],
    tolerance: i32,
) {
    let radius = (diameter * 0.5).max(1.0);
    let dx = p1[0] - p0[0];
    let dy = p1[1] - p0[1];
    let dist = (dx * dx + dy * dy).sqrt();
    let step = (diameter * 0.25).clamp(1.0, 4.0);
    let steps = (dist / step).ceil().max(1.0) as i32;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let cx = p0[0] + dx * t;
        let cy = p0[1] + dy * t;
        let x0 = ((cx - radius).floor() as i32).max(0) as u32;
        let x1 = ((cx + radius).ceil() as i32).min(width as i32 - 1);
        let y0 = ((cy - radius).floor() as i32).max(0) as u32;
        let y1 = ((cy + radius).ceil() as i32).min(height as i32 - 1);
        if x1 < x0 as i32 || y1 < y0 as i32 {
            continue;
        }
        for y in y0..=y1 as u32 {
            let dy_p = (y as f32 + 0.5) - cy;
            for x in x0..=x1 as u32 {
                if let Some(sel) = selection {
                    if !sel.contains(x, y) {
                        continue;
                    }
                }
                let dx_p = (x as f32 + 0.5) - cx;
                if dx_p * dx_p + dy_p * dy_p <= radius * radius {
                    let idx = ((y * width + x) * 4) as usize;
                    let cur = [
                        pixels[idx],
                        pixels[idx + 1],
                        pixels[idx + 2],
                        pixels[idx + 3],
                    ];
                    if color_within_tolerance(cur, target_color, tolerance.max(5)) {
                        pixels[idx] = new_color[0];
                        pixels[idx + 1] = new_color[1];
                        pixels[idx + 2] = new_color[2];
                        pixels[idx + 3] = cur[3];
                    }
                }
            }
        }
    }
}

/// Begin Move Selected Pixels by lifting either the selection or whole active layer.
pub fn begin_move_pixels(doc: &Document, start: [i32; 2]) -> FloatingMoveState {
    let w = doc.width;
    let h = doc.height;
    let layer_px = &doc.active_layer_ref().pixels;
    let mut base = layer_px.clone();
    let mut lifted = vec![0u8; layer_px.len()];

    match &doc.selection {
        Some(sel) => {
            for y in 0..h {
                for x in 0..w {
                    if sel.contains(x, y) {
                        let idx = ((y * w + x) * 4) as usize;
                        lifted[idx..idx + 4].copy_from_slice(&layer_px[idx..idx + 4]);
                        base[idx..idx + 4].copy_from_slice(&[0, 0, 0, 0]);
                    }
                }
            }
        }
        None => {
            lifted.copy_from_slice(layer_px);
            base.fill(0);
        }
    }

    FloatingMoveState {
        base_pixels: base,
        lifted_pixels: lifted,
        orig_selection: doc.selection.clone(),
        start_pt: start,
        offset: [0, 0],
    }
}

pub fn compose_move_pixels(state: &FloatingMoveState, width: u32, height: u32) -> Vec<u8> {
    let mut out = state.base_pixels.clone();
    let dx = state.offset[0];
    let dy = state.offset[1];
    for y in 0..height as i32 {
        let ny = y + dy;
        if ny < 0 || ny >= height as i32 {
            continue;
        }
        for x in 0..width as i32 {
            let nx = x + dx;
            if nx < 0 || nx >= width as i32 {
                continue;
            }
            let s = ((y as u32 * width + x as u32) * 4) as usize;
            let src = [
                state.lifted_pixels[s],
                state.lifted_pixels[s + 1],
                state.lifted_pixels[s + 2],
                state.lifted_pixels[s + 3],
            ];
            if src[3] == 0 {
                continue;
            }
            let d = ((ny as u32 * width + nx as u32) * 4) as usize;
            let dst = [out[d], out[d + 1], out[d + 2], out[d + 3]];
            let c = composite_pixel(dst, src, 1.0, BlendMode::Normal);
            out[d..d + 4].copy_from_slice(&c);
        }
    }
    out
}

/// Rasterize text into `pixels` using egui's loaded font atlas (supports ASCII, Unicode, Korean).
pub fn rasterize_text_with_egui(
    ctx: &eframe::egui::Context,
    pixels: &mut [u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    pos: [f32; 2],
    text: &str,
    font_size: f32,
    bold: bool,
    color: [u8; 4],
) {
    use eframe::egui::{Color32, FontId};
    if text.is_empty() {
        return;
    }
    let font_id = FontId::proportional(font_size.clamp(6.0, 200.0));
    let galley = ctx.fonts(|f| f.layout_no_wrap(text.to_string(), font_id, Color32::WHITE));
    let (atlas_w, atlas_h, atlas_pixels) = ctx.fonts(|f| {
        let img = f.image();
        (img.width(), img.height(), img.pixels.clone())
    });

    let passes: &[(f32, f32)] = if bold {
        &[(0.0, 0.0), (1.0, 0.0)]
    } else {
        &[(0.0, 0.0)]
    };

    for &(ox, oy) in passes {
        for row in &galley.rows {
            for glyph in &row.glyphs {
                let uv = glyph.uv_rect;
                if uv.size[0] <= 0.0 || uv.size[1] <= 0.0 {
                    continue;
                }
                let gx = pos[0] + glyph.pos.x + uv.offset.x + ox;
                let gy = pos[1] + glyph.pos.y + uv.offset.y + oy;
                let gw = uv.size[0].round() as usize;
                let gh = uv.size[1].round() as usize;
                let ux0 = uv.min[0] as usize;
                let uy0 = uv.min[1] as usize;

                for dy in 0..gh {
                    let py = (gy.round() as i32) + dy as i32;
                    if py < 0 || py >= height as i32 {
                        continue;
                    }
                    let ay = uy0 + dy;
                    if ay >= atlas_h {
                        continue;
                    }
                    for dx in 0..gw {
                        let px = (gx.round() as i32) + dx as i32;
                        if px < 0 || px >= width as i32 {
                            continue;
                        }
                        let ax = ux0 + dx;
                        if ax >= atlas_w {
                            continue;
                        }
                        let cov = atlas_pixels[ay * atlas_w + ax];
                        if cov <= 0.01 {
                            continue;
                        }
                        let ux = px as u32;
                        let uy = py as u32;
                        if let Some(sel) = selection {
                            if !sel.contains(ux, uy) {
                                continue;
                            }
                        }
                        let idx = ((uy * width + ux) * 4) as usize;
                        let dst = [
                            pixels[idx],
                            pixels[idx + 1],
                            pixels[idx + 2],
                            pixels[idx + 3],
                        ];
                        let mut src = color;
                        src[3] = ((color[3] as f32) * cov).round().clamp(0.0, 255.0) as u8;
                        let out = paint_over(dst, src, false);
                        pixels[idx..idx + 4].copy_from_slice(&out);
                    }
                }
            }
        }
    }
}
