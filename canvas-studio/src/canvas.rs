use std::path::PathBuf;

use crate::blend::{composite_pixel, BlendMode};
use crate::history::{DocumentSnapshot, HistoryStack};

/// Exact 23 preset zoom levels from `Pinta.Core.ViewActions::.cctor` (RVA 0x6d80).
pub const ZOOM_LEVELS: [f32; 23] = [
    36.0, 24.0, 16.0, 12.0, 8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.75, 1.5, 1.25, 1.0, 0.66, 0.5,
    0.33, 0.25, 0.16, 0.12, 0.08, 0.05,
];

/// Default startup canvas size (`Pinta.MainClass::OpenFilesFromCommandLine` RVA 0x5e64).
pub const DEFAULT_CANVAS_WIDTH: u32 = 800;
pub const DEFAULT_CANVAS_HEIGHT: u32 = 600;

/// New Image dialog presets (`Pinta.NewImageDialog::.cctor` RVA 0x4230).
pub const IMAGE_PRESETS: [(&str, u32, u32); 4] = [
    ("640 x 480", 640, 480),
    ("800 x 600", 800, 600),
    ("1024 x 768", 1024, 768),
    ("1600 x 1200", 1600, 1200),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackgroundFill {
    White,
    SecondaryColor,
    Transparent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResamplingMode {
    Bilinear,
    NearestNeighbor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

#[derive(Clone)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: f32, // 0.0..=1.0
    pub blend_mode: BlendMode,
    pub pixels: Vec<u8>, // width * height * 4 (RGBA)
}

impl Layer {
    pub fn new_filled(name: impl Into<String>, width: u32, height: u32, color: [u8; 4]) -> Self {
        let count = (width as usize) * (height as usize);
        let mut pixels = Vec::with_capacity(count * 4);
        for _ in 0..count {
            pixels.extend_from_slice(&color);
        }
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            pixels,
        }
    }

    pub fn new_transparent(name: impl Into<String>, width: u32, height: u32) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            pixels: vec![0u8; (width as usize) * (height as usize) * 4],
        }
    }

    #[inline]
    pub fn get_pixel(&self, width: u32, x: u32, y: u32) -> [u8; 4] {
        let idx = ((y as usize) * (width as usize) + (x as usize)) * 4;
        [
            self.pixels[idx],
            self.pixels[idx + 1],
            self.pixels[idx + 2],
            self.pixels[idx + 3],
        ]
    }

    #[inline]
    pub fn set_pixel(&mut self, width: u32, x: u32, y: u32, c: [u8; 4]) {
        let idx = ((y as usize) * (width as usize) + (x as usize)) * 4;
        self.pixels[idx] = c[0];
        self.pixels[idx + 1] = c[1];
        self.pixels[idx + 2] = c[2];
        self.pixels[idx + 3] = c[3];
    }

    pub fn make_thumbnail(&self, width: u32, height: u32, max_dim: u32) -> (u32, u32, Vec<u8>) {
        let scale = (max_dim as f32 / width.max(height).max(1) as f32).min(1.0);
        let tw = ((width as f32 * scale).round() as u32).max(1);
        let th = ((height as f32 * scale).round() as u32).max(1);
        let mut out = vec![0u8; (tw * th * 4) as usize];
        for ty in 0..th {
            let sy = ((ty as f32 + 0.5) * height as f32 / th as f32) as u32;
            let sy = sy.min(height.saturating_sub(1));
            for tx in 0..tw {
                let sx = ((tx as f32 + 0.5) * width as f32 / tw as f32) as u32;
                let sx = sx.min(width.saturating_sub(1));
                let p = self.get_pixel(width, sx, sy);
                // Composite over small checkerboard for thumbnail clarity
                let checker = if ((tx / 4) + (ty / 4)) % 2 == 0 {
                    215u8
                } else {
                    175u8
                };
                let c = composite_pixel(
                    [checker, checker, checker, 255],
                    p,
                    1.0,
                    BlendMode::Normal,
                );
                let o = ((ty * tw + tx) * 4) as usize;
                out[o..o + 4].copy_from_slice(&c);
            }
        }
        (tw, th, out)
    }
}

#[derive(Clone, Debug)]
pub struct SelectionMask {
    pub width: u32,
    pub height: u32,
    /// 0 = unselected, 255 = selected
    pub mask: Vec<u8>,
}

impl SelectionMask {
    pub fn empty(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            mask: vec![0u8; (width as usize) * (height as usize)],
        }
    }

    pub fn full(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            mask: vec![255u8; (width as usize) * (height as usize)],
        }
    }

    pub fn from_rect(width: u32, height: u32, x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        let mut s = Self::empty(width, height);
        let min_x = x0.min(x1).max(0) as u32;
        let max_x = x0.max(x1).min(width as i32 - 1);
        let min_y = y0.min(y1).max(0) as u32;
        let max_y = y0.max(y1).min(height as i32 - 1);
        if max_x >= min_x as i32 && max_y >= min_y as i32 {
            for y in min_y..=max_y as u32 {
                let row = (y * width) as usize;
                for x in min_x..=max_x as u32 {
                    s.mask[row + x as usize] = 255;
                }
            }
        }
        s
    }

    pub fn from_ellipse(width: u32, height: u32, x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        let mut s = Self::empty(width, height);
        let min_x = x0.min(x1) as f32;
        let max_x = x0.max(x1) as f32;
        let min_y = y0.min(y1) as f32;
        let max_y = y0.max(y1) as f32;
        let cx = (min_x + max_x) * 0.5;
        let cy = (min_y + max_y) * 0.5;
        let rx = ((max_x - min_x) * 0.5).max(0.5);
        let ry = ((max_y - min_y) * 0.5).max(0.5);
        let y_start = (min_y.floor() as i32).max(0) as u32;
        let y_end = (max_y.ceil() as i32).min(height as i32 - 1);
        let x_start = (min_x.floor() as i32).max(0) as u32;
        let x_end = (max_x.ceil() as i32).min(width as i32 - 1);
        if y_end >= y_start as i32 && x_end >= x_start as i32 {
            for y in y_start..=y_end as u32 {
                let dy = (y as f32 + 0.5 - cy) / ry;
                let dy2 = dy * dy;
                let row = (y * width) as usize;
                for x in x_start..=x_end as u32 {
                    let dx = (x as f32 + 0.5 - cx) / rx;
                    if dx * dx + dy2 <= 1.0 {
                        s.mask[row + x as usize] = 255;
                    }
                }
            }
        }
        s
    }

    pub fn from_polygon(width: u32, height: u32, pts: &[(f32, f32)]) -> Self {
        let mut s = Self::empty(width, height);
        if pts.len() < 3 {
            return s;
        }
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;
        for &(_, y) in pts {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        }
        let y0 = (min_y.floor() as i32).max(0) as u32;
        let y1 = (max_y.ceil() as i32).min(height as i32 - 1);
        if y1 < y0 as i32 {
            return s;
        }
        let mut nodes = Vec::with_capacity(32);
        for y in y0..=y1 as u32 {
            let fy = y as f32 + 0.5;
            nodes.clear();
            let mut j = pts.len() - 1;
            for i in 0..pts.len() {
                let (xi, yi) = pts[i];
                let (xj, yj) = pts[j];
                if (yi < fy && yj >= fy) || (yj < fy && yi >= fy) {
                    let x = xi + (fy - yi) / (yj - yi) * (xj - xi);
                    nodes.push(x);
                }
                j = i;
            }
            nodes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let row = (y * width) as usize;
            for pair in nodes.chunks_exact(2) {
                let xs = (pair[0].floor() as i32).max(0) as u32;
                let xe = (pair[1].ceil() as i32).min(width as i32 - 1);
                if xe >= xs as i32 {
                    for x in xs..=xe as u32 {
                        s.mask[row + x as usize] = 255;
                    }
                }
            }
        }
        s
    }

    #[inline]
    pub fn contains(&self, x: u32, y: u32) -> bool {
        if x >= self.width || y >= self.height {
            return false;
        }
        self.mask[(y * self.width + x) as usize] > 127
    }

    pub fn is_empty(&self) -> bool {
        !self.mask.iter().any(|&v| v > 127)
    }

    pub fn bounds(&self) -> Option<(u32, u32, u32, u32)> {
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        let mut any = false;
        for y in 0..self.height {
            let row = (y * self.width) as usize;
            for x in 0..self.width {
                if self.mask[row + x as usize] > 127 {
                    any = true;
                    if x < min_x {
                        min_x = x;
                    }
                    if x > max_x {
                        max_x = x;
                    }
                    if y < min_y {
                        min_y = y;
                    }
                    if y > max_y {
                        max_y = y;
                    }
                }
            }
        }
        if any {
            Some((min_x, min_y, max_x - min_x + 1, max_y - min_y + 1))
        } else {
            None
        }
    }

    pub fn invert(&mut self) {
        for v in &mut self.mask {
            *v = 255 - *v;
        }
    }

    pub fn translate(&self, dx: i32, dy: i32) -> Self {
        let mut out = Self::empty(self.width, self.height);
        for y in 0..self.height as i32 {
            let sy = y - dy;
            if sy < 0 || sy >= self.height as i32 {
                continue;
            }
            for x in 0..self.width as i32 {
                let sx = x - dx;
                if sx < 0 || sx >= self.width as i32 {
                    continue;
                }
                out.mask[(y as u32 * self.width + x as u32) as usize] =
                    self.mask[(sy as u32 * self.width + sx as u32) as usize];
            }
        }
        out
    }

    pub fn combine(&mut self, other: &SelectionMask, mode: SelectionCombineMode) {
        if self.mask.len() != other.mask.len() {
            return;
        }
        for (a, &b) in self.mask.iter_mut().zip(other.mask.iter()) {
            let sa = *a > 127;
            let sb = b > 127;
            let res = match mode {
                SelectionCombineMode::Replace => sb,
                SelectionCombineMode::Union => sa || sb,
                SelectionCombineMode::Exclude => sa && !sb,
                SelectionCombineMode::Xor => sa ^ sb,
                SelectionCombineMode::Intersect => sa && sb,
            };
            *a = if res { 255 } else { 0 };
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionCombineMode {
    Replace,
    Union,
    Exclude,
    Xor,
    Intersect,
}

impl SelectionCombineMode {
    pub const ALL: [SelectionCombineMode; 5] = [
        SelectionCombineMode::Replace,
        SelectionCombineMode::Union,
        SelectionCombineMode::Exclude,
        SelectionCombineMode::Xor,
        SelectionCombineMode::Intersect,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SelectionCombineMode::Replace => "Replace",
            SelectionCombineMode::Union => "Union (+)",
            SelectionCombineMode::Exclude => "Exclude (-)",
            SelectionCombineMode::Xor => "Xor",
            SelectionCombineMode::Intersect => "Intersect",
        }
    }
}

#[derive(Clone)]
pub struct ClipboardPixels {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub struct Document {
    pub id: u64,
    pub title: String,
    pub file_path: Option<PathBuf>,
    pub dirty: bool,
    pub width: u32,
    pub height: u32,
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    pub next_layer_num: usize,
    pub selection: Option<SelectionMask>,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub history: HistoryStack,
    pub composite_rgba: Vec<u8>,
    pub composite_dirty: bool,
    pub version: u64,
    /// Temporary floating overlay (used for live preview of shapes/gradients/effects/move-pixels)
    pub preview_layer_pixels: Option<(usize, Vec<u8>)>,
}

impl Document {
    pub fn new(id: u64, title: impl Into<String>, width: u32, height: u32, bg: [u8; 4]) -> Self {
        let width = width.clamp(1, 8192);
        let height = height.clamp(1, 8192);
        let bg_layer = Layer::new_filled("Background", width, height, bg);
        let layers = vec![bg_layer];
        let snap = DocumentSnapshot {
            width,
            height,
            layers: layers.clone(),
            active_layer: 0,
            selection: None,
        };
        let mut doc = Self {
            id,
            title: title.into(),
            file_path: None,
            dirty: false,
            width,
            height,
            layers,
            active_layer: 0,
            next_layer_num: 2,
            selection: None,
            zoom: 1.0,
            pan: [0.0, 0.0],
            history: HistoryStack::new("Open Image", snap),
            composite_rgba: vec![0u8; (width as usize) * (height as usize) * 4],
            composite_dirty: true,
            version: 1,
            preview_layer_pixels: None,
        };
        doc.recomposite();
        doc
    }

    pub fn from_rgba(
        id: u64,
        title: impl Into<String>,
        file_path: Option<PathBuf>,
        width: u32,
        height: u32,
        pixels: Vec<u8>,
    ) -> Self {
        let layer = Layer {
            name: "Background".to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            pixels,
        };
        let layers = vec![layer];
        let snap = DocumentSnapshot {
            width,
            height,
            layers: layers.clone(),
            active_layer: 0,
            selection: None,
        };
        let mut doc = Self {
            id,
            title: title.into(),
            file_path,
            dirty: false,
            width,
            height,
            layers,
            active_layer: 0,
            next_layer_num: 2,
            selection: None,
            zoom: 1.0,
            pan: [0.0, 0.0],
            history: HistoryStack::new("Open Image", snap),
            composite_rgba: vec![0u8; (width as usize) * (height as usize) * 4],
            composite_dirty: true,
            version: 1,
            preview_layer_pixels: None,
        };
        doc.recomposite();
        doc
    }

    pub fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            width: self.width,
            height: self.height,
            layers: self.layers.clone(),
            active_layer: self.active_layer,
            selection: self.selection.clone(),
        }
    }

    pub fn commit_history(&mut self, label: impl Into<String>) {
        let snap = self.snapshot();
        self.history.push(label, snap);
        self.dirty = true;
        self.mark_dirty();
    }

    pub fn restore_snapshot(&mut self, snap: DocumentSnapshot) {
        self.width = snap.width;
        self.height = snap.height;
        self.layers = snap.layers;
        self.active_layer = snap.active_layer.min(self.layers.len().saturating_sub(1));
        self.selection = snap.selection;
        self.preview_layer_pixels = None;
        self.mark_dirty();
    }

    pub fn undo(&mut self) -> bool {
        if let Some(snap) = self.history.undo().cloned() {
            self.restore_snapshot(snap);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(snap) = self.history.redo().cloned() {
            self.restore_snapshot(snap);
            true
        } else {
            false
        }
    }

    pub fn jump_history(&mut self, idx: usize) -> bool {
        if let Some(snap) = self.history.jump_to(idx).cloned() {
            self.restore_snapshot(snap);
            true
        } else {
            false
        }
    }

    pub fn mark_dirty(&mut self) {
        self.composite_dirty = true;
        self.version = self.version.wrapping_add(1);
    }

    #[inline]
    pub fn active_layer_ref(&self) -> &Layer {
        &self.layers[self.active_layer]
    }

    #[inline]
    pub fn active_layer_mut(&mut self) -> &mut Layer {
        &mut self.layers[self.active_layer]
    }

    #[inline]
    pub fn pixel_selected(&self, x: u32, y: u32) -> bool {
        match &self.selection {
            Some(sel) => sel.contains(x, y),
            None => true,
        }
    }

    pub fn recomposite(&mut self) {
        let count = (self.width as usize) * (self.height as usize);
        if self.composite_rgba.len() != count * 4 {
            self.composite_rgba.resize(count * 4, 0);
        } else {
            self.composite_rgba.fill(0);
        }

        for (li, layer) in self.layers.iter().enumerate() {
            if !layer.visible || layer.opacity <= 0.0001 {
                continue;
            }
            let src_pixels: &[u8] = match &self.preview_layer_pixels {
                Some((pi, buf)) if *pi == li && buf.len() == count * 4 => buf.as_slice(),
                _ => layer.pixels.as_slice(),
            };
            if src_pixels.len() != count * 4 {
                continue;
            }
            let opacity = layer.opacity;
            let mode = layer.blend_mode;
            for i in 0..count {
                let o = i * 4;
                let src = [
                    src_pixels[o],
                    src_pixels[o + 1],
                    src_pixels[o + 2],
                    src_pixels[o + 3],
                ];
                if src[3] == 0 {
                    continue;
                }
                let dst = [
                    self.composite_rgba[o],
                    self.composite_rgba[o + 1],
                    self.composite_rgba[o + 2],
                    self.composite_rgba[o + 3],
                ];
                let out = composite_pixel(dst, src, opacity, mode);
                self.composite_rgba[o] = out[0];
                self.composite_rgba[o + 1] = out[1];
                self.composite_rgba[o + 2] = out[2];
                self.composite_rgba[o + 3] = out[3];
            }
        }
        self.composite_dirty = false;
    }

    pub fn add_layer(&mut self) {
        let name = format!("Layer {}", self.next_layer_num);
        self.next_layer_num += 1;
        let layer = Layer::new_transparent(name, self.width, self.height);
        let insert_idx = (self.active_layer + 1).min(self.layers.len());
        self.layers.insert(insert_idx, layer);
        self.active_layer = insert_idx;
        self.commit_history("Add New Layer");
    }

    pub fn delete_active_layer(&mut self) {
        if self.layers.len() <= 1 {
            return;
        }
        self.layers.remove(self.active_layer);
        if self.active_layer >= self.layers.len() {
            self.active_layer = self.layers.len() - 1;
        }
        self.commit_history("Delete Layer");
    }

    pub fn duplicate_active_layer(&mut self) {
        let mut dup = self.layers[self.active_layer].clone();
        dup.name = format!("{} copy", dup.name);
        let insert_idx = self.active_layer + 1;
        self.layers.insert(insert_idx, dup);
        self.active_layer = insert_idx;
        self.commit_history("Duplicate Layer");
    }

    pub fn merge_layer_down(&mut self) {
        if self.active_layer == 0 || self.layers.len() <= 1 {
            return;
        }
        let top = self.layers.remove(self.active_layer);
        self.active_layer -= 1;
        let bottom = &mut self.layers[self.active_layer];
        if top.visible && top.opacity > 0.0001 {
            let count = (self.width as usize) * (self.height as usize);
            for i in 0..count {
                let o = i * 4;
                let dst = [
                    bottom.pixels[o],
                    bottom.pixels[o + 1],
                    bottom.pixels[o + 2],
                    bottom.pixels[o + 3],
                ];
                let src = [
                    top.pixels[o],
                    top.pixels[o + 1],
                    top.pixels[o + 2],
                    top.pixels[o + 3],
                ];
                let c = composite_pixel(dst, src, top.opacity, top.blend_mode);
                bottom.pixels[o..o + 4].copy_from_slice(&c);
            }
        }
        self.commit_history("Merge Layer Down");
    }

    pub fn move_layer_up(&mut self) {
        if self.active_layer + 1 < self.layers.len() {
            self.layers.swap(self.active_layer, self.active_layer + 1);
            self.active_layer += 1;
            self.commit_history("Move Layer Up");
        }
    }

    pub fn move_layer_down(&mut self) {
        if self.active_layer > 0 {
            self.layers.swap(self.active_layer, self.active_layer - 1);
            self.active_layer -= 1;
            self.commit_history("Move Layer Down");
        }
    }

    pub fn flatten(&mut self) {
        if self.layers.len() <= 1 {
            return;
        }
        self.recomposite();
        let flat = Layer {
            name: "Background".to_string(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            pixels: self.composite_rgba.clone(),
        };
        self.layers = vec![flat];
        self.active_layer = 0;
        self.commit_history("Flatten Image");
    }

    pub fn flip_active_layer_horizontal(&mut self) {
        let w = self.width as usize;
        let h = self.height as usize;
        let pixels = &mut self.layers[self.active_layer].pixels;
        for y in 0..h {
            for x in 0..w / 2 {
                let a = (y * w + x) * 4;
                let b = (y * w + (w - 1 - x)) * 4;
                for c in 0..4 {
                    pixels.swap(a + c, b + c);
                }
            }
        }
        self.commit_history("Flip Layer Horizontal");
    }

    pub fn flip_active_layer_vertical(&mut self) {
        let w = self.width as usize;
        let h = self.height as usize;
        let pixels = &mut self.layers[self.active_layer].pixels;
        for y in 0..h / 2 {
            let row_a = y * w * 4;
            let row_b = (h - 1 - y) * w * 4;
            for i in 0..w * 4 {
                pixels.swap(row_a + i, row_b + i);
            }
        }
        self.commit_history("Flip Layer Vertical");
    }

    pub fn flip_image_horizontal(&mut self) {
        let w = self.width as usize;
        let h = self.height as usize;
        for layer in &mut self.layers {
            for y in 0..h {
                for x in 0..w / 2 {
                    let a = (y * w + x) * 4;
                    let b = (y * w + (w - 1 - x)) * 4;
                    for c in 0..4 {
                        layer.pixels.swap(a + c, b + c);
                    }
                }
            }
        }
        self.selection = None;
        self.commit_history("Flip Horizontal");
    }

    pub fn flip_image_vertical(&mut self) {
        let w = self.width as usize;
        let h = self.height as usize;
        for layer in &mut self.layers {
            for y in 0..h / 2 {
                let row_a = y * w * 4;
                let row_b = (h - 1 - y) * w * 4;
                for i in 0..w * 4 {
                    layer.pixels.swap(row_a + i, row_b + i);
                }
            }
        }
        self.selection = None;
        self.commit_history("Flip Vertical");
    }

    pub fn rotate_90_cw(&mut self) {
        let ow = self.width as usize;
        let oh = self.height as usize;
        let nw = oh;
        let nh = ow;
        for layer in &mut self.layers {
            let mut out = vec![0u8; nw * nh * 4];
            for y in 0..oh {
                for x in 0..ow {
                    let nx = oh - 1 - y;
                    let ny = x;
                    let src = (y * ow + x) * 4;
                    let dst = (ny * nw + nx) * 4;
                    out[dst..dst + 4].copy_from_slice(&layer.pixels[src..src + 4]);
                }
            }
            layer.pixels = out;
        }
        self.width = nw as u32;
        self.height = nh as u32;
        self.selection = None;
        self.commit_history("Rotate 90° Clockwise");
    }

    pub fn rotate_90_ccw(&mut self) {
        let ow = self.width as usize;
        let oh = self.height as usize;
        let nw = oh;
        let nh = ow;
        for layer in &mut self.layers {
            let mut out = vec![0u8; nw * nh * 4];
            for y in 0..oh {
                for x in 0..ow {
                    let nx = y;
                    let ny = ow - 1 - x;
                    let src = (y * ow + x) * 4;
                    let dst = (ny * nw + nx) * 4;
                    out[dst..dst + 4].copy_from_slice(&layer.pixels[src..src + 4]);
                }
            }
            layer.pixels = out;
        }
        self.width = nw as u32;
        self.height = nh as u32;
        self.selection = None;
        self.commit_history("Rotate 90° Counter-Clockwise");
    }

    pub fn rotate_180(&mut self) {
        let ow = self.width as usize;
        let oh = self.height as usize;
        for layer in &mut self.layers {
            let mut out = vec![0u8; ow * oh * 4];
            for y in 0..oh {
                for x in 0..ow {
                    let nx = ow - 1 - x;
                    let ny = oh - 1 - y;
                    let src = (y * ow + x) * 4;
                    let dst = (ny * ow + nx) * 4;
                    out[dst..dst + 4].copy_from_slice(&layer.pixels[src..src + 4]);
                }
            }
            layer.pixels = out;
        }
        self.selection = None;
        self.commit_history("Rotate 180°");
    }

    pub fn resize_image(&mut self, new_w: u32, new_h: u32, mode: ResamplingMode) {
        let nw = new_w.clamp(1, 8192);
        let nh = new_h.clamp(1, 8192);
        let ow = self.width;
        let oh = self.height;
        for layer in &mut self.layers {
            layer.pixels = resample_rgba(&layer.pixels, ow, oh, nw, nh, mode);
        }
        self.width = nw;
        self.height = nh;
        self.selection = None;
        self.commit_history("Resize Image");
    }

    pub fn resize_canvas(&mut self, new_w: u32, new_h: u32, anchor: Anchor) {
        let nw = new_w.clamp(1, 8192);
        let nh = new_h.clamp(1, 8192);
        let ow = self.width as i32;
        let oh = self.height as i32;
        let dx = match anchor {
            Anchor::TopLeft | Anchor::Left | Anchor::BottomLeft => 0,
            Anchor::Top | Anchor::Center | Anchor::Bottom => (nw as i32 - ow) / 2,
            Anchor::TopRight | Anchor::Right | Anchor::BottomRight => nw as i32 - ow,
        };
        let dy = match anchor {
            Anchor::TopLeft | Anchor::Top | Anchor::TopRight => 0,
            Anchor::Left | Anchor::Center | Anchor::Right => (nh as i32 - oh) / 2,
            Anchor::BottomLeft | Anchor::Bottom | Anchor::BottomRight => nh as i32 - oh,
        };

        for (li, layer) in self.layers.iter_mut().enumerate() {
            let mut out = vec![0u8; (nw * nh * 4) as usize];
            if li == 0 {
                // Fill newly revealed background with white if bottom layer is opaque background
                for px in out.chunks_exact_mut(4) {
                    px.copy_from_slice(&[255, 255, 255, 255]);
                }
            }
            for sy in 0..oh {
                let ny = sy + dy;
                if ny < 0 || ny >= nh as i32 {
                    continue;
                }
                for sx in 0..ow {
                    let nx = sx + dx;
                    if nx < 0 || nx >= nw as i32 {
                        continue;
                    }
                    let src = ((sy as u32 * self.width + sx as u32) * 4) as usize;
                    let dst = ((ny as u32 * nw + nx as u32) * 4) as usize;
                    out[dst..dst + 4].copy_from_slice(&layer.pixels[src..src + 4]);
                }
            }
            layer.pixels = out;
        }
        self.width = nw;
        self.height = nh;
        self.selection = None;
        self.commit_history("Resize Canvas");
    }

    pub fn crop_to_selection(&mut self) -> bool {
        let Some(sel) = &self.selection else {
            return false;
        };
        let Some((bx, by, bw, bh)) = sel.bounds() else {
            return false;
        };
        for layer in &mut self.layers {
            let mut out = vec![0u8; (bw * bh * 4) as usize];
            for y in 0..bh {
                for x in 0..bw {
                    let sx = bx + x;
                    let sy = by + y;
                    if sel.contains(sx, sy) {
                        let src = ((sy * self.width + sx) * 4) as usize;
                        let dst = ((y * bw + x) * 4) as usize;
                        out[dst..dst + 4].copy_from_slice(&layer.pixels[src..src + 4]);
                    }
                }
            }
            layer.pixels = out;
        }
        self.width = bw;
        self.height = bh;
        self.selection = None;
        self.commit_history("Crop to Selection");
        true
    }

    pub fn erase_selection(&mut self) {
        let sel = self.selection.clone();
        let w = self.width;
        let h = self.height;
        let layer = &mut self.layers[self.active_layer];
        match sel {
            Some(s) => {
                for y in 0..h {
                    for x in 0..w {
                        if s.contains(x, y) {
                            layer.set_pixel(w, x, y, [0, 0, 0, 0]);
                        }
                    }
                }
            }
            None => {
                layer.pixels.fill(0);
            }
        }
        self.commit_history("Erase Selection");
    }

    pub fn fill_selection(&mut self, color: [u8; 4]) {
        let sel = self.selection.clone();
        let w = self.width;
        let h = self.height;
        let layer = &mut self.layers[self.active_layer];
        match sel {
            Some(s) => {
                for y in 0..h {
                    for x in 0..w {
                        if s.contains(x, y) {
                            layer.set_pixel(w, x, y, color);
                        }
                    }
                }
            }
            None => {
                for px in layer.pixels.chunks_exact_mut(4) {
                    px.copy_from_slice(&color);
                }
            }
        }
        self.commit_history("Fill Selection");
    }

    pub fn copy_selection(&mut self, merged: bool) -> ClipboardPixels {
        if merged && self.composite_dirty {
            self.recomposite();
        }
        let src_buf: &[u8] = if merged {
            &self.composite_rgba
        } else {
            &self.layers[self.active_layer].pixels
        };
        let (bx, by, bw, bh) = self
            .selection
            .as_ref()
            .and_then(|s| s.bounds())
            .unwrap_or((0, 0, self.width, self.height));
        let mut pixels = vec![0u8; (bw * bh * 4) as usize];
        for y in 0..bh {
            for x in 0..bw {
                let sx = bx + x;
                let sy = by + y;
                if self.pixel_selected(sx, sy) {
                    let s = ((sy * self.width + sx) * 4) as usize;
                    let d = ((y * bw + x) * 4) as usize;
                    pixels[d..d + 4].copy_from_slice(&src_buf[s..s + 4]);
                }
            }
        }
        ClipboardPixels {
            width: bw,
            height: bh,
            pixels,
        }
    }

    pub fn paste_clipboard(&mut self, clip: &ClipboardPixels, new_layer: bool) {
        if new_layer {
            let name = format!("Layer {}", self.next_layer_num);
            self.next_layer_num += 1;
            let insert_idx = (self.active_layer + 1).min(self.layers.len());
            self.layers.insert(
                insert_idx,
                Layer::new_transparent(name, self.width, self.height),
            );
            self.active_layer = insert_idx;
        }
        let w = self.width;
        let h = self.height;
        let cw = clip.width.min(w);
        let ch = clip.height.min(h);
        let layer = &mut self.layers[self.active_layer];
        for y in 0..ch {
            for x in 0..cw {
                let s = ((y * clip.width + x) * 4) as usize;
                let src = [
                    clip.pixels[s],
                    clip.pixels[s + 1],
                    clip.pixels[s + 2],
                    clip.pixels[s + 3],
                ];
                if src[3] > 0 {
                    let dst = layer.get_pixel(w, x, y);
                    let c = composite_pixel(dst, src, 1.0, BlendMode::Normal);
                    layer.set_pixel(w, x, y, c);
                }
            }
        }
        self.selection = Some(SelectionMask::from_rect(
            w,
            h,
            0,
            0,
            cw as i32 - 1,
            ch as i32 - 1,
        ));
        self.commit_history(if new_layer {
            "Paste into New Layer"
        } else {
            "Paste"
        });
    }

    pub fn zoom_in(&mut self) {
        // ZOOM_LEVELS is descending (36.0 -> 0.05)
        for &z in ZOOM_LEVELS.iter().rev() {
            if z > self.zoom + 0.001 {
                self.zoom = z;
                return;
            }
        }
        self.zoom = ZOOM_LEVELS[0];
    }

    pub fn zoom_out(&mut self) {
        for &z in ZOOM_LEVELS.iter() {
            if z < self.zoom - 0.001 {
                self.zoom = z;
                return;
            }
        }
        self.zoom = *ZOOM_LEVELS.last().unwrap();
    }

    pub fn zoom_to_window(&mut self, avail_w: f32, avail_h: f32) {
        // Best Fit margin: 20px (Pinta.MainWindow::ZoomToWindow_Activated RVA 0x7278)
        let mw = (avail_w - 40.0).max(32.0);
        let mh = (avail_h - 40.0).max(32.0);
        let zx = mw / self.width.max(1) as f32;
        let zy = mh / self.height.max(1) as f32;
        self.zoom = zx.min(zy).clamp(0.05, 36.0);
        self.pan = [0.0, 0.0];
    }

    pub fn import_layer_rgba(&mut self, name: String, iw: u32, ih: u32, pixels: Vec<u8>) {
        let mut layer = Layer::new_transparent(name, self.width, self.height);
        let cw = iw.min(self.width);
        let ch = ih.min(self.height);
        for y in 0..ch {
            for x in 0..cw {
                let s = ((y * iw + x) * 4) as usize;
                let d = ((y * self.width + x) * 4) as usize;
                layer.pixels[d..d + 4].copy_from_slice(&pixels[s..s + 4]);
            }
        }
        let insert_idx = (self.active_layer + 1).min(self.layers.len());
        self.layers.insert(insert_idx, layer);
        self.active_layer = insert_idx;
        self.commit_history("Import Layer");
    }
}

pub fn resample_rgba(
    src: &[u8],
    ow: u32,
    oh: u32,
    nw: u32,
    nh: u32,
    mode: ResamplingMode,
) -> Vec<u8> {
    let mut out = vec![0u8; (nw * nh * 4) as usize];
    for ny in 0..nh {
        let fy = (ny as f32 + 0.5) * (oh as f32 / nh as f32) - 0.5;
        for nx in 0..nw {
            let fx = (nx as f32 + 0.5) * (ow as f32 / nw as f32) - 0.5;
            let c = match mode {
                ResamplingMode::NearestNeighbor => {
                    let sx = (fx.round() as i32).clamp(0, ow as i32 - 1) as u32;
                    let sy = (fy.round() as i32).clamp(0, oh as i32 - 1) as u32;
                    let i = ((sy * ow + sx) * 4) as usize;
                    [src[i], src[i + 1], src[i + 2], src[i + 3]]
                }
                ResamplingMode::Bilinear => {
                    let x0 = (fx.floor() as i32).clamp(0, ow as i32 - 1) as u32;
                    let y0 = (fy.floor() as i32).clamp(0, oh as i32 - 1) as u32;
                    let x1 = (x0 + 1).min(ow - 1);
                    let y1 = (y0 + 1).min(oh - 1);
                    let tx = (fx - fx.floor()).clamp(0.0, 1.0);
                    let ty = (fy - fy.floor()).clamp(0.0, 1.0);
                    let i00 = ((y0 * ow + x0) * 4) as usize;
                    let i10 = ((y0 * ow + x1) * 4) as usize;
                    let i01 = ((y1 * ow + x0) * 4) as usize;
                    let i11 = ((y1 * ow + x1) * 4) as usize;
                    let mut res = [0u8; 4];
                    for ch in 0..4 {
                        let top = src[i00 + ch] as f32 * (1.0 - tx) + src[i10 + ch] as f32 * tx;
                        let bot = src[i01 + ch] as f32 * (1.0 - tx) + src[i11 + ch] as f32 * tx;
                        res[ch] = (top * (1.0 - ty) + bot * ty + 0.5).clamp(0.0, 255.0) as u8;
                    }
                    res
                }
            };
            let o = ((ny * nw + nx) * 4) as usize;
            out[o..o + 4].copy_from_slice(&c);
        }
    }
    out
}
