#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod blend;
mod canvas;
mod effects;
mod file_io;
mod history;
mod icons;
mod palette;
mod tools;

use std::collections::HashMap;
use std::path::PathBuf;

use eframe::egui::{
    self, Align2, Color32, ColorImage, CornerRadius, FontId, Key, Margin, Modifiers, Pos2, Rect,
    RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Vec2,
};

use blend::BlendMode;
use canvas::{
    Anchor, BackgroundFill, ClipboardPixels, Document, ResamplingMode, SelectionCombineMode,
    SelectionMask, DEFAULT_CANVAS_HEIGHT, DEFAULT_CANVAS_WIDTH, IMAGE_PRESETS, ZOOM_LEVELS,
};
use effects::{apply_effect, EffectSpec};
use palette::{color32_to_rgba, rgba_to_color32, PaletteState};
use tools::{
    begin_move_pixels, clone_stamp_segment, compose_move_pixels, draw_freeform_shape,
    draw_pencil_line, draw_shape, flood_fill, magic_wand_mask, rasterize_text_with_egui,
    recolor_segment, render_gradient, sample_color, stroke_segment, ActiveTextState,
    GradientColorMode, GradientKind, ShapeFillStyle, ToolKind, ToolRuntime, ToolSettings,
};

const WINDOW_DEFAULT_WIDTH: f32 = 1100.0;
const WINDOW_DEFAULT_HEIGHT: f32 = 750.0;
const TOOL_OPTIONS_BAR_HEIGHT: f32 = 44.0;

#[derive(Clone, Debug)]
struct NewImageDialogState {
    width: i32,
    height: i32,
    background: BackgroundFill,
}

#[derive(Clone, Debug)]
struct ResizeImageDialogState {
    by_percentage: bool,
    percentage: i32,
    width: i32,
    height: i32,
    orig_w: u32,
    orig_h: u32,
    maintain_aspect: bool,
    resampling: ResamplingMode,
}

#[derive(Clone, Debug)]
struct ResizeCanvasDialogState {
    width: i32,
    height: i32,
    anchor: Anchor,
}

#[derive(Clone, Debug)]
struct LayerPropsDialogState {
    name: String,
    visible: bool,
    opacity_pct: i32,
    blend_mode: BlendMode,
}

#[derive(Clone, Debug)]
struct GridDialogState {
    show_grid: bool,
    cell_width: i32,
    cell_height: i32,
}

struct ActiveEffectDialog {
    spec: EffectSpec,
    original_pixels: Vec<u8>,
    needs_preview: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ColorPickerTarget {
    Primary,
    Secondary,
}

struct CachedDocTexture {
    version: u64,
    nearest: bool,
    handle: TextureHandle,
}

struct CanvasStudioApp {
    documents: Vec<Document>,
    active_doc: usize,
    next_doc_id: u64,
    next_untitled_num: usize,

    active_tool: ToolKind,
    tool_settings: ToolSettings,
    tool_runtime: ToolRuntime,
    palette: PaletteState,
    clipboard: Option<ClipboardPixels>,

    // View toggles (defaults from Pinta.MainWindow::LoadUserSettings RVA 0x6d94)
    show_toolbar: bool,
    show_toolbox: bool,
    show_tool_windows: bool,
    show_statusbar: bool,
    show_image_tabs: bool,
    show_rulers: bool,
    show_grid: bool,
    grid_width: i32,
    grid_height: i32,
    dark_mode: bool,

    // Cached GPU textures
    doc_textures: HashMap<u64, CachedDocTexture>,
    thumb_textures: HashMap<(u64, usize), (u64, TextureHandle)>,

    // Cursor & status
    cursor_canvas_pos: Option<[i32; 2]>,
    last_canvas_viewport: Vec2,
    initial_fit_done: bool,
    status_message: Option<String>,

    // Dialogs
    new_image_dialog: Option<NewImageDialogState>,
    resize_image_dialog: Option<ResizeImageDialogState>,
    resize_canvas_dialog: Option<ResizeCanvasDialogState>,
    layer_props_dialog: Option<LayerPropsDialogState>,
    grid_dialog: Option<GridDialogState>,
    effect_dialog: Option<ActiveEffectDialog>,
    color_picker_target: Option<ColorPickerTarget>,
    show_about: bool,

    // Screenshot automation mode (--screenshot <path>)
    screenshot_path: Option<PathBuf>,
    frame_count: u32,
    screenshot_requested: bool,
    screenshot_saved: bool,

    #[cfg(target_arch = "wasm32")]
    egui_ctx: egui::Context,
    #[cfg(target_arch = "wasm32")]
    web_pending_files: std::rc::Rc<std::cell::RefCell<Vec<file_io::PendingWebFile>>>,
}

impl CanvasStudioApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        screenshot_path: Option<PathBuf>,
        open_file: Option<PathBuf>,
    ) -> Self {
        install_system_fallback_fonts(&cc.egui_ctx);

        let mut docs = Vec::new();
        let mut next_doc_id = 1u64;
        let mut next_untitled_num = 1usize;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = open_file {
            if let Ok((w, h, px)) = file_io::load_rgba_file(&path) {
                let title = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Image")
                    .to_string();
                docs.push(Document::from_rgba(next_doc_id, title, Some(path), w, h, px));
                next_doc_id += 1;
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = open_file;
        if docs.is_empty() {
            docs.push(Document::new(
                next_doc_id,
                format!("Image {}", next_untitled_num),
                DEFAULT_CANVAS_WIDTH,
                DEFAULT_CANVAS_HEIGHT,
                [255, 255, 255, 255],
            ));
            next_doc_id += 1;
            next_untitled_num += 1;
        }

        Self {
            documents: docs,
            active_doc: 0,
            next_doc_id,
            next_untitled_num,
            active_tool: ToolKind::Paintbrush,
            tool_settings: ToolSettings::default(),
            tool_runtime: ToolRuntime::default(),
            palette: PaletteState::default(),
            clipboard: None,
            show_toolbar: true,
            show_toolbox: true,
            show_tool_windows: true,
            show_statusbar: true,
            show_image_tabs: true,
            show_rulers: false,
            show_grid: false,
            grid_width: 64,
            grid_height: 64,
            dark_mode: false,
            doc_textures: HashMap::new(),
            thumb_textures: HashMap::new(),
            cursor_canvas_pos: None,
            last_canvas_viewport: Vec2::new(760.0, 560.0),
            initial_fit_done: false,
            status_message: None,
            new_image_dialog: None,
            resize_image_dialog: None,
            resize_canvas_dialog: None,
            layer_props_dialog: None,
            grid_dialog: None,
            effect_dialog: None,
            color_picker_target: None,
            show_about: false,
            screenshot_path,
            frame_count: 0,
            screenshot_requested: false,
            screenshot_saved: false,
            #[cfg(target_arch = "wasm32")]
            egui_ctx: cc.egui_ctx.clone(),
            #[cfg(target_arch = "wasm32")]
            web_pending_files: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
        }
    }

    #[inline]
    fn doc(&self) -> &Document {
        &self.documents[self.active_doc]
    }

    #[inline]
    fn doc_mut(&mut self) -> &mut Document {
        &mut self.documents[self.active_doc]
    }

    fn apply_theme(&self, ctx: &egui::Context) {
        let mut visuals = if self.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        if self.dark_mode {
            visuals.panel_fill = Color32::from_rgb(36, 38, 42);
            visuals.window_fill = Color32::from_rgb(42, 44, 49);
            visuals.extreme_bg_color = Color32::from_rgb(26, 28, 32);
            visuals.selection.bg_fill = Color32::from_rgb(53, 132, 228);
            visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
        } else {
            visuals.panel_fill = Color32::from_rgb(244, 245, 247);
            visuals.window_fill = Color32::from_rgb(250, 250, 252);
            visuals.extreme_bg_color = Color32::from_rgb(232, 234, 238);
            visuals.selection.bg_fill = Color32::from_rgb(53, 132, 228);
            visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
        }
        ctx.set_visuals(visuals);
    }

    fn create_new_document(&mut self, width: u32, height: u32, bg: BackgroundFill) {
        let fill = match bg {
            BackgroundFill::White => [255, 255, 255, 255],
            BackgroundFill::SecondaryColor => self.palette.secondary,
            BackgroundFill::Transparent => [0, 0, 0, 0],
        };
        let title = format!("Image {}", self.next_untitled_num);
        self.next_untitled_num += 1;
        let doc = Document::new(self.next_doc_id, title, width, height, fill);
        self.next_doc_id += 1;
        self.documents.push(doc);
        self.active_doc = self.documents.len() - 1;
        self.tool_runtime = ToolRuntime::default();
    }

    fn open_file_dialog(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = file_io::open_image_dialog() {
            match file_io::load_rgba_file(&path) {
                Ok((w, h, px)) => {
                    let title = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Image")
                        .to_string();
                    let doc = Document::from_rgba(self.next_doc_id, title, Some(path), w, h, px);
                    self.next_doc_id += 1;
                    self.documents.push(doc);
                    self.active_doc = self.documents.len() - 1;
                    self.tool_runtime = ToolRuntime::default();
                    self.status_message = Some("Opened image.".to_string());
                }
                Err(e) => self.status_message = Some(e),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            file_io::trigger_web_open_dialog(
                self.web_pending_files.clone(),
                self.egui_ctx.clone(),
                false,
            );
        }
    }

    fn save_active_document(&mut self, force_dialog: bool) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let existing_path = self.doc().file_path.clone();
            let target_path = if !force_dialog && existing_path.is_some() {
                existing_path
            } else {
                let default_name = format!("{}.png", self.doc().title.trim_end_matches('*'));
                file_io::save_image_dialog(&default_name)
            };
            if let Some(path) = target_path {
                let doc = self.doc_mut();
                if doc.composite_dirty {
                    doc.recomposite();
                }
                match file_io::save_rgba_file(&path, doc.width, doc.height, &doc.composite_rgba) {
                    Ok(()) => {
                        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                            doc.title = name.to_string();
                        }
                        doc.file_path = Some(path);
                        doc.dirty = false;
                        self.status_message = Some("Saved image.".to_string());
                    }
                    Err(e) => self.status_message = Some(e),
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = force_dialog;
            let doc = self.doc_mut();
            if doc.composite_dirty {
                doc.recomposite();
            }
            let base = doc.title.trim_end_matches('*');
            let filename = if base.ends_with(".png") || base.ends_with(".jpg") || base.ends_with(".jpeg") || base.ends_with(".bmp") {
                base.to_string()
            } else {
                format!("{base}.png")
            };
            let ext = filename.rsplit('.').next().unwrap_or("png").to_ascii_lowercase();
            let mime = match ext.as_str() {
                "jpg" | "jpeg" => "image/jpeg",
                "bmp" => "image/bmp",
                _ => "image/png",
            };
            match file_io::encode_rgba_bytes(&ext, doc.width, doc.height, &doc.composite_rgba)
                .and_then(|bytes| file_io::download_web_file(&filename, mime, &bytes))
            {
                Ok(()) => {
                    doc.dirty = false;
                    self.status_message = Some(format!("Downloaded {filename}."));
                }
                Err(e) => self.status_message = Some(e),
            }
        }
    }

    fn import_layer_dialog(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = file_io::open_image_dialog() {
            match file_io::load_rgba_file(&path) {
                Ok((w, h, px)) => {
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Imported Layer")
                        .to_string();
                    self.doc_mut().import_layer_rgba(name, w, h, px);
                    self.status_message = Some("Imported layer.".to_string());
                }
                Err(e) => self.status_message = Some(e),
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            file_io::trigger_web_open_dialog(
                self.web_pending_files.clone(),
                self.egui_ctx.clone(),
                true,
            );
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn poll_web_files(&mut self, ctx: &egui::Context) {
        let pending: Vec<file_io::PendingWebFile> =
            self.web_pending_files.borrow_mut().drain(..).collect();
        for item in pending {
            match file_io::decode_rgba_bytes(&item.bytes) {
                Ok((w, h, px)) => {
                    if item.as_layer {
                        self.doc_mut().import_layer_rgba(item.name, w, h, px);
                        self.status_message = Some("Imported layer.".to_string());
                    } else {
                        let doc = Document::from_rgba(self.next_doc_id, item.name, None, w, h, px);
                        self.next_doc_id += 1;
                        self.documents.push(doc);
                        self.active_doc = self.documents.len() - 1;
                        self.tool_runtime = ToolRuntime::default();
                        self.status_message = Some("Opened image.".to_string());
                    }
                }
                Err(e) => self.status_message = Some(e),
            }
        }

        // Also support drag-and-drop image files onto the browser canvas
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for file in dropped {
            if let Some(bytes) = file.bytes {
                let name = if file.name.is_empty() {
                    "Dropped Image".to_string()
                } else {
                    file.name
                };
                match file_io::decode_rgba_bytes(&bytes) {
                    Ok((w, h, px)) => {
                        let doc = Document::from_rgba(self.next_doc_id, name, None, w, h, px);
                        self.next_doc_id += 1;
                        self.documents.push(doc);
                        self.active_doc = self.documents.len() - 1;
                        self.tool_runtime = ToolRuntime::default();
                        self.status_message = Some("Opened dropped image.".to_string());
                    }
                    Err(e) => self.status_message = Some(e),
                }
            }
        }
    }

    fn close_document(&mut self, idx: usize) {
        if self.documents.len() <= 1 {
            // Reset single document to blank 800x600 instead of leaving zero documents
            let doc = Document::new(
                self.next_doc_id,
                format!("Image {}", self.next_untitled_num),
                DEFAULT_CANVAS_WIDTH,
                DEFAULT_CANVAS_HEIGHT,
                [255, 255, 255, 255],
            );
            self.next_doc_id += 1;
            self.next_untitled_num += 1;
            self.documents[0] = doc;
            self.active_doc = 0;
            return;
        }
        let removed_id = self.documents[idx].id;
        self.doc_textures.remove(&removed_id);
        self.documents.remove(idx);
        if self.active_doc >= self.documents.len() {
            self.active_doc = self.documents.len() - 1;
        }
    }

    fn trigger_effect(&mut self, spec: EffectSpec) {
        if spec.is_instant() {
            let primary = self.palette.primary;
            let secondary = self.palette.secondary;
            let doc = self.doc_mut();
            let src = doc.active_layer_ref().pixels.clone();
            let res = apply_effect(
                &src,
                doc.width,
                doc.height,
                doc.selection.as_ref(),
                primary,
                secondary,
                &spec,
            );
            doc.active_layer_mut().pixels = res;
            doc.commit_history(spec.name());
        } else {
            let orig = self.doc().active_layer_ref().pixels.clone();
            self.effect_dialog = Some(ActiveEffectDialog {
                spec,
                original_pixels: orig,
                needs_preview: true,
            });
        }
    }

    fn commit_active_text(&mut self, ctx: &egui::Context) {
        if let Some(txt) = self.tool_runtime.active_text.take() {
            if !txt.text.is_empty() {
                let size = self.tool_settings.text_size;
                let bold = self.tool_settings.text_bold;
                let doc = self.doc_mut();
                let w = doc.width;
                let h = doc.height;
                let sel = doc.selection.clone();
                rasterize_text_with_egui(
                    ctx,
                    &mut doc.active_layer_mut().pixels,
                    w,
                    h,
                    sel.as_ref(),
                    txt.pos,
                    &txt.text,
                    size,
                    bold,
                    txt.color,
                );
                doc.commit_history("Text");
            }
        }
    }

    fn select_tool_by_shortcut(&mut self, ch: char) {
        let matches: Vec<ToolKind> = ToolKind::ALL
            .iter()
            .copied()
            .filter(|t| t.shortcut() == ch)
            .collect();
        if matches.is_empty() {
            return;
        }
        if let Some(pos) = matches.iter().position(|&t| t == self.active_tool) {
            self.active_tool = matches[(pos + 1) % matches.len()];
        } else {
            self.active_tool = matches[0];
        }
    }

    fn handle_global_shortcuts(&mut self, ctx: &egui::Context) {
        // Don't hijack typing when a text field or modal dialog has keyboard focus
        if ctx.wants_keyboard_input() || self.tool_runtime.active_text.is_some() {
            if self.tool_runtime.active_text.is_some()
                && ctx.input(|i| i.key_pressed(Key::Escape))
            {
                self.commit_active_text(ctx);
            }
            return;
        }

        let input = ctx.input(|i| {
            (
                i.modifiers,
                i.key_pressed(Key::N),
                i.key_pressed(Key::O),
                i.key_pressed(Key::S),
                i.key_pressed(Key::Z),
                i.key_pressed(Key::Y),
                i.key_pressed(Key::C),
                i.key_pressed(Key::X),
                i.key_pressed(Key::V),
                i.key_pressed(Key::A),
                i.key_pressed(Key::D),
                i.key_pressed(Key::I),
                i.key_pressed(Key::B),
                i.key_pressed(Key::P),
                i.key_pressed(Key::E),
                i.key_pressed(Key::F),
                i.key_pressed(Key::G),
                i.key_pressed(Key::K),
                i.key_pressed(Key::T),
                i.key_pressed(Key::H),
                i.key_pressed(Key::L),
                i.key_pressed(Key::R),
                i.key_pressed(Key::M),
                i.key_pressed(Key::Delete),
                i.key_pressed(Key::OpenBracket),
                i.key_pressed(Key::CloseBracket),
                i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals),
                i.key_pressed(Key::Minus),
                i.key_pressed(Key::Num0),
                i.key_pressed(Key::F4),
            )
        });
        let (
            m, k_n, k_o, k_s, k_z, k_y, k_c, k_x, k_v, k_a, k_d, k_i, k_b, k_p, k_e, k_f, k_g,
            k_k, k_t, k_h, k_l, k_r, k_m, k_del, k_lb, k_rb, k_plus, k_minus, k_0, k_f4,
        ) = input;

        let ctrl = m.command || m.ctrl;
        if ctrl {
            if k_n && m.shift {
                self.doc_mut().add_layer();
            } else if k_n {
                self.new_image_dialog = Some(NewImageDialogState {
                    width: DEFAULT_CANVAS_WIDTH as i32,
                    height: DEFAULT_CANVAS_HEIGHT as i32,
                    background: BackgroundFill::White,
                });
            } else if k_o {
                self.open_file_dialog();
            } else if k_s {
                self.save_active_document(m.shift);
            } else if k_z && m.shift {
                self.doc_mut().redo();
            } else if k_z {
                self.doc_mut().undo();
            } else if k_y {
                self.doc_mut().redo();
            } else if k_c {
                self.clipboard = Some(self.doc_mut().copy_selection(m.shift));
            } else if k_x && m.shift {
                self.doc_mut().crop_to_selection();
            } else if k_x {
                self.clipboard = Some(self.doc_mut().copy_selection(false));
                self.doc_mut().erase_selection();
            } else if k_v {
                if let Some(clip) = self.clipboard.clone() {
                    self.doc_mut().paste_clipboard(&clip, m.shift);
                }
            } else if k_a {
                let w = self.doc().width;
                let h = self.doc().height;
                self.doc_mut().selection = Some(SelectionMask::full(w, h));
            } else if k_d && m.shift {
                self.doc_mut().duplicate_active_layer();
            } else if k_d {
                self.doc_mut().selection = None;
            } else if k_i && m.shift {
                self.trigger_effect(EffectSpec::InvertColors);
            } else if k_i {
                if let Some(sel) = &mut self.doc_mut().selection {
                    sel.invert();
                }
            } else if k_m {
                self.doc_mut().merge_layer_down();
            } else if k_b && m.shift {
                self.trigger_effect(EffectSpec::BrightnessContrast {
                    brightness: 0,
                    contrast: 0,
                });
            } else if k_b {
                let vp = self.last_canvas_viewport;
                self.doc_mut().zoom_to_window(vp.x, vp.y);
            } else if k_g && m.shift {
                self.trigger_effect(EffectSpec::BlackAndWhite);
            } else if k_f && m.shift {
                self.doc_mut().flatten();
            } else if k_e && m.shift {
                self.trigger_effect(EffectSpec::Sepia);
            } else if k_plus {
                self.doc_mut().zoom_in();
            } else if k_minus {
                self.doc_mut().zoom_out();
            } else if k_0 {
                let d = self.doc_mut();
                d.zoom = 1.0;
                d.pan = [0.0, 0.0];
            }
            return;
        }

        // Single-key shortcuts (no Ctrl/Alt)
        if m.alt {
            return;
        }
        if k_del {
            self.doc_mut().erase_selection();
        } else if k_lb {
            self.tool_settings.brush_width = (self.tool_settings.brush_width - 1).max(1);
        } else if k_rb {
            self.tool_settings.brush_width = (self.tool_settings.brush_width + 1).min(200);
        } else if k_x {
            self.palette.swap();
        } else if k_f4 {
            let l = self.doc().active_layer_ref();
            self.layer_props_dialog = Some(LayerPropsDialogState {
                name: l.name.clone(),
                visible: l.visible,
                opacity_pct: (l.opacity * 100.0).round() as i32,
                blend_mode: l.blend_mode,
            });
        } else if k_m {
            self.select_tool_by_shortcut('M');
        } else if k_z {
            self.select_tool_by_shortcut('Z');
        } else if k_h {
            self.select_tool_by_shortcut('H');
        } else if k_s {
            self.select_tool_by_shortcut('S');
        } else if k_b {
            self.select_tool_by_shortcut('B');
        } else if k_p {
            self.select_tool_by_shortcut('P');
        } else if k_e {
            self.select_tool_by_shortcut('E');
        } else if k_f {
            self.select_tool_by_shortcut('F');
        } else if k_g {
            self.select_tool_by_shortcut('G');
        } else if k_k {
            self.select_tool_by_shortcut('K');
        } else if k_t {
            self.select_tool_by_shortcut('T');
        } else if k_o {
            self.select_tool_by_shortcut('O');
        } else if k_l {
            self.select_tool_by_shortcut('L');
        } else if k_r {
            self.select_tool_by_shortcut('R');
        }
    }

    fn show_menu_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New...   Ctrl+N").clicked() {
                    self.new_image_dialog = Some(NewImageDialogState {
                        width: DEFAULT_CANVAS_WIDTH as i32,
                        height: DEFAULT_CANVAS_HEIGHT as i32,
                        background: BackgroundFill::White,
                    });
                    ui.close_menu();
                }
                if ui.button("Open...   Ctrl+O").clicked() {
                    self.open_file_dialog();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Save   Ctrl+S").clicked() {
                    self.save_active_document(false);
                    ui.close_menu();
                }
                if ui.button("Save As...   Ctrl+Shift+S").clicked() {
                    self.save_active_document(true);
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Close Tab").clicked() {
                    let idx = self.active_doc;
                    self.close_document(idx);
                    ui.close_menu();
                }
                if ui.button("Quit").clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Edit", |ui| {
                let can_undo = self.doc().history.can_undo();
                let can_redo = self.doc().history.can_redo();
                if ui
                    .add_enabled(can_undo, egui::Button::new("Undo   Ctrl+Z"))
                    .clicked()
                {
                    self.doc_mut().undo();
                    ui.close_menu();
                }
                if ui
                    .add_enabled(can_redo, egui::Button::new("Redo   Ctrl+Y"))
                    .clicked()
                {
                    self.doc_mut().redo();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Cut   Ctrl+X").clicked() {
                    self.clipboard = Some(self.doc_mut().copy_selection(false));
                    self.doc_mut().erase_selection();
                    ui.close_menu();
                }
                if ui.button("Copy   Ctrl+C").clicked() {
                    self.clipboard = Some(self.doc_mut().copy_selection(false));
                    ui.close_menu();
                }
                if ui.button("Copy Merged   Ctrl+Shift+C").clicked() {
                    self.clipboard = Some(self.doc_mut().copy_selection(true));
                    ui.close_menu();
                }
                let has_clip = self.clipboard.is_some();
                if ui
                    .add_enabled(has_clip, egui::Button::new("Paste   Ctrl+V"))
                    .clicked()
                {
                    if let Some(clip) = self.clipboard.clone() {
                        self.doc_mut().paste_clipboard(&clip, false);
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(
                        has_clip,
                        egui::Button::new("Paste into New Layer   Ctrl+Shift+V"),
                    )
                    .clicked()
                {
                    if let Some(clip) = self.clipboard.clone() {
                        self.doc_mut().paste_clipboard(&clip, true);
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(has_clip, egui::Button::new("Paste into New Image"))
                    .clicked()
                {
                    if let Some(clip) = self.clipboard.clone() {
                        let title = format!("Image {}", self.next_untitled_num);
                        self.next_untitled_num += 1;
                        let doc = Document::from_rgba(
                            self.next_doc_id,
                            title,
                            None,
                            clip.width,
                            clip.height,
                            clip.pixels,
                        );
                        self.next_doc_id += 1;
                        self.documents.push(doc);
                        self.active_doc = self.documents.len() - 1;
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Select All   Ctrl+A").clicked() {
                    let w = self.doc().width;
                    let h = self.doc().height;
                    self.doc_mut().selection = Some(SelectionMask::full(w, h));
                    ui.close_menu();
                }
                if ui.button("Deselect   Ctrl+D").clicked() {
                    self.doc_mut().selection = None;
                    ui.close_menu();
                }
                if ui.button("Invert Selection   Ctrl+I").clicked() {
                    if let Some(sel) = &mut self.doc_mut().selection {
                        sel.invert();
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Erase Selection   Del").clicked() {
                    self.doc_mut().erase_selection();
                    ui.close_menu();
                }
                if ui.button("Fill Selection (Primary)").clicked() {
                    let c = self.palette.primary;
                    self.doc_mut().fill_selection(c);
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                if ui.button("Zoom In   Ctrl++").clicked() {
                    self.doc_mut().zoom_in();
                    ui.close_menu();
                }
                if ui.button("Zoom Out   Ctrl+-").clicked() {
                    self.doc_mut().zoom_out();
                    ui.close_menu();
                }
                if ui.button("Best Fit   Ctrl+B").clicked() {
                    let vp = self.last_canvas_viewport;
                    self.doc_mut().zoom_to_window(vp.x, vp.y);
                    ui.close_menu();
                }
                if ui.button("Normal Size (100%)   Ctrl+0").clicked() {
                    let d = self.doc_mut();
                    d.zoom = 1.0;
                    d.pan = [0.0, 0.0];
                    ui.close_menu();
                }
                ui.separator();
                ui.checkbox(&mut self.show_grid, "Canvas Grid");
                if ui.button("Canvas Grid Settings...").clicked() {
                    self.grid_dialog = Some(GridDialogState {
                        show_grid: self.show_grid,
                        cell_width: self.grid_width,
                        cell_height: self.grid_height,
                    });
                    ui.close_menu();
                }
                ui.checkbox(&mut self.show_rulers, "Rulers");
                ui.separator();
                ui.checkbox(&mut self.show_toolbar, "Toolbar");
                ui.checkbox(&mut self.show_toolbox, "Tool Box");
                ui.checkbox(&mut self.show_tool_windows, "Layers & History");
                ui.checkbox(&mut self.show_image_tabs, "Image Tabs");
                ui.checkbox(&mut self.show_statusbar, "Status Bar");
                ui.separator();
                ui.menu_button("Color Scheme", |ui| {
                    if ui.selectable_label(!self.dark_mode, "Light").clicked() {
                        self.dark_mode = false;
                        ui.close_menu();
                    }
                    if ui.selectable_label(self.dark_mode, "Dark").clicked() {
                        self.dark_mode = true;
                        ui.close_menu();
                    }
                });
            });

            ui.menu_button("Image", |ui| {
                let has_sel = self.doc().selection.is_some();
                if ui
                    .add_enabled(has_sel, egui::Button::new("Crop to Selection   Ctrl+Shift+X"))
                    .clicked()
                {
                    self.doc_mut().crop_to_selection();
                    ui.close_menu();
                }
                if ui.button("Resize Image...").clicked() {
                    let w = self.doc().width;
                    let h = self.doc().height;
                    self.resize_image_dialog = Some(ResizeImageDialogState {
                        by_percentage: false,
                        percentage: 100,
                        width: w as i32,
                        height: h as i32,
                        orig_w: w,
                        orig_h: h,
                        maintain_aspect: true,
                        resampling: ResamplingMode::Bilinear,
                    });
                    ui.close_menu();
                }
                if ui.button("Resize Canvas...").clicked() {
                    let w = self.doc().width as i32;
                    let h = self.doc().height as i32;
                    self.resize_canvas_dialog = Some(ResizeCanvasDialogState {
                        width: w,
                        height: h,
                        anchor: Anchor::Center,
                    });
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Flip Horizontal").clicked() {
                    self.doc_mut().flip_image_horizontal();
                    ui.close_menu();
                }
                if ui.button("Flip Vertical").clicked() {
                    self.doc_mut().flip_image_vertical();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Rotate 90° Clockwise").clicked() {
                    self.doc_mut().rotate_90_cw();
                    ui.close_menu();
                }
                if ui.button("Rotate 90° Counter-Clockwise").clicked() {
                    self.doc_mut().rotate_90_ccw();
                    ui.close_menu();
                }
                if ui.button("Rotate 180°").clicked() {
                    self.doc_mut().rotate_180();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Flatten   Ctrl+Shift+F").clicked() {
                    self.doc_mut().flatten();
                    ui.close_menu();
                }
            });

            ui.menu_button("Layers", |ui| {
                if ui.button("Add New Layer   Ctrl+Shift+N").clicked() {
                    self.doc_mut().add_layer();
                    ui.close_menu();
                }
                let multi = self.doc().layers.len() > 1;
                if ui
                    .add_enabled(multi, egui::Button::new("Delete Layer"))
                    .clicked()
                {
                    self.doc_mut().delete_active_layer();
                    ui.close_menu();
                }
                if ui.button("Duplicate Layer   Ctrl+Shift+D").clicked() {
                    self.doc_mut().duplicate_active_layer();
                    ui.close_menu();
                }
                let can_merge = self.doc().active_layer > 0;
                if ui
                    .add_enabled(can_merge, egui::Button::new("Merge Layer Down   Ctrl+M"))
                    .clicked()
                {
                    self.doc_mut().merge_layer_down();
                    ui.close_menu();
                }
                if ui.button("Import from File...").clicked() {
                    self.import_layer_dialog();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Flip Horizontal").clicked() {
                    self.doc_mut().flip_active_layer_horizontal();
                    ui.close_menu();
                }
                if ui.button("Flip Vertical").clicked() {
                    self.doc_mut().flip_active_layer_vertical();
                    ui.close_menu();
                }
                ui.separator();
                if ui.button("Layer Properties...   F4").clicked() {
                    let l = self.doc().active_layer_ref();
                    self.layer_props_dialog = Some(LayerPropsDialogState {
                        name: l.name.clone(),
                        visible: l.visible,
                        opacity_pct: (l.opacity * 100.0).round() as i32,
                        blend_mode: l.blend_mode,
                    });
                    ui.close_menu();
                }
            });

            ui.menu_button("Adjustments", |ui| {
                if ui.button("Auto Level").clicked() {
                    self.trigger_effect(EffectSpec::AutoLevel);
                    ui.close_menu();
                }
                if ui.button("Black and White   Ctrl+Shift+G").clicked() {
                    self.trigger_effect(EffectSpec::BlackAndWhite);
                    ui.close_menu();
                }
                if ui.button("Brightness / Contrast...   Ctrl+Shift+B").clicked() {
                    self.trigger_effect(EffectSpec::BrightnessContrast {
                        brightness: 0,
                        contrast: 0,
                    });
                    ui.close_menu();
                }
                if ui.button("Hue / Saturation...").clicked() {
                    self.trigger_effect(EffectSpec::HueSaturation {
                        hue: 0,
                        saturation: 100,
                        lightness: 0,
                    });
                    ui.close_menu();
                }
                if ui.button("Invert Colors   Ctrl+Shift+I").clicked() {
                    self.trigger_effect(EffectSpec::InvertColors);
                    ui.close_menu();
                }
                if ui.button("Levels...").clicked() {
                    self.trigger_effect(EffectSpec::Levels {
                        in_black: 0,
                        in_white: 255,
                        gamma: 1.0,
                        out_black: 0,
                        out_white: 255,
                    });
                    ui.close_menu();
                }
                if ui.button("Posterize...").clicked() {
                    self.trigger_effect(EffectSpec::Posterize {
                        red: 16,
                        green: 16,
                        blue: 16,
                        linked: true,
                    });
                    ui.close_menu();
                }
                if ui.button("Sepia   Ctrl+Shift+E").clicked() {
                    self.trigger_effect(EffectSpec::Sepia);
                    ui.close_menu();
                }
            });

            ui.menu_button("Effects", |ui| {
                ui.menu_button("Artistic", |ui| {
                    if ui.button("Oil Painting...").clicked() {
                        self.trigger_effect(EffectSpec::OilPainting {
                            brush_size: 3,
                            coarseness: 50,
                        });
                        ui.close_menu();
                    }
                    if ui.button("Pencil Sketch...").clicked() {
                        self.trigger_effect(EffectSpec::PencilSketch { pencil_tip_size: 2 });
                        ui.close_menu();
                    }
                });
                ui.menu_button("Blurs", |ui| {
                    if ui.button("Gaussian Blur...").clicked() {
                        self.trigger_effect(EffectSpec::GaussianBlur { radius: 2 });
                        ui.close_menu();
                    }
                    if ui.button("Motion Blur...").clicked() {
                        self.trigger_effect(EffectSpec::MotionBlur {
                            angle: 25.0,
                            distance: 10,
                            centered: true,
                        });
                        ui.close_menu();
                    }
                    if ui.button("Radial Blur...").clicked() {
                        self.trigger_effect(EffectSpec::RadialBlur { angle: 2.0 });
                        ui.close_menu();
                    }
                    if ui.button("Zoom Blur...").clicked() {
                        self.trigger_effect(EffectSpec::ZoomBlur { amount: 10 });
                        ui.close_menu();
                    }
                });
                ui.menu_button("Distort", |ui| {
                    if ui.button("Bulge...").clicked() {
                        self.trigger_effect(EffectSpec::Bulge { amount: 45 });
                        ui.close_menu();
                    }
                    if ui.button("Frosted Glass...").clicked() {
                        self.trigger_effect(EffectSpec::FrostedGlass { radius: 1 });
                        ui.close_menu();
                    }
                    if ui.button("Pixelate...").clicked() {
                        self.trigger_effect(EffectSpec::Pixelate { cell_size: 2 });
                        ui.close_menu();
                    }
                    if ui.button("Twist...").clicked() {
                        self.trigger_effect(EffectSpec::Twist {
                            amount: 30,
                            size: 100,
                        });
                        ui.close_menu();
                    }
                });
                ui.menu_button("Noise", |ui| {
                    if ui.button("Add Noise...").clicked() {
                        self.trigger_effect(EffectSpec::AddNoise {
                            intensity: 64,
                            saturation: 100,
                            coverage: 100.0,
                        });
                        ui.close_menu();
                    }
                });
                ui.menu_button("Photo", |ui| {
                    if ui.button("Glow...").clicked() {
                        self.trigger_effect(EffectSpec::Glow {
                            radius: 6,
                            brightness: 10,
                            contrast: 10,
                        });
                        ui.close_menu();
                    }
                    if ui.button("Sharpen...").clicked() {
                        self.trigger_effect(EffectSpec::Sharpen { amount: 2 });
                        ui.close_menu();
                    }
                    if ui.button("Vignette...").clicked() {
                        self.trigger_effect(EffectSpec::Vignette {
                            radius: 50,
                            density: 1.0,
                        });
                        ui.close_menu();
                    }
                });
                ui.menu_button("Render", |ui| {
                    if ui.button("Clouds...").clicked() {
                        self.trigger_effect(EffectSpec::Clouds {
                            scale: 250,
                            power: 50,
                        });
                        ui.close_menu();
                    }
                    if ui.button("Julia Fractal...").clicked() {
                        self.trigger_effect(EffectSpec::JuliaFractal { zoom: 1.0 });
                        ui.close_menu();
                    }
                    if ui.button("Mandelbrot Fractal...").clicked() {
                        self.trigger_effect(EffectSpec::MandelbrotFractal { zoom: 10.0 });
                        ui.close_menu();
                    }
                });
                ui.menu_button("Stylize", |ui| {
                    if ui.button("Edge Detect...").clicked() {
                        self.trigger_effect(EffectSpec::EdgeDetect { angle: 45.0 });
                        ui.close_menu();
                    }
                    if ui.button("Emboss...").clicked() {
                        self.trigger_effect(EffectSpec::Emboss { angle: 0.0 });
                        ui.close_menu();
                    }
                    if ui.button("Relief...").clicked() {
                        self.trigger_effect(EffectSpec::Relief { angle: 45.0 });
                        ui.close_menu();
                    }
                });
            });

            ui.menu_button("Window", |ui| {
                for i in 0..self.documents.len() {
                    let title = self.documents[i].title.clone();
                    if ui.selectable_label(i == self.active_doc, title).clicked() {
                        self.active_doc = i;
                        ui.close_menu();
                    }
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About Canvas Studio").clicked() {
                    self.show_about = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn show_main_toolbar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if ui.button("New").on_hover_text("New Image (Ctrl+N)").clicked() {
                self.new_image_dialog = Some(NewImageDialogState {
                    width: DEFAULT_CANVAS_WIDTH as i32,
                    height: DEFAULT_CANVAS_HEIGHT as i32,
                    background: BackgroundFill::White,
                });
            }
            if ui.button("Open").on_hover_text("Open Image (Ctrl+O)").clicked() {
                self.open_file_dialog();
            }
            if ui.button("Save").on_hover_text("Save Image (Ctrl+S)").clicked() {
                self.save_active_document(false);
            }
            ui.separator();
            if ui.button("Cut").on_hover_text("Cut (Ctrl+X)").clicked() {
                self.clipboard = Some(self.doc_mut().copy_selection(false));
                self.doc_mut().erase_selection();
            }
            if ui.button("Copy").on_hover_text("Copy (Ctrl+C)").clicked() {
                self.clipboard = Some(self.doc_mut().copy_selection(false));
            }
            if ui
                .add_enabled(self.clipboard.is_some(), egui::Button::new("Paste"))
                .on_hover_text("Paste (Ctrl+V)")
                .clicked()
            {
                if let Some(clip) = self.clipboard.clone() {
                    self.doc_mut().paste_clipboard(&clip, false);
                }
            }
            ui.separator();
            if ui
                .add_enabled(self.doc().history.can_undo(), egui::Button::new("Undo"))
                .on_hover_text("Undo (Ctrl+Z)")
                .clicked()
            {
                self.doc_mut().undo();
            }
            if ui
                .add_enabled(self.doc().history.can_redo(), egui::Button::new("Redo"))
                .on_hover_text("Redo (Ctrl+Y)")
                .clicked()
            {
                self.doc_mut().redo();
            }
            ui.separator();
            if ui
                .add_enabled(self.doc().selection.is_some(), egui::Button::new("Crop"))
                .on_hover_text("Crop to Selection (Ctrl+Shift+X)")
                .clicked()
            {
                self.doc_mut().crop_to_selection();
            }
            if ui
                .add_enabled(self.doc().selection.is_some(), egui::Button::new("Deselect"))
                .on_hover_text("Deselect (Ctrl+D)")
                .clicked()
            {
                self.doc_mut().selection = None;
            }
            ui.separator();
            if ui.button("-").on_hover_text("Zoom Out").clicked() {
                self.doc_mut().zoom_out();
            }
            let zoom_pct = format!("{}%", (self.doc().zoom * 100.0).round() as i32);
            egui::ComboBox::from_id_salt("toolbar_zoom_combo")
                .width(74.0)
                .selected_text(zoom_pct)
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, "Window").clicked() {
                        let vp = self.last_canvas_viewport;
                        self.doc_mut().zoom_to_window(vp.x, vp.y);
                    }
                    for &z in &ZOOM_LEVELS {
                        let label = format!("{}%", (z * 100.0).round() as i32);
                        let selected = (self.doc().zoom - z).abs() < 0.005;
                        if ui.selectable_label(selected, label).clicked() {
                            self.doc_mut().zoom = z;
                        }
                    }
                });
            if ui.button("+").on_hover_text("Zoom In").clicked() {
                self.doc_mut().zoom_in();
            }
            if ui.button("Fit").on_hover_text("Best Fit (Ctrl+B)").clicked() {
                let vp = self.last_canvas_viewport;
                self.doc_mut().zoom_to_window(vp.x, vp.y);
            }
        });
    }

    fn show_tool_options_bar(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let tool = self.active_tool;
            ui.label(
                RichText::new(format!("Tool: {}", tool.name()))
                    .strong()
                    .size(13.0),
            );
            ui.separator();

            if tool.uses_brush_width() {
                ui.label("Brush width:");
                if ui.small_button("-").clicked() {
                    self.tool_settings.brush_width = (self.tool_settings.brush_width - 1).max(1);
                }
                ui.add(
                    egui::DragValue::new(&mut self.tool_settings.brush_width)
                        .range(1..=200)
                        .suffix(" px"),
                );
                if ui.small_button("+").clicked() {
                    self.tool_settings.brush_width = (self.tool_settings.brush_width + 1).min(200);
                }
                ui.separator();
            }

            if matches!(
                tool,
                ToolKind::Paintbrush
                    | ToolKind::LineCurve
                    | ToolKind::Rectangle
                    | ToolKind::RoundedRectangle
                    | ToolKind::Ellipse
                    | ToolKind::FreeformShape
            ) {
                ui.checkbox(&mut self.tool_settings.antialiasing, "Antialiasing");
                egui::ComboBox::from_id_salt("tool_blend_mode")
                    .width(96.0)
                    .selected_text(if self.tool_settings.overwrite_blend {
                        "Overwrite"
                    } else {
                        "Normal"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.tool_settings.overwrite_blend, false, "Normal");
                        ui.selectable_value(
                            &mut self.tool_settings.overwrite_blend,
                            true,
                            "Overwrite",
                        );
                    });
            }

            if tool.uses_fill_style() {
                ui.separator();
                ui.label("Fill Style:");
                egui::ComboBox::from_id_salt("shape_fill_style")
                    .width(108.0)
                    .selected_text(self.tool_settings.fill_style.label())
                    .show_ui(ui, |ui| {
                        for fs in ShapeFillStyle::ALL {
                            ui.selectable_value(&mut self.tool_settings.fill_style, fs, fs.label());
                        }
                    });
            }

            if tool == ToolKind::RoundedRectangle {
                ui.label("Radius:");
                ui.add(
                    egui::DragValue::new(&mut self.tool_settings.corner_radius)
                        .range(0..=120)
                        .suffix(" px"),
                );
            }

            if tool.uses_tolerance() {
                ui.label("Tolerance:");
                ui.add(
                    egui::Slider::new(&mut self.tool_settings.tolerance, 0..=100)
                        .suffix("%"),
                );
                if matches!(tool, ToolKind::PaintBucket | ToolKind::MagicWand) {
                    ui.checkbox(&mut self.tool_settings.flood_contiguous, "Contiguous");
                }
            }

            if tool.uses_selection_mode() {
                ui.separator();
                ui.label("Selection Mode:");
                egui::ComboBox::from_id_salt("sel_combine_mode")
                    .width(98.0)
                    .selected_text(self.tool_settings.selection_mode.label())
                    .show_ui(ui, |ui| {
                        for m in SelectionCombineMode::ALL {
                            ui.selectable_value(
                                &mut self.tool_settings.selection_mode,
                                m,
                                m.label(),
                            );
                        }
                    });
            }

            if tool == ToolKind::Gradient {
                ui.label("Gradient:");
                egui::ComboBox::from_id_salt("grad_kind")
                    .width(120.0)
                    .selected_text(self.tool_settings.gradient_kind.label())
                    .show_ui(ui, |ui| {
                        for gk in GradientKind::ALL {
                            ui.selectable_value(
                                &mut self.tool_settings.gradient_kind,
                                gk,
                                gk.label(),
                            );
                        }
                    });
                ui.selectable_value(
                    &mut self.tool_settings.gradient_mode,
                    GradientColorMode::Color,
                    "Color Mode",
                );
                ui.selectable_value(
                    &mut self.tool_settings.gradient_mode,
                    GradientColorMode::Transparency,
                    "Transparency",
                );
            }

            if tool == ToolKind::ColorPicker {
                ui.label("Sampling:");
                egui::ComboBox::from_id_salt("picker_sample")
                    .width(64.0)
                    .selected_text(format!(
                        "{}x{}",
                        self.tool_settings.picker_sample_size,
                        self.tool_settings.picker_sample_size
                    ))
                    .show_ui(ui, |ui| {
                        for sz in [1, 3, 5, 7, 9] {
                            ui.selectable_value(
                                &mut self.tool_settings.picker_sample_size,
                                sz,
                                format!("{sz}x{sz}"),
                            );
                        }
                    });
                ui.checkbox(
                    &mut self.tool_settings.picker_sample_layer_only,
                    "Current Layer Only",
                );
            }

            if tool == ToolKind::Text {
                ui.label("Size:");
                ui.add(
                    egui::DragValue::new(&mut self.tool_settings.text_size)
                        .range(8.0..=160.0)
                        .suffix(" pt"),
                );
                ui.checkbox(&mut self.tool_settings.text_bold, "Bold");
                if self.tool_runtime.active_text.is_some() {
                    if ui.button("Commit Text").clicked() {
                        self.commit_active_text(ctx);
                    }
                    if ui.button("Cancel").clicked() {
                        self.tool_runtime.active_text = None;
                    }
                }
            }

            if tool == ToolKind::CloneStamp {
                let src_label = match self.tool_runtime.clone_source {
                    Some([x, y]) => format!("Source: ({x}, {y})  [Ctrl+Click to change]"),
                    None => "Source: unset  [Ctrl+Click on canvas to set]".to_string(),
                };
                ui.label(src_label);
            }
        });
    }

    fn show_toolbox_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new("Tools").strong().size(12.0));
        });
        ui.add_space(2.0);

        let fg = if self.dark_mode {
            Color32::from_gray(232)
        } else {
            Color32::from_gray(38)
        };
        let accent = Color32::from_rgb(53, 132, 228);

        egui::Grid::new("toolbox_grid")
            .spacing([4.0, 4.0])
            .min_col_width(30.0)
            .show(ui, |ui| {
                for (i, &tool) in ToolKind::ALL.iter().enumerate() {
                    let selected = self.active_tool == tool;
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(30.0, 30.0), Sense::click());
                    let bg = if selected {
                        accent.gamma_multiply(0.22)
                    } else if resp.hovered() {
                        if self.dark_mode {
                            Color32::from_gray(58)
                        } else {
                            Color32::from_gray(225)
                        }
                    } else {
                        Color32::TRANSPARENT
                    };
                    let border = if selected {
                        Stroke::new(1.5_f32, accent)
                    } else {
                        Stroke::new(
                            1.0_f32,
                            if self.dark_mode {
                                Color32::from_gray(55)
                            } else {
                                Color32::from_gray(214)
                            },
                        )
                    };
                    ui.painter().rect(
                        rect,
                        CornerRadius::same(4),
                        bg,
                        border,
                        StrokeKind::Inside,
                    );
                    icons::paint_tool_icon(ui.painter(), rect, tool, fg, accent);

                    if resp
                        .on_hover_text(format!("{} ({})", tool.name(), tool.shortcut()))
                        .clicked()
                    {
                        if self.active_tool == ToolKind::Text && tool != ToolKind::Text {
                            self.commit_active_text(ctx);
                        }
                        self.active_tool = tool;
                    }
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
    }

    fn show_right_dock(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let total_h = ui.available_height();
        let layers_h = (total_h * 0.56).max(200.0);

        // --- LAYERS PANEL ---
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), layers_h),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Layers").strong().size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{}", self.doc().layers.len()))
                                .weak()
                                .size(11.0),
                        );
                    });
                });
                ui.separator();

                // Active layer quick Blend Mode & Opacity strip
                let active_idx = self.doc().active_layer;
                let mut cur_blend = self.doc().layers[active_idx].blend_mode;
                let mut cur_op = (self.doc().layers[active_idx].opacity * 100.0).round() as i32;
                let mut changed_props = false;
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Blend:").size(11.5));
                    egui::ComboBox::from_id_salt("layer_blend_quick")
                        .width(128.0)
                        .selected_text(cur_blend.label())
                        .show_ui(ui, |ui| {
                            for bm in BlendMode::ALL {
                                if ui.selectable_value(&mut cur_blend, bm, bm.label()).changed() {
                                    changed_props = true;
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Opacity:").size(11.5));
                    if ui
                        .add(egui::Slider::new(&mut cur_op, 0..=100).suffix("%"))
                        .changed()
                    {
                        changed_props = true;
                    }
                });
                if changed_props {
                    let doc = self.doc_mut();
                    doc.layers[active_idx].blend_mode = cur_blend;
                    doc.layers[active_idx].opacity = cur_op as f32 / 100.0;
                    doc.mark_dirty();
                }
                ui.add_space(3.0);

                // Layer list (top-most layer displayed at top)
                let list_h = (ui.available_height() - 34.0).max(80.0);
                let doc_id = self.doc().id;
                let doc_ver = self.doc().version;
                let doc_w = self.doc().width;
                let doc_h = self.doc().height;
                let layer_count = self.doc().layers.len();

                // Refresh mini thumbnails if needed
                for li in 0..layer_count {
                    let needs = match self.thumb_textures.get(&(doc_id, li)) {
                        Some((v, _)) => *v != doc_ver,
                        None => true,
                    };
                    if needs {
                        let (tw, th, rgba) =
                            self.doc().layers[li].make_thumbnail(doc_w, doc_h, 34);
                        let img = ColorImage::from_rgba_unmultiplied(
                            [tw as usize, th as usize],
                            &rgba,
                        );
                        let handle = ctx.load_texture(
                            format!("thumb_{doc_id}_{li}"),
                            img,
                            TextureOptions::LINEAR,
                        );
                        self.thumb_textures.insert((doc_id, li), (doc_ver, handle));
                    }
                }

                let mut select_layer = None;
                let mut toggle_vis = None;

                egui::ScrollArea::vertical()
                    .id_salt("layers_scroll")
                    .max_height(list_h)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for li in (0..layer_count).rev() {
                            let is_active = li == self.doc().active_layer;
                            let layer = &self.doc().layers[li];
                            let bg = if is_active {
                                Color32::from_rgb(53, 132, 228).gamma_multiply(0.22)
                            } else {
                                Color32::TRANSPARENT
                            };
                            let frame = egui::Frame::new()
                                .fill(bg)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(Margin::symmetric(4, 3));
                            let resp = frame.show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let mut vis = layer.visible;
                                    if ui.checkbox(&mut vis, "").changed() {
                                        toggle_vis = Some(li);
                                    }
                                    if let Some((_, tex)) = self.thumb_textures.get(&(doc_id, li))
                                    {
                                        let sz = tex.size_vec2();
                                        let scale = (28.0 / sz.x.max(sz.y).max(1.0)).min(1.0);
                                        let disp = sz * scale;
                                        let (tr, _) = ui.allocate_exact_size(
                                            Vec2::new(30.0, 26.0),
                                            Sense::hover(),
                                        );
                                        let img_r = Rect::from_center_size(tr.center(), disp);
                                        ui.painter().image(
                                            tex.id(),
                                            img_r,
                                            Rect::from_min_max(
                                                Pos2::ZERO,
                                                Pos2::new(1.0, 1.0),
                                            ),
                                            Color32::WHITE,
                                        );
                                        ui.painter().rect_stroke(
                                            img_r,
                                            0.0,
                                            Stroke::new(1.0_f32, Color32::from_gray(130)),
                                            StrokeKind::Outside,
                                        );
                                    }
                                    ui.vertical(|ui| {
                                        let name_lbl = ui.add(
                                            egui::Label::new(
                                                RichText::new(&layer.name).strong(),
                                            )
                                            .sense(Sense::click()),
                                        );
                                        if name_lbl.clicked() {
                                            select_layer = Some(li);
                                        }
                                        ui.label(
                                            RichText::new(format!(
                                                "{} · {}%",
                                                layer.blend_mode.label(),
                                                (layer.opacity * 100.0).round() as i32
                                            ))
                                            .weak()
                                            .size(10.5),
                                        );
                                    });
                                });
                            });
                            if resp.response.interact(Sense::click()).clicked() {
                                select_layer = Some(li);
                            }
                        }
                    });

                if let Some(li) = select_layer {
                    self.doc_mut().active_layer = li;
                }
                if let Some(li) = toggle_vis {
                    let doc = self.doc_mut();
                    doc.layers[li].visible = !doc.layers[li].visible;
                    doc.mark_dirty();
                }

                ui.separator();
                // Layer action toolbar
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    if ui.small_button("+").on_hover_text("Add New Layer").clicked() {
                        self.doc_mut().add_layer();
                    }
                    if ui
                        .add_enabled(layer_count > 1, egui::Button::new("-").small())
                        .on_hover_text("Delete Layer")
                        .clicked()
                    {
                        self.doc_mut().delete_active_layer();
                    }
                    if ui
                        .small_button("Dup")
                        .on_hover_text("Duplicate Layer")
                        .clicked()
                    {
                        self.doc_mut().duplicate_active_layer();
                    }
                    if ui
                        .add_enabled(
                            self.doc().active_layer > 0,
                            egui::Button::new("Mrg").small(),
                        )
                        .on_hover_text("Merge Layer Down")
                        .clicked()
                    {
                        self.doc_mut().merge_layer_down();
                    }
                    if ui
                        .add_enabled(
                            self.doc().active_layer + 1 < layer_count,
                            egui::Button::new("Up").small(),
                        )
                        .on_hover_text("Move Layer Up")
                        .clicked()
                    {
                        self.doc_mut().move_layer_up();
                    }
                    if ui
                        .add_enabled(
                            self.doc().active_layer > 0,
                            egui::Button::new("Dn").small(),
                        )
                        .on_hover_text("Move Layer Down")
                        .clicked()
                    {
                        self.doc_mut().move_layer_down();
                    }
                    if ui
                        .small_button("...")
                        .on_hover_text("Layer Properties (F4)")
                        .clicked()
                    {
                        let l = self.doc().active_layer_ref();
                        self.layer_props_dialog = Some(LayerPropsDialogState {
                            name: l.name.clone(),
                            visible: l.visible,
                            opacity_pct: (l.opacity * 100.0).round() as i32,
                            blend_mode: l.blend_mode,
                        });
                    }
                });
            },
        );

        ui.separator();

        // --- HISTORY PANEL ---
        ui.horizontal(|ui| {
            ui.label(RichText::new("History").strong().size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        self.doc().history.can_redo(),
                        egui::Button::new("Redo").small(),
                    )
                    .clicked()
                {
                    self.doc_mut().redo();
                }
                if ui
                    .add_enabled(
                        self.doc().history.can_undo(),
                        egui::Button::new("Undo").small(),
                    )
                    .clicked()
                {
                    self.doc_mut().undo();
                }
            });
        });
        ui.separator();

        let mut jump_target = None;
        let cur_hist = self.doc().history.current;
        egui::ScrollArea::vertical()
            .id_salt("history_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (i, item) in self.doc().history.items.iter().enumerate() {
                    let is_cur = i == cur_hist;
                    let is_future = i > cur_hist;
                    let mut text = RichText::new(format!(
                        "{} {}",
                        if is_cur { "•" } else { " " },
                        item.label
                    ))
                    .size(12.0);
                    if is_cur {
                        text = text.strong().color(Color32::WHITE);
                    } else if is_future {
                        text = text.weak().italics();
                    }
                    if ui.selectable_label(is_cur, text).clicked() {
                        jump_target = Some(i);
                    }
                }
            });
        if let Some(idx) = jump_target {
            self.doc_mut().jump_history(idx);
        }
    }

    fn show_status_and_palette_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            // --- Overlapping Primary & Secondary Color Swatches ---
            let (swatch_area, _) =
                ui.allocate_exact_size(Vec2::new(42.0, 34.0), Sense::hover());
            let sec_rect = Rect::from_min_size(
                swatch_area.min + Vec2::new(16.0, 12.0),
                Vec2::new(22.0, 20.0),
            );
            let pri_rect = Rect::from_min_size(
                swatch_area.min + Vec2::new(2.0, 2.0),
                Vec2::new(22.0, 20.0),
            );
            let sec_resp = ui.interact(sec_rect, ui.id().with("sec_swatch"), Sense::click());
            let pri_resp = ui.interact(pri_rect, ui.id().with("pri_swatch"), Sense::click());

            ui.painter().rect(
                sec_rect,
                CornerRadius::same(2),
                rgba_to_color32(self.palette.secondary),
                Stroke::new(1.0_f32, Color32::from_gray(90)),
                StrokeKind::Inside,
            );
            ui.painter().rect(
                pri_rect,
                CornerRadius::same(2),
                rgba_to_color32(self.palette.primary),
                Stroke::new(1.5_f32, Color32::WHITE),
                StrokeKind::Inside,
            );
            ui.painter().rect_stroke(
                pri_rect,
                CornerRadius::same(2),
                Stroke::new(1.0_f32, Color32::from_gray(50)),
                StrokeKind::Outside,
            );

            if pri_resp
                .on_hover_text("Primary Color (click to edit)")
                .clicked()
            {
                self.color_picker_target = Some(ColorPickerTarget::Primary);
            }
            if sec_resp
                .on_hover_text("Secondary Color (click to edit)")
                .clicked()
            {
                self.color_picker_target = Some(ColorPickerTarget::Secondary);
            }

            // Swap & Reset mini buttons (crisp vector icons, zero font dependency)
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                let btn_fg = if self.dark_mode {
                    Color32::from_gray(225)
                } else {
                    Color32::from_gray(55)
                };
                let btn_bg = if self.dark_mode {
                    Color32::from_gray(55)
                } else {
                    Color32::from_gray(232)
                };
                let (sr, sresp) = ui.allocate_exact_size(Vec2::new(18.0, 15.0), Sense::click());
                ui.painter().rect(
                    sr,
                    CornerRadius::same(2),
                    if sresp.hovered() { btn_bg } else { Color32::TRANSPARENT },
                    Stroke::new(0.8_f32, Color32::from_gray(165)),
                    StrokeKind::Inside,
                );
                let sc = sr.center();
                ui.painter().line_segment(
                    [Pos2::new(sc.x - 4.5, sc.y - 2.2), Pos2::new(sc.x + 4.5, sc.y - 2.2)],
                    Stroke::new(1.2_f32, btn_fg),
                );
                ui.painter().line_segment(
                    [Pos2::new(sc.x + 2.0, sc.y - 4.5), Pos2::new(sc.x + 4.5, sc.y - 2.2)],
                    Stroke::new(1.2_f32, btn_fg),
                );
                ui.painter().line_segment(
                    [Pos2::new(sc.x + 4.5, sc.y + 2.2), Pos2::new(sc.x - 4.5, sc.y + 2.2)],
                    Stroke::new(1.2_f32, btn_fg),
                );
                ui.painter().line_segment(
                    [Pos2::new(sc.x - 2.0, sc.y + 4.5), Pos2::new(sc.x - 4.5, sc.y + 2.2)],
                    Stroke::new(1.2_f32, btn_fg),
                );
                if sresp.on_hover_text("Swap Colors (X)").clicked() {
                    self.palette.swap();
                }

                let (rr, rresp) = ui.allocate_exact_size(Vec2::new(18.0, 15.0), Sense::click());
                ui.painter().rect(
                    rr,
                    CornerRadius::same(2),
                    if rresp.hovered() { btn_bg } else { Color32::TRANSPARENT },
                    Stroke::new(0.8_f32, Color32::from_gray(165)),
                    StrokeKind::Inside,
                );
                let rc = rr.center();
                let rw_rect = Rect::from_min_size(Pos2::new(rc.x - 1.5, rc.y - 1.0), Vec2::new(6.5, 6.0));
                let rb_rect = Rect::from_min_size(Pos2::new(rc.x - 5.0, rc.y - 4.5), Vec2::new(6.5, 6.0));
                ui.painter().rect(
                    rw_rect,
                    CornerRadius::ZERO,
                    Color32::WHITE,
                    Stroke::new(0.8_f32, Color32::from_gray(90)),
                    StrokeKind::Inside,
                );
                ui.painter().rect(
                    rb_rect,
                    CornerRadius::ZERO,
                    Color32::BLACK,
                    Stroke::new(0.8_f32, Color32::from_gray(90)),
                    StrokeKind::Inside,
                );
                if rresp.on_hover_text("Reset Colors to Black & White").clicked() {
                    self.palette.reset();
                }
            });

            // --- 2-Row 34-Color Exact Default Palette (17 columns x 2 rows) ---
            let cols = 17usize;
            let cell = 15.0;
            let gap = 1.5;
            let pal_w = cols as f32 * (cell + gap);
            let pal_h = 2.0 * cell + gap;
            let (pal_rect, _) = ui.allocate_exact_size(Vec2::new(pal_w, pal_h), Sense::hover());
            for col in 0..cols {
                for row in 0..2 {
                    let idx = col * 2 + row;
                    if idx >= self.palette.colors.len() {
                        continue;
                    }
                    let c = self.palette.colors[idx];
                    let r = Rect::from_min_size(
                        Pos2::new(
                            pal_rect.left() + col as f32 * (cell + gap),
                            pal_rect.top() + row as f32 * (cell + gap),
                        ),
                        Vec2::new(cell, cell),
                    );
                    let resp = ui.interact(r, ui.id().with(("pal", idx)), Sense::click());
                    ui.painter().rect(
                        r,
                        CornerRadius::same(1),
                        rgba_to_color32(c),
                        Stroke::new(
                            if resp.hovered() { 1.5 } else { 0.8_f32 },
                            if resp.hovered() {
                                Color32::WHITE
                            } else {
                                Color32::from_gray(110)
                            },
                        ),
                        StrokeKind::Inside,
                    );
                    if resp.clicked() {
                        self.palette.set_primary(c);
                    } else if resp.secondary_clicked() {
                        self.palette.set_secondary(c);
                    }
                }
            }

            ui.separator();

            // --- Recent Colors (up to 10, 5x2 grid) ---
            let rec_w = 5.0 * (cell + gap);
            let (rec_rect, _) = ui.allocate_exact_size(Vec2::new(rec_w, pal_h), Sense::hover());
            for i in 0..10 {
                let col = i % 5;
                let row = i / 5;
                let r = Rect::from_min_size(
                    Pos2::new(
                        rec_rect.left() + col as f32 * (cell + gap),
                        rec_rect.top() + row as f32 * (cell + gap),
                    ),
                    Vec2::new(cell, cell),
                );
                if let Some(&c) = self.palette.recent.get(i) {
                    let resp = ui.interact(r, ui.id().with(("rec", i)), Sense::click());
                    ui.painter().rect(
                        r,
                        CornerRadius::same(1),
                        rgba_to_color32(c),
                        Stroke::new(0.8_f32, Color32::from_gray(110)),
                        StrokeKind::Inside,
                    );
                    if resp.clicked() {
                        self.palette.primary = c;
                    } else if resp.secondary_clicked() {
                        self.palette.secondary = c;
                    }
                } else {
                    ui.painter().rect_stroke(
                        r,
                        CornerRadius::same(1),
                        Stroke::new(0.6_f32, Color32::from_gray(150)),
                        StrokeKind::Inside,
                    );
                }
            }

            ui.separator();

            // --- Hint + Selection + Document Size + Cursor Coordinates ---
            let hint = self
                .status_message
                .as_deref()
                .unwrap_or_else(|| self.active_tool.hint());
            ui.label(RichText::new(hint).size(11.5).weak());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let zoom_str = format!("{}%", (self.doc().zoom * 100.0).round() as i32);
                ui.label(RichText::new(zoom_str).strong().size(11.5));
                ui.separator();
                let pos_str = match self.cursor_canvas_pos {
                    Some([x, y]) => format!("{x}, {y}"),
                    None => "—, —".to_string(),
                };
                ui.label(RichText::new(pos_str).monospace().size(11.5));
                ui.separator();
                let size_str = format!("{} × {}", self.doc().width, self.doc().height);
                ui.label(RichText::new(size_str).monospace().size(11.5));
                if let Some((_, _, sw, sh)) =
                    self.doc().selection.as_ref().and_then(|s| s.bounds())
                {
                    ui.separator();
                    ui.label(
                        RichText::new(format!("Sel: {sw} × {sh}"))
                            .monospace()
                            .size(11.5),
                    );
                }
            });
        });
    }

    fn show_document_tabs(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            let mut switch_to = None;
            let mut close_idx = None;
            for i in 0..self.documents.len() {
                let d = &self.documents[i];
                let selected = i == self.active_doc;
                let label = if d.dirty {
                    format!("{}*", d.title)
                } else {
                    d.title.clone()
                };
                let bg = if selected {
                    if self.dark_mode {
                        Color32::from_rgb(56, 60, 68)
                    } else {
                        Color32::WHITE
                    }
                } else if self.dark_mode {
                    Color32::from_rgb(32, 34, 38)
                } else {
                    Color32::from_rgb(230, 232, 236)
                };
                egui::Frame::new()
                    .fill(bg)
                    .corner_radius(CornerRadius {
                        nw: 4,
                        ne: 4,
                        sw: 0,
                        se: 0,
                    })
                    .inner_margin(Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Label::new(
                                        if selected {
                                            RichText::new(label).strong().size(12.0)
                                        } else {
                                            RichText::new(label).size(12.0)
                                        },
                                    )
                                    .sense(Sense::click()),
                                )
                                .clicked()
                            {
                                switch_to = Some(i);
                            }
                            if ui.small_button("×").clicked() {
                                close_idx = Some(i);
                            }
                        });
                    });
            }
            if ui
                .small_button("+")
                .on_hover_text("New Document")
                .clicked()
            {
                self.create_new_document(
                    DEFAULT_CANVAS_WIDTH,
                    DEFAULT_CANVAS_HEIGHT,
                    BackgroundFill::White,
                );
            }
            if let Some(i) = switch_to {
                self.active_doc = i;
                self.tool_runtime = ToolRuntime::default();
            }
            if let Some(i) = close_idx {
                self.close_document(i);
            }
        });
    }

    fn show_canvas_viewport(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        // Ensure active doc composite is up to date
        if self.doc().composite_dirty {
            self.doc_mut().recomposite();
        }

        let doc_id = self.doc().id;
        let doc_ver = self.doc().version;
        let doc_w = self.doc().width;
        let doc_h = self.doc().height;
        let zoom = self.doc().zoom;
        let use_nearest = zoom >= 2.0;

        let needs_upload = match self.doc_textures.get(&doc_id) {
            Some(c) => c.version != doc_ver || c.nearest != use_nearest,
            None => true,
        };
        if needs_upload {
            let img = ColorImage::from_rgba_unmultiplied(
                [doc_w as usize, doc_h as usize],
                &self.doc().composite_rgba,
            );
            let opts = if use_nearest {
                TextureOptions::NEAREST
            } else {
                TextureOptions::LINEAR
            };
            let handle = ctx.load_texture(format!("canvas_doc_{doc_id}"), img, opts);
            self.doc_textures.insert(
                doc_id,
                CachedDocTexture {
                    version: doc_ver,
                    nearest: use_nearest,
                    handle,
                },
            );
        }

        let avail = ui.available_size();
        self.last_canvas_viewport = avail;
        if !self.initial_fit_done && avail.x > 100.0 && avail.y > 100.0 {
            self.initial_fit_done = true;
            if doc_w as f32 + 40.0 > avail.x || doc_h as f32 + 40.0 > avail.y {
                self.doc_mut().zoom_to_window(avail.x, avail.y);
            }
        }
        let zoom = self.doc().zoom;
        let (vp_rect, resp) = ui.allocate_exact_size(avail, Sense::click_and_drag());

        // Dark neutral workspace surround (authentic to Pinta/GTK4 canvas backdrop)
        let surround_bg = if self.dark_mode {
            Color32::from_rgb(24, 26, 30)
        } else {
            Color32::from_rgb(198, 202, 209)
        };
        let painter = ui.painter_at(vp_rect);
        painter.rect_filled(vp_rect, 0.0, surround_bg);

        let ruler_sz = if self.show_rulers { 20.0 } else { 0.0 };
        let canvas_area = Rect::from_min_max(
            Pos2::new(vp_rect.left() + ruler_sz, vp_rect.top() + ruler_sz),
            vp_rect.max,
        );

        let scaled_w = doc_w as f32 * zoom;
        let scaled_h = doc_h as f32 * zoom;
        let pan = self.doc().pan;
        let canvas_center = canvas_area.center() + Vec2::new(pan[0], pan[1]);
        let canvas_rect = Rect::from_center_size(canvas_center, Vec2::new(scaled_w, scaled_h));

        // Soft drop shadow behind canvas
        painter.rect_filled(
            canvas_rect.translate(Vec2::new(3.0, 4.0)),
            0.0,
            Color32::from_black_alpha(55),
        );

        // Checkerboard background for transparency
        paint_checkerboard(&painter, canvas_rect.intersect(canvas_area), canvas_rect.min, 10.0);

        // Draw composited canvas texture
        if let Some(cached) = self.doc_textures.get(&doc_id) {
            painter.image(
                cached.handle.id(),
                canvas_rect,
                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                Color32::WHITE,
            );
        }

        // Canvas border
        painter.rect_stroke(
            canvas_rect,
            0.0,
            Stroke::new(1.0_f32, Color32::from_gray(80)),
            StrokeKind::Outside,
        );

        // Optional Pixel / Canvas Grid
        if self.show_grid {
            let gw = (self.grid_width.max(1) as f32) * zoom;
            let gh = (self.grid_height.max(1) as f32) * zoom;
            if gw >= 4.0 && gh >= 4.0 {
                let grid_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(80, 120, 190, 95));
                let mut x = canvas_rect.left() + gw;
                while x < canvas_rect.right() {
                    painter.line_segment(
                        [Pos2::new(x, canvas_rect.top()), Pos2::new(x, canvas_rect.bottom())],
                        grid_stroke,
                    );
                    x += gw;
                }
                let mut y = canvas_rect.top() + gh;
                while y < canvas_rect.bottom() {
                    painter.line_segment(
                        [Pos2::new(canvas_rect.left(), y), Pos2::new(canvas_rect.right(), y)],
                        grid_stroke,
                    );
                    y += gh;
                }
            }
        }

        // Animated marching ants selection outline
        if let Some(sel) = &self.doc().selection {
            if let Some((bx, by, bw, bh)) = sel.bounds() {
                let sr = Rect::from_min_size(
                    Pos2::new(
                        canvas_rect.left() + bx as f32 * zoom,
                        canvas_rect.top() + by as f32 * zoom,
                    ),
                    Vec2::new(bw as f32 * zoom, bh as f32 * zoom),
                );
                let phase = (ctx.input(|i| i.time) * 16.0) as f32;
                paint_marching_ants_rect(&painter, sr, phase);
            }
        }

        // Live selection drag rubber-band preview
        if self.tool_runtime.dragging {
            let to_screen = |pt: [f32; 2]| {
                Pos2::new(
                    canvas_rect.left() + pt[0] * zoom,
                    canvas_rect.top() + pt[1] * zoom,
                )
            };
            match self.active_tool {
                ToolKind::RectangleSelect | ToolKind::EllipseSelect | ToolKind::Zoom => {
                    let a = to_screen(self.tool_runtime.start_pt);
                    let b = to_screen(self.tool_runtime.cur_pt);
                    let r = Rect::from_two_pos(a, b);
                    painter.rect_stroke(
                        r,
                        0.0,
                        Stroke::new(1.2_f32, Color32::from_rgb(53, 132, 228)),
                        StrokeKind::Inside,
                    );
                }
                ToolKind::LassoSelect | ToolKind::FreeformShape => {
                    if self.tool_runtime.poly_points.len() >= 2 {
                        let pts: Vec<Pos2> = self
                            .tool_runtime
                            .poly_points
                            .iter()
                            .map(|&(x, y)| to_screen([x, y]))
                            .collect();
                        painter.line(pts, Stroke::new(1.4_f32, Color32::from_rgb(53, 132, 228)));
                    }
                }
                ToolKind::Gradient => {
                    let a = to_screen(self.tool_runtime.start_pt);
                    let b = to_screen(self.tool_runtime.cur_pt);
                    painter.line_segment([a, b], Stroke::new(1.5_f32, Color32::WHITE));
                    painter.circle_stroke(a, 4.0, Stroke::new(1.5_f32, Color32::BLACK));
                    painter.circle_stroke(b, 4.0, Stroke::new(1.5_f32, Color32::BLACK));
                }
                _ => {}
            }
        }

        // Clone Stamp source crosshair
        if self.active_tool == ToolKind::CloneStamp {
            if let Some([sx, sy]) = self.tool_runtime.clone_source {
                let p = Pos2::new(
                    canvas_rect.left() + (sx as f32 + 0.5) * zoom,
                    canvas_rect.top() + (sy as f32 + 0.5) * zoom,
                );
                let cs = Stroke::new(1.5_f32, Color32::from_rgb(255, 80, 80));
                painter.circle_stroke(p, 7.0, cs);
                painter.line_segment([Pos2::new(p.x - 10.0, p.y), Pos2::new(p.x + 10.0, p.y)], cs);
                painter.line_segment([Pos2::new(p.x, p.y - 10.0), Pos2::new(p.x, p.y + 10.0)], cs);
            }
        }

        // Brush cursor ring preview on hover
        if let Some(hover) = resp.hover_pos() {
            let cx = (hover.x - canvas_rect.left()) / zoom;
            let cy = (hover.y - canvas_rect.top()) / zoom;
            if cx >= 0.0 && cy >= 0.0 && cx < doc_w as f32 && cy < doc_h as f32 {
                self.cursor_canvas_pos = Some([cx as i32, cy as i32]);
            } else {
                self.cursor_canvas_pos = None;
            }
            if matches!(
                self.active_tool,
                ToolKind::Paintbrush | ToolKind::Eraser | ToolKind::CloneStamp | ToolKind::Recolor
            ) && canvas_area.contains(hover)
            {
                let rad = (self.tool_settings.brush_width as f32 * 0.5 * zoom).max(2.0);
                painter.circle_stroke(hover, rad, Stroke::new(1.0_f32, Color32::BLACK));
                painter.circle_stroke(
                    hover,
                    (rad - 1.0).max(1.0),
                    Stroke::new(1.0_f32, Color32::WHITE),
                );
            }
        } else {
            self.cursor_canvas_pos = None;
        }

        // Active Text Tool live inline input box on canvas
        if let Some(active_text) = self.tool_runtime.active_text.clone() {
            let screen_pos = Pos2::new(
                canvas_rect.left() + active_text.pos[0] * zoom,
                canvas_rect.top() + active_text.pos[1] * zoom,
            );
            let mut text_buf = active_text.text.clone();
            let mut commit_now = false;
            egui::Area::new(egui::Id::new("canvas_text_editor"))
                .fixed_pos(screen_pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 215))
                        .stroke(Stroke::new(1.2_f32, Color32::from_rgb(53, 132, 228)))
                        .inner_margin(Margin::same(3))
                        .show(ui, |ui| {
                            let te = ui.add(
                                egui::TextEdit::singleline(&mut text_buf)
                                    .font(FontId::proportional(
                                        (self.tool_settings.text_size * zoom).clamp(10.0, 96.0),
                                    ))
                                    .text_color(rgba_to_color32(active_text.color))
                                    .desired_width(220.0),
                            );
                            te.request_focus();
                            if te.lost_focus() && ctx.input(|i| i.key_pressed(Key::Enter)) {
                                commit_now = true;
                            }
                        });
                });
            if let Some(at) = &mut self.tool_runtime.active_text {
                at.text = text_buf;
            }
            if commit_now {
                self.commit_active_text(ctx);
            }
        }

        // Optional Rulers
        if self.show_rulers {
            paint_rulers(&painter, vp_rect, canvas_rect, zoom, self.dark_mode);
        }

        // --- Mouse Wheel Zoom & Pan ---
        if resp.hovered() {
            let (scroll_delta, modifiers) = ctx.input(|i| (i.raw_scroll_delta, i.modifiers));
            if scroll_delta.y.abs() > 0.1 {
                if modifiers.ctrl || modifiers.command {
                    if scroll_delta.y > 0.0 {
                        self.doc_mut().zoom_in();
                    } else {
                        self.doc_mut().zoom_out();
                    }
                } else {
                    self.doc_mut().pan[0] += scroll_delta.x;
                    self.doc_mut().pan[1] += scroll_delta.y;
                }
            }
        }

        // --- Tool Pointer Interaction ---
        let space_pan = ctx.input(|i| i.key_down(Key::Space));
        let middle_drag = ctx.input(|i| i.pointer.middle_down());
        if (space_pan || middle_drag || self.active_tool == ToolKind::Pan) && resp.dragged() {
            let delta = resp.drag_delta();
            self.doc_mut().pan[0] += delta.x;
            self.doc_mut().pan[1] += delta.y;
            return;
        }

        self.handle_canvas_pointer(ctx, &resp, canvas_rect, zoom);
    }

    fn handle_canvas_pointer(
        &mut self,
        ctx: &egui::Context,
        resp: &egui::Response,
        canvas_rect: Rect,
        zoom: f32,
    ) {
        let Some(pointer_pos) = resp.interact_pointer_pos() else {
            return;
        };
        let pt = [
            (pointer_pos.x - canvas_rect.left()) / zoom,
            (pointer_pos.y - canvas_rect.top()) / zoom,
        ];
        let modifiers: Modifiers = ctx.input(|i| i.modifiers);
        let secondary = ctx.input(|i| i.pointer.secondary_down() || i.pointer.secondary_clicked());
        let color = if secondary {
            self.palette.secondary
        } else {
            self.palette.primary
        };
        let other_color = if secondary {
            self.palette.primary
        } else {
            self.palette.secondary
        };

        let tool = self.active_tool;

        // Instant click tools (Zoom, PaintBucket, MagicWand, ColorPicker, Text, CloneStamp Ctrl+Click)
        if resp.clicked() || resp.secondary_clicked() {
            let ix = pt[0].floor() as i32;
            let iy = pt[1].floor() as i32;
            match tool {
                ToolKind::Zoom => {
                    if resp.secondary_clicked() {
                        self.doc_mut().zoom_out();
                    } else {
                        self.doc_mut().zoom_in();
                    }
                    return;
                }
                ToolKind::PaintBucket => {
                    if ix >= 0 && iy >= 0 {
                        let tol = self.tool_settings.tolerance;
                        let contig = self.tool_settings.flood_contiguous;
                        let ov = self.tool_settings.overwrite_blend;
                        let doc = self.doc_mut();
                        let w = doc.width;
                        let h = doc.height;
                        let sel = doc.selection.clone();
                        flood_fill(
                            &mut doc.active_layer_mut().pixels,
                            w,
                            h,
                            sel.as_ref(),
                            ix as u32,
                            iy as u32,
                            color,
                            tol,
                            contig,
                            ov,
                        );
                        doc.commit_history("Paint Bucket");
                    }
                    return;
                }
                ToolKind::MagicWand => {
                    if ix >= 0 && iy >= 0 {
                        let tol = self.tool_settings.tolerance;
                        let contig = self.tool_settings.flood_contiguous;
                        let mode = self.tool_settings.selection_mode;
                        let doc = self.doc_mut();
                        let w = doc.width;
                        let h = doc.height;
                        let mask = magic_wand_mask(
                            &doc.active_layer_ref().pixels,
                            w,
                            h,
                            ix as u32,
                            iy as u32,
                            tol,
                            contig,
                        );
                        apply_selection_mask(doc, mask, mode);
                    }
                    return;
                }
                ToolKind::ColorPicker => {
                    let sample_sz = self.tool_settings.picker_sample_size;
                    let layer_only = self.tool_settings.picker_sample_layer_only;
                    let picked = {
                        let doc = self.doc();
                        let buf = if layer_only {
                            &doc.active_layer_ref().pixels
                        } else {
                            &doc.composite_rgba
                        };
                        sample_color(buf, doc.width, doc.height, ix, iy, sample_sz)
                    };
                    if resp.secondary_clicked() {
                        self.palette.set_secondary(picked);
                    } else {
                        self.palette.set_primary(picked);
                    }
                    return;
                }
                ToolKind::Text => {
                    self.commit_active_text(ctx);
                    self.tool_runtime.active_text = Some(ActiveTextState {
                        pos: pt,
                        text: String::new(),
                        color: self.palette.primary,
                    });
                    return;
                }
                ToolKind::CloneStamp if modifiers.ctrl || modifiers.command => {
                    self.tool_runtime.clone_source = Some([ix, iy]);
                    self.status_message = Some(format!("Clone source set to ({ix}, {iy})."));
                    return;
                }
                ToolKind::Paintbrush => {
                    let bw = self.tool_settings.brush_width as f32;
                    let aa = self.tool_settings.antialiasing;
                    let ov = self.tool_settings.overwrite_blend;
                    let doc = self.doc_mut();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    stroke_segment(
                        &mut doc.active_layer_mut().pixels,
                        w,
                        h,
                        sel.as_ref(),
                        pt,
                        pt,
                        bw,
                        color,
                        aa,
                        ov,
                        false,
                    );
                    doc.commit_history("Paintbrush");
                    return;
                }
                ToolKind::Pencil => {
                    let ov = self.tool_settings.overwrite_blend;
                    let doc = self.doc_mut();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    draw_pencil_line(
                        &mut doc.active_layer_mut().pixels,
                        w,
                        h,
                        sel.as_ref(),
                        ix,
                        iy,
                        ix,
                        iy,
                        color,
                        ov,
                    );
                    doc.commit_history("Pencil");
                    return;
                }
                ToolKind::Eraser => {
                    let bw = self.tool_settings.brush_width as f32;
                    let aa = self.tool_settings.antialiasing;
                    let doc = self.doc_mut();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    stroke_segment(
                        &mut doc.active_layer_mut().pixels,
                        w,
                        h,
                        sel.as_ref(),
                        pt,
                        pt,
                        bw,
                        [0, 0, 0, 0],
                        aa,
                        true,
                        true,
                    );
                    doc.commit_history("Eraser");
                    return;
                }
                _ => {}
            }
        }

        // --- Drag Start ---
        if resp.drag_started() {
            if tool == ToolKind::CloneStamp && (modifiers.ctrl || modifiers.command) {
                self.tool_runtime.clone_source = Some([pt[0].floor() as i32, pt[1].floor() as i32]);
                return;
            }
            self.tool_runtime.dragging = true;
            self.tool_runtime.secondary_drag = secondary;
            self.tool_runtime.start_pt = pt;
            self.tool_runtime.last_pt = pt;
            self.tool_runtime.cur_pt = pt;
            self.tool_runtime.poly_points.clear();
            self.tool_runtime.poly_points.push((pt[0], pt[1]));

            match tool {
                ToolKind::CloneStamp => {
                    if let Some(src) = self.tool_runtime.clone_source {
                        self.tool_runtime.clone_drag_offset =
                            Some([src[0] - pt[0].floor() as i32, src[1] - pt[1].floor() as i32]);
                    } else {
                        self.tool_runtime.clone_drag_offset = None;
                    }
                }
                ToolKind::MoveSelectedPixels => {
                    let st = begin_move_pixels(self.doc(), [pt[0].round() as i32, pt[1].round() as i32]);
                    self.tool_runtime.move_state = Some(st);
                }
                ToolKind::MoveSelection => {
                    self.tool_runtime.orig_selection_for_move = self.doc().selection.clone();
                }
                _ => {}
            }
        }

        // --- Dragging ---
        if resp.dragged() && self.tool_runtime.dragging {
            let draw_col = if self.tool_runtime.secondary_drag {
                self.palette.secondary
            } else {
                self.palette.primary
            };
            let alt_col = if self.tool_runtime.secondary_drag {
                self.palette.primary
            } else {
                self.palette.secondary
            };
            let prev = self.tool_runtime.last_pt;
            self.tool_runtime.cur_pt = pt;

            match tool {
                ToolKind::Paintbrush => {
                    let bw = self.tool_settings.brush_width as f32;
                    let aa = self.tool_settings.antialiasing;
                    let ov = self.tool_settings.overwrite_blend;
                    let doc = self.doc_mut();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    stroke_segment(
                        &mut doc.active_layer_mut().pixels,
                        w,
                        h,
                        sel.as_ref(),
                        prev,
                        pt,
                        bw,
                        draw_col,
                        aa,
                        ov,
                        false,
                    );
                    doc.mark_dirty();
                }
                ToolKind::Pencil => {
                    let ov = self.tool_settings.overwrite_blend;
                    let doc = self.doc_mut();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    draw_pencil_line(
                        &mut doc.active_layer_mut().pixels,
                        w,
                        h,
                        sel.as_ref(),
                        prev[0].floor() as i32,
                        prev[1].floor() as i32,
                        pt[0].floor() as i32,
                        pt[1].floor() as i32,
                        draw_col,
                        ov,
                    );
                    doc.mark_dirty();
                }
                ToolKind::Eraser => {
                    let bw = self.tool_settings.brush_width as f32;
                    let aa = self.tool_settings.antialiasing;
                    let doc = self.doc_mut();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    stroke_segment(
                        &mut doc.active_layer_mut().pixels,
                        w,
                        h,
                        sel.as_ref(),
                        prev,
                        pt,
                        bw,
                        [0, 0, 0, 0],
                        aa,
                        true,
                        true,
                    );
                    doc.mark_dirty();
                }
                ToolKind::CloneStamp => {
                    if let Some(off) = self.tool_runtime.clone_drag_offset {
                        let bw = self.tool_settings.brush_width as f32;
                        let doc = self.doc_mut();
                        let w = doc.width;
                        let h = doc.height;
                        let sel = doc.selection.clone();
                        clone_stamp_segment(
                            &mut doc.active_layer_mut().pixels,
                            w,
                            h,
                            sel.as_ref(),
                            prev,
                            pt,
                            off,
                            bw,
                        );
                        doc.mark_dirty();
                    }
                }
                ToolKind::Recolor => {
                    let bw = self.tool_settings.brush_width as f32;
                    let tol = self.tool_settings.tolerance;
                    let doc = self.doc_mut();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    recolor_segment(
                        &mut doc.active_layer_mut().pixels,
                        w,
                        h,
                        sel.as_ref(),
                        prev,
                        pt,
                        bw,
                        draw_col,
                        alt_col,
                        tol,
                    );
                    doc.mark_dirty();
                }
                ToolKind::LineCurve
                | ToolKind::Rectangle
                | ToolKind::RoundedRectangle
                | ToolKind::Ellipse => {
                    let settings = self.tool_settings.clone();
                    let start = self.tool_runtime.start_pt;
                    let doc = self.doc_mut();
                    let mut buf = doc.active_layer_ref().pixels.clone();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    draw_shape(
                        &mut buf,
                        w,
                        h,
                        sel.as_ref(),
                        tool,
                        start,
                        pt,
                        draw_col,
                        alt_col,
                        &settings,
                        modifiers.shift,
                    );
                    let li = doc.active_layer;
                    doc.preview_layer_pixels = Some((li, buf));
                    doc.mark_dirty();
                }
                ToolKind::Gradient => {
                    let start = self.tool_runtime.start_pt;
                    let gk = self.tool_settings.gradient_kind;
                    let gm = self.tool_settings.gradient_mode;
                    let doc = self.doc_mut();
                    let mut buf = doc.active_layer_ref().pixels.clone();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    render_gradient(&mut buf, w, h, sel.as_ref(), start, pt, draw_col, alt_col, gk, gm);
                    let li = doc.active_layer;
                    doc.preview_layer_pixels = Some((li, buf));
                    doc.mark_dirty();
                }
                ToolKind::LassoSelect | ToolKind::FreeformShape => {
                    let last = self.tool_runtime.poly_points.last().copied();
                    if last.map_or(true, |(lx, ly)| (lx - pt[0]).hypot(ly - pt[1]) >= 2.0) {
                        self.tool_runtime.poly_points.push((pt[0], pt[1]));
                    }
                }
                ToolKind::MoveSelectedPixels => {
                    let w = self.doc().width;
                    let h = self.doc().height;
                    if let Some(st) = &mut self.tool_runtime.move_state {
                        st.offset = [
                            pt[0].round() as i32 - st.start_pt[0],
                            pt[1].round() as i32 - st.start_pt[1],
                        ];
                        let dx = st.offset[0];
                        let dy = st.offset[1];
                        let translated_sel = st.orig_selection.as_ref().map(|s| s.translate(dx, dy));
                        let composed = compose_move_pixels(st, w, h);
                        let doc = self.doc_mut();
                        let li = doc.active_layer;
                        doc.preview_layer_pixels = Some((li, composed));
                        doc.selection = translated_sel;
                        doc.mark_dirty();
                    }
                }
                ToolKind::MoveSelection => {
                    if let Some(orig) = &self.tool_runtime.orig_selection_for_move {
                        let dx = (pt[0] - self.tool_runtime.start_pt[0]).round() as i32;
                        let dy = (pt[1] - self.tool_runtime.start_pt[1]).round() as i32;
                        self.doc_mut().selection = Some(orig.translate(dx, dy));
                    }
                }
                _ => {}
            }
            self.tool_runtime.last_pt = pt;
        }

        // --- Drag Stopped (Commit) ---
        if resp.drag_stopped() && self.tool_runtime.dragging {
            self.tool_runtime.dragging = false;
            let draw_col = if self.tool_runtime.secondary_drag {
                other_color
            } else {
                color
            };
            let alt_col = if self.tool_runtime.secondary_drag {
                color
            } else {
                other_color
            };
            let start = self.tool_runtime.start_pt;
            let end = self.tool_runtime.cur_pt;

            match tool {
                ToolKind::Paintbrush
                | ToolKind::Pencil
                | ToolKind::Eraser
                | ToolKind::CloneStamp
                | ToolKind::Recolor => {
                    self.doc_mut().commit_history(tool.name());
                }
                ToolKind::LineCurve
                | ToolKind::Rectangle
                | ToolKind::RoundedRectangle
                | ToolKind::Ellipse
                | ToolKind::Gradient => {
                    let doc = self.doc_mut();
                    if let Some((li, buf)) = doc.preview_layer_pixels.take() {
                        if li < doc.layers.len() {
                            doc.layers[li].pixels = buf;
                            doc.commit_history(tool.name());
                        }
                    }
                }
                ToolKind::FreeformShape => {
                    let pts = self.tool_runtime.poly_points.clone();
                    let settings = self.tool_settings.clone();
                    let doc = self.doc_mut();
                    let w = doc.width;
                    let h = doc.height;
                    let sel = doc.selection.clone();
                    draw_freeform_shape(
                        &mut doc.active_layer_mut().pixels,
                        w,
                        h,
                        sel.as_ref(),
                        &pts,
                        draw_col,
                        alt_col,
                        &settings,
                    );
                    doc.commit_history("Freeform Shape");
                }
                ToolKind::RectangleSelect => {
                    let mode = self.tool_settings.selection_mode;
                    let doc = self.doc_mut();
                    let mut p1 = end;
                    if modifiers.shift {
                        let s = (end[0] - start[0]).abs().max((end[1] - start[1]).abs());
                        p1 = [
                            start[0] + s * if end[0] >= start[0] { 1.0 } else { -1.0 },
                            start[1] + s * if end[1] >= start[1] { 1.0 } else { -1.0 },
                        ];
                    }
                    let mask = SelectionMask::from_rect(
                        doc.width,
                        doc.height,
                        start[0].round() as i32,
                        start[1].round() as i32,
                        p1[0].round() as i32,
                        p1[1].round() as i32,
                    );
                    apply_selection_mask(doc, mask, mode);
                }
                ToolKind::EllipseSelect => {
                    let mode = self.tool_settings.selection_mode;
                    let doc = self.doc_mut();
                    let mut p1 = end;
                    if modifiers.shift {
                        let s = (end[0] - start[0]).abs().max((end[1] - start[1]).abs());
                        p1 = [
                            start[0] + s * if end[0] >= start[0] { 1.0 } else { -1.0 },
                            start[1] + s * if end[1] >= start[1] { 1.0 } else { -1.0 },
                        ];
                    }
                    let mask = SelectionMask::from_ellipse(
                        doc.width,
                        doc.height,
                        start[0].round() as i32,
                        start[1].round() as i32,
                        p1[0].round() as i32,
                        p1[1].round() as i32,
                    );
                    apply_selection_mask(doc, mask, mode);
                }
                ToolKind::LassoSelect => {
                    let mode = self.tool_settings.selection_mode;
                    let pts = self.tool_runtime.poly_points.clone();
                    let doc = self.doc_mut();
                    let mask = SelectionMask::from_polygon(doc.width, doc.height, &pts);
                    apply_selection_mask(doc, mask, mode);
                }
                ToolKind::MoveSelectedPixels => {
                    self.tool_runtime.move_state = None;
                    let doc = self.doc_mut();
                    if let Some((li, buf)) = doc.preview_layer_pixels.take() {
                        if li < doc.layers.len() {
                            doc.layers[li].pixels = buf;
                            doc.commit_history("Move Selected Pixels");
                        }
                    }
                }
                ToolKind::MoveSelection => {
                    self.tool_runtime.orig_selection_for_move = None;
                    self.doc_mut().commit_history("Move Selection");
                }
                _ => {}
            }
        }
    }

    fn show_dialogs(&mut self, ctx: &egui::Context) {
        // --- New Image Dialog ---
        if let Some(mut st) = self.new_image_dialog.clone() {
            let mut open = true;
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("New Image")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Preset:");
                        for &(label, pw, ph) in &IMAGE_PRESETS {
                            if ui.small_button(label).clicked() {
                                st.width = pw as i32;
                                st.height = ph as i32;
                            }
                        }
                    });
                    ui.add_space(4.0);
                    egui::Grid::new("new_img_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Width:");
                        ui.add(egui::DragValue::new(&mut st.width).range(1..=4096).suffix(" px"));
                        ui.end_row();
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(&mut st.height).range(1..=4096).suffix(" px"));
                        ui.end_row();
                        ui.label("Orientation:");
                        ui.horizontal(|ui| {
                            if ui.selectable_label(st.height > st.width, "Portrait").clicked()
                                && st.width > st.height
                            {
                                std::mem::swap(&mut st.width, &mut st.height);
                            }
                            if ui.selectable_label(st.width >= st.height, "Landscape").clicked()
                                && st.height > st.width
                            {
                                std::mem::swap(&mut st.width, &mut st.height);
                            }
                        });
                        ui.end_row();
                        ui.label("Background:");
                        ui.vertical(|ui| {
                            ui.radio_value(&mut st.background, BackgroundFill::White, "White");
                            ui.radio_value(
                                &mut st.background,
                                BackgroundFill::SecondaryColor,
                                "Background Color",
                            );
                            ui.radio_value(
                                &mut st.background,
                                BackgroundFill::Transparent,
                                "Transparent",
                            );
                        });
                        ui.end_row();
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            confirm = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if confirm {
                self.create_new_document(st.width.max(1) as u32, st.height.max(1) as u32, st.background);
                self.new_image_dialog = None;
            } else if open && !cancel {
                self.new_image_dialog = Some(st);
            } else {
                self.new_image_dialog = None;
            }
        }

        // --- Resize Image Dialog ---
        if let Some(mut st) = self.resize_image_dialog.clone() {
            let mut open = true;
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Resize Image")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.radio_value(&mut st.by_percentage, true, "By percentage:");
                    if st.by_percentage {
                        if ui
                            .add(egui::DragValue::new(&mut st.percentage).range(1..=1000).suffix("%"))
                            .changed()
                        {
                            st.width = ((st.orig_w as f32 * st.percentage as f32 / 100.0).round() as i32).max(1);
                            st.height = ((st.orig_h as f32 * st.percentage as f32 / 100.0).round() as i32).max(1);
                        }
                    }
                    ui.radio_value(&mut st.by_percentage, false, "By absolute size:");
                    if !st.by_percentage {
                        ui.horizontal(|ui| {
                            ui.label("Width:");
                            if ui.add(egui::DragValue::new(&mut st.width).range(1..=4096).suffix(" px")).changed()
                                && st.maintain_aspect
                            {
                                st.height = ((st.width as f32 * st.orig_h as f32 / st.orig_w.max(1) as f32).round() as i32).max(1);
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Height:");
                            if ui.add(egui::DragValue::new(&mut st.height).range(1..=4096).suffix(" px")).changed()
                                && st.maintain_aspect
                            {
                                st.width = ((st.height as f32 * st.orig_w as f32 / st.orig_h.max(1) as f32).round() as i32).max(1);
                            }
                        });
                        ui.checkbox(&mut st.maintain_aspect, "Maintain aspect ratio");
                    }
                    ui.horizontal(|ui| {
                        ui.label("Resampling:");
                        ui.radio_value(&mut st.resampling, ResamplingMode::Bilinear, "Bilinear");
                        ui.radio_value(&mut st.resampling, ResamplingMode::NearestNeighbor, "Nearest");
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            confirm = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if confirm {
                self.doc_mut().resize_image(st.width.max(1) as u32, st.height.max(1) as u32, st.resampling);
                self.resize_image_dialog = None;
            } else if open && !cancel {
                self.resize_image_dialog = Some(st);
            } else {
                self.resize_image_dialog = None;
            }
        }

        // --- Resize Canvas Dialog (9-point Anchor grid) ---
        if let Some(mut st) = self.resize_canvas_dialog.clone() {
            let mut open = true;
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Resize Canvas")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(egui::DragValue::new(&mut st.width).range(1..=4096).suffix(" px"));
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(&mut st.height).range(1..=4096).suffix(" px"));
                    });
                    ui.add_space(4.0);
                    ui.label("Anchor:");
                    let anchors = [
                        [Anchor::TopLeft, Anchor::Top, Anchor::TopRight],
                        [Anchor::Left, Anchor::Center, Anchor::Right],
                        [Anchor::BottomLeft, Anchor::Bottom, Anchor::BottomRight],
                    ];
                    let labels = [
                        ["↖", "↑", "↗"],
                        ["←", "•", "→"],
                        ["↙", "↓", "↘"],
                    ];
                    egui::Grid::new("anchor_grid").spacing([3.0, 3.0]).show(ui, |ui| {
                        for r in 0..3 {
                            for c in 0..3 {
                                let a = anchors[r][c];
                                if ui
                                    .add_sized(
                                        Vec2::new(30.0, 30.0),
                                        egui::SelectableLabel::new(st.anchor == a, labels[r][c]),
                                    )
                                    .clicked()
                                {
                                    st.anchor = a;
                                }
                            }
                            ui.end_row();
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            confirm = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if confirm {
                self.doc_mut().resize_canvas(st.width.max(1) as u32, st.height.max(1) as u32, st.anchor);
                self.resize_canvas_dialog = None;
            } else if open && !cancel {
                self.resize_canvas_dialog = Some(st);
            } else {
                self.resize_canvas_dialog = None;
            }
        }

        // --- Layer Properties Dialog ---
        if let Some(mut st) = self.layer_props_dialog.clone() {
            let mut open = true;
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Layer Properties")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    egui::Grid::new("layer_props_grid").num_columns(2).show(ui, |ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut st.name);
                        ui.end_row();
                        ui.label("Visible:");
                        ui.checkbox(&mut st.visible, "");
                        ui.end_row();
                        ui.label("Blend Mode:");
                        egui::ComboBox::from_id_salt("props_blend")
                            .selected_text(st.blend_mode.label())
                            .show_ui(ui, |ui| {
                                for bm in BlendMode::ALL {
                                    ui.selectable_value(&mut st.blend_mode, bm, bm.label());
                                }
                            });
                        ui.end_row();
                        ui.label("Opacity:");
                        ui.add(egui::Slider::new(&mut st.opacity_pct, 0..=100).suffix("%"));
                        ui.end_row();
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            confirm = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if confirm {
                let doc = self.doc_mut();
                let l = doc.active_layer_mut();
                l.name = st.name;
                l.visible = st.visible;
                l.opacity = st.opacity_pct as f32 / 100.0;
                l.blend_mode = st.blend_mode;
                doc.commit_history("Layer Properties");
                self.layer_props_dialog = None;
            } else if open && !cancel {
                self.layer_props_dialog = Some(st);
            } else {
                self.layer_props_dialog = None;
            }
        }

        // --- Canvas Grid Dialog ---
        if let Some(mut st) = self.grid_dialog.clone() {
            let mut open = true;
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Canvas Grid Settings")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.checkbox(&mut st.show_grid, "Show Grid");
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(egui::DragValue::new(&mut st.cell_width).range(2..=512).suffix(" px"));
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(&mut st.cell_height).range(2..=512).suffix(" px"));
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            confirm = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if confirm {
                self.show_grid = st.show_grid;
                self.grid_width = st.cell_width.max(2);
                self.grid_height = st.cell_height.max(2);
                self.grid_dialog = None;
            } else if open && !cancel {
                self.grid_dialog = Some(st);
            } else {
                self.grid_dialog = None;
            }
        }

        // --- Color Picker Popup ---
        if let Some(target) = self.color_picker_target {
            let mut open = true;
            let title = match target {
                ColorPickerTarget::Primary => "Primary Color",
                ColorPickerTarget::Secondary => "Secondary Color",
            };
            let cur = match target {
                ColorPickerTarget::Primary => self.palette.primary,
                ColorPickerTarget::Secondary => self.palette.secondary,
            };
            let mut c32 = rgba_to_color32(cur);
            egui::Window::new(title)
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    if egui::color_picker::color_picker_color32(
                        ui,
                        &mut c32,
                        egui::color_picker::Alpha::OnlyBlend,
                    ) {
                        let rgba = color32_to_rgba(c32);
                        match target {
                            ColorPickerTarget::Primary => self.palette.set_primary(rgba),
                            ColorPickerTarget::Secondary => self.palette.set_secondary(rgba),
                        }
                    }
                    ui.horizontal(|ui| {
                        let rgba = color32_to_rgba(c32);
                        ui.label(format!(
                            "Hex: #{:02X}{:02X}{:02X}  RGBA: ({}, {}, {}, {})",
                            rgba[0], rgba[1], rgba[2], rgba[0], rgba[1], rgba[2], rgba[3]
                        ));
                    });
                });
            if !open {
                self.color_picker_target = None;
            }
        }

        // --- Live-Preview Effect / Adjustment Dialog ---
        if self.effect_dialog.is_some() {
            let mut open = true;
            let mut ok_clicked = false;
            let mut cancel_clicked = false;
            let mut changed = false;

            if let Some(dlg) = &mut self.effect_dialog {
                let title = dlg.spec.name();
                egui::Window::new(title)
                    .collapsible(false)
                    .resizable(false)
                    .open(&mut open)
                    .show(ctx, |ui| {
                        changed = show_effect_controls(ui, &mut dlg.spec);
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() {
                                ok_clicked = true;
                            }
                            if ui.button("Cancel").clicked() {
                                cancel_clicked = true;
                            }
                        });
                    });
            }

            if changed {
                if let Some(dlg) = &mut self.effect_dialog {
                    dlg.needs_preview = true;
                }
            }

            if let Some(dlg) = &mut self.effect_dialog {
                if dlg.needs_preview {
                    let primary = self.palette.primary;
                    let secondary = self.palette.secondary;
                    let spec = dlg.spec.clone();
                    let orig = dlg.original_pixels.clone();
                    dlg.needs_preview = false;
                    let doc = self.doc_mut();
                    let preview = apply_effect(
                        &orig,
                        doc.width,
                        doc.height,
                        doc.selection.as_ref(),
                        primary,
                        secondary,
                        &spec,
                    );
                    let li = doc.active_layer;
                    doc.preview_layer_pixels = Some((li, preview));
                    doc.mark_dirty();
                }
            }

            if ok_clicked {
                if let Some(dlg) = self.effect_dialog.take() {
                    let name = dlg.spec.name();
                    let doc = self.doc_mut();
                    if let Some((li, buf)) = doc.preview_layer_pixels.take() {
                        if li < doc.layers.len() {
                            doc.layers[li].pixels = buf;
                            doc.commit_history(name);
                        }
                    }
                }
            } else if cancel_clicked || !open {
                self.effect_dialog = None;
                let doc = self.doc_mut();
                doc.preview_layer_pixels = None;
                doc.mark_dirty();
            }
        }

        // --- About Dialog ---
        if self.show_about {
            let mut open = true;
            egui::Window::new("About Canvas Studio")
                .collapsible(false)
                .resizable(false)
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.heading("Canvas Studio");
                    ui.label("Layered raster drawing and photo editing desktop application.");
                    ui.add_space(4.0);
                    ui.label("Built with Rust & egui/eframe.");
                });
            if !open {
                self.show_about = false;
            }
        }
    }

    fn handle_screenshot_lifecycle(&mut self, ctx: &egui::Context) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = ctx;
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(target_path) = self.screenshot_path.clone() else {
                return;
            };

            // Check if screenshot event arrived
            let captured = ctx.input(|i| {
                for ev in &i.raw.events {
                    if let egui::Event::Screenshot { image, .. } = ev {
                        return Some(image.clone());
                    }
                }
                None
            });

            if let Some(img) = captured {
                let w = img.width() as u32;
                let h = img.height() as u32;
                let mut rgba = Vec::with_capacity((w * h * 4) as usize);
                for c in &img.pixels {
                    rgba.extend_from_slice(&c.to_srgba_unmultiplied());
                }
                let _ = file_io::save_rgba_file(&target_path, w, h, &rgba);
                self.screenshot_saved = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }

            self.frame_count = self.frame_count.saturating_add(1);
            ctx.request_repaint();

            if self.frame_count >= 115 && !self.screenshot_requested {
                self.screenshot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            } else if self.frame_count == 180 && !self.screenshot_saved {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            } else if self.frame_count >= 240 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

impl eframe::App for CanvasStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        #[cfg(target_arch = "wasm32")]
        self.poll_web_files(ctx);

        self.apply_theme(ctx);
        self.handle_global_shortcuts(ctx);

        // Top Menu Bar
        egui::TopBottomPanel::top("menu_bar")
            .exact_height(26.0)
            .show(ctx, |ui| {
                self.show_menu_bar(ctx, ui);
            });

        // Main Toolbar
        if self.show_toolbar {
            egui::TopBottomPanel::top("main_toolbar")
                .exact_height(32.0)
                .show(ctx, |ui| {
                    ui.vertical_centered_justified(|ui| {
                        self.show_main_toolbar(ui);
                    });
                });
        }

        // Tool Options Bar (44-48px height, matching Pinta CreateToolToolBar RVA 0x6ba8)
        egui::TopBottomPanel::top("tool_options_bar")
            .exact_height(TOOL_OPTIONS_BAR_HEIGHT)
            .show(ctx, |ui| {
                self.show_tool_options_bar(ctx, ui);
            });

        // Bottom Status & Palette Bar (StatusBarColorPaletteWidget)
        if self.show_statusbar {
            egui::TopBottomPanel::bottom("status_palette_bar")
                .exact_height(46.0)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        self.show_status_and_palette_bar(ui);
                    });
                });
        }

        // Left ToolBox (2-column grid of 22 tools)
        if self.show_toolbox {
            egui::SidePanel::left("toolbox_panel")
                .resizable(false)
                .exact_width(76.0)
                .show(ctx, |ui| {
                    self.show_toolbox_panel(ctx, ui);
                });
        }

        // Right Dock (Layers + History)
        if self.show_tool_windows {
            egui::SidePanel::right("right_dock_panel")
                .resizable(true)
                .default_width(224.0)
                .width_range(180.0..=340.0)
                .show(ctx, |ui| {
                    self.show_right_dock(ctx, ui);
                });
        }

        // Center Workspace (Document Tabs + Canvas Viewport)
        let center_strip_bg = if self.dark_mode {
            Color32::from_rgb(30, 32, 36)
        } else {
            Color32::from_rgb(232, 234, 238)
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(center_strip_bg).inner_margin(Margin::ZERO))
            .show(ctx, |ui| {
                if self.show_image_tabs {
                    egui::Frame::new()
                        .fill(center_strip_bg)
                        .inner_margin(Margin::symmetric(6, 3))
                        .show(ui, |ui| {
                            self.show_document_tabs(ui);
                        });
                }
                self.show_canvas_viewport(ctx, ui);
            });

        self.show_dialogs(ctx);

        // Animate marching ants gently when selection is active
        if self.doc().selection.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        self.handle_screenshot_lifecycle(ctx);
    }
}

fn apply_selection_mask(doc: &mut Document, mask: SelectionMask, mode: SelectionCombineMode) {
    match (&mut doc.selection, mode) {
        (Some(existing), m) if m != SelectionCombineMode::Replace => {
            existing.combine(&mask, m);
            if existing.is_empty() {
                doc.selection = None;
            }
        }
        _ => {
            doc.selection = if mask.is_empty() { None } else { Some(mask) };
        }
    }
    doc.commit_history("Selection");
}

fn show_effect_controls(ui: &mut egui::Ui, spec: &mut EffectSpec) -> bool {
    let mut ch = false;
    match spec {
        EffectSpec::BrightnessContrast {
            brightness,
            contrast,
        } => {
            ch |= ui
                .add(egui::Slider::new(brightness, -100..=100).text("Brightness"))
                .changed();
            ch |= ui
                .add(egui::Slider::new(contrast, -100..=100).text("Contrast"))
                .changed();
        }
        EffectSpec::HueSaturation {
            hue,
            saturation,
            lightness,
        } => {
            ch |= ui
                .add(egui::Slider::new(hue, -180..=180).text("Hue").suffix("°"))
                .changed();
            ch |= ui
                .add(egui::Slider::new(saturation, 0..=200).text("Saturation"))
                .changed();
            ch |= ui
                .add(egui::Slider::new(lightness, -100..=100).text("Lightness"))
                .changed();
        }
        EffectSpec::Levels {
            in_black,
            in_white,
            gamma,
            out_black,
            out_white,
        } => {
            ch |= ui.add(egui::Slider::new(in_black, 0..=254).text("Input Black")).changed();
            ch |= ui.add(egui::Slider::new(in_white, 1..=255).text("Input White")).changed();
            ch |= ui.add(egui::Slider::new(gamma, 0.1..=5.0).text("Gamma")).changed();
            ch |= ui.add(egui::Slider::new(out_black, 0..=255).text("Output Black")).changed();
            ch |= ui.add(egui::Slider::new(out_white, 0..=255).text("Output White")).changed();
        }
        EffectSpec::Posterize {
            red,
            green,
            blue,
            linked,
        } => {
            ui.checkbox(linked, "Linked");
            if ui.add(egui::Slider::new(red, 2..=64).text("Red")).changed() {
                if *linked {
                    *green = *red;
                    *blue = *red;
                }
                ch = true;
            }
            if ui.add(egui::Slider::new(green, 2..=64).text("Green")).changed() {
                if *linked {
                    *red = *green;
                    *blue = *green;
                }
                ch = true;
            }
            if ui.add(egui::Slider::new(blue, 2..=64).text("Blue")).changed() {
                if *linked {
                    *red = *blue;
                    *green = *blue;
                }
                ch = true;
            }
        }
        EffectSpec::GaussianBlur { radius } => {
            ch |= ui.add(egui::Slider::new(radius, 1..=30).text("Radius")).changed();
        }
        EffectSpec::MotionBlur {
            angle,
            distance,
            centered,
        } => {
            ch |= ui.add(egui::Slider::new(angle, -180.0..=180.0).text("Angle").suffix("°")).changed();
            ch |= ui.add(egui::Slider::new(distance, 1..=50).text("Distance")).changed();
            ch |= ui.checkbox(centered, "Centered").changed();
        }
        EffectSpec::RadialBlur { angle } => {
            ch |= ui.add(egui::Slider::new(angle, 0.5..=25.0).text("Angle").suffix("°")).changed();
        }
        EffectSpec::ZoomBlur { amount } => {
            ch |= ui.add(egui::Slider::new(amount, 1..=100).text("Zoom Amount")).changed();
        }
        EffectSpec::Sharpen { amount } => {
            ch |= ui.add(egui::Slider::new(amount, 1..=20).text("Amount")).changed();
        }
        EffectSpec::Glow {
            radius,
            brightness,
            contrast,
        } => {
            ch |= ui.add(egui::Slider::new(radius, 1..=20).text("Radius")).changed();
            ch |= ui.add(egui::Slider::new(brightness, -100..=100).text("Brightness")).changed();
            ch |= ui.add(egui::Slider::new(contrast, -100..=100).text("Contrast")).changed();
        }
        EffectSpec::Vignette { radius, density } => {
            ch |= ui.add(egui::Slider::new(radius, 10..=100).text("Radius")).changed();
            ch |= ui.add(egui::Slider::new(density, 0.1..=4.0).text("Density")).changed();
        }
        EffectSpec::Pixelate { cell_size } => {
            ch |= ui.add(egui::Slider::new(cell_size, 1..=100).text("Cell Size")).changed();
        }
        EffectSpec::Bulge { amount } => {
            ch |= ui.add(egui::Slider::new(amount, -200..=100).text("Bulge")).changed();
        }
        EffectSpec::Twist { amount, size } => {
            ch |= ui.add(egui::Slider::new(amount, -100..=100).text("Amount")).changed();
            ch |= ui.add(egui::Slider::new(size, 10..=200).text("Size")).changed();
        }
        EffectSpec::FrostedGlass { radius } => {
            ch |= ui.add(egui::Slider::new(radius, 1..=12).text("Max Scatter Radius")).changed();
        }
        EffectSpec::AddNoise {
            intensity,
            saturation,
            coverage,
        } => {
            ch |= ui.add(egui::Slider::new(intensity, 0..=100).text("Intensity")).changed();
            ch |= ui.add(egui::Slider::new(saturation, 0..=200).text("Color Saturation")).changed();
            ch |= ui.add(egui::Slider::new(coverage, 0.0..=100.0).text("Coverage").suffix("%")).changed();
        }
        EffectSpec::EdgeDetect { angle }
        | EffectSpec::Emboss { angle }
        | EffectSpec::Relief { angle } => {
            ch |= ui.add(egui::Slider::new(angle, 0.0..=360.0).text("Angle").suffix("°")).changed();
        }
        EffectSpec::OilPainting {
            brush_size,
            coarseness,
        } => {
            ch |= ui.add(egui::Slider::new(brush_size, 1..=5).text("Brush Size")).changed();
            ch |= ui.add(egui::Slider::new(coarseness, 4..=64).text("Coarseness")).changed();
        }
        EffectSpec::PencilSketch { pencil_tip_size } => {
            ch |= ui
                .add(egui::Slider::new(pencil_tip_size, 1..=20).text("Pencil Tip Size"))
                .changed();
        }
        EffectSpec::Clouds { scale, power } => {
            ch |= ui.add(egui::Slider::new(scale, 10..=500).text("Scale")).changed();
            ch |= ui.add(egui::Slider::new(power, 0..=100).text("Roughness")).changed();
        }
        EffectSpec::MandelbrotFractal { zoom } => {
            ch |= ui.add(egui::Slider::new(zoom, 1.0..=100.0).text("Zoom")).changed();
        }
        EffectSpec::JuliaFractal { zoom } => {
            ch |= ui.add(egui::Slider::new(zoom, 0.5..=20.0).text("Zoom")).changed();
        }
        _ => {}
    }
    ch
}

fn paint_checkerboard(painter: &egui::Painter, clip: Rect, origin: Pos2, tile: f32) {
    if !clip.is_positive() {
        return;
    }
    let light = Color32::from_gray(238);
    let dark = Color32::from_gray(204);
    painter.rect_filled(clip, 0.0, light);

    let x0 = ((clip.left() - origin.x) / tile).floor() as i32;
    let x1 = ((clip.right() - origin.x) / tile).ceil() as i32;
    let y0 = ((clip.top() - origin.y) / tile).floor() as i32;
    let y1 = ((clip.bottom() - origin.y) / tile).ceil() as i32;

    for gy in y0..y1 {
        for gx in x0..x1 {
            if (gx + gy).rem_euclid(2) == 1 {
                let r = Rect::from_min_size(
                    Pos2::new(origin.x + gx as f32 * tile, origin.y + gy as f32 * tile),
                    Vec2::new(tile, tile),
                )
                .intersect(clip);
                if r.is_positive() {
                    painter.rect_filled(r, 0.0, dark);
                }
            }
        }
    }
}

fn paint_marching_ants_rect(painter: &egui::Painter, r: Rect, phase: f32) {
    let seg = 5.0;
    let off = phase % (seg * 2.0);
    // Top & bottom
    let mut x = r.left() - off;
    let mut idx = 0;
    while x < r.right() {
        let a = x.max(r.left());
        let b = (x + seg).min(r.right());
        if b > a {
            let c = if idx % 2 == 0 { Color32::BLACK } else { Color32::WHITE };
            painter.line_segment([Pos2::new(a, r.top()), Pos2::new(b, r.top())], Stroke::new(1.2_f32, c));
            painter.line_segment([Pos2::new(a, r.bottom()), Pos2::new(b, r.bottom())], Stroke::new(1.2_f32, c));
        }
        x += seg;
        idx += 1;
    }
    // Left & right
    let mut y = r.top() - off;
    idx = 0;
    while y < r.bottom() {
        let a = y.max(r.top());
        let b = (y + seg).min(r.bottom());
        if b > a {
            let c = if idx % 2 == 0 { Color32::BLACK } else { Color32::WHITE };
            painter.line_segment([Pos2::new(r.left(), a), Pos2::new(r.left(), b)], Stroke::new(1.2_f32, c));
            painter.line_segment([Pos2::new(r.right(), a), Pos2::new(r.right(), b)], Stroke::new(1.2_f32, c));
        }
        y += seg;
        idx += 1;
    }
}

fn paint_rulers(
    painter: &egui::Painter,
    vp_rect: Rect,
    canvas_rect: Rect,
    zoom: f32,
    dark_mode: bool,
) {
    let r_sz = 20.0;
    let bg = if dark_mode {
        Color32::from_rgb(44, 46, 52)
    } else {
        Color32::from_rgb(236, 238, 242)
    };
    let fg = if dark_mode {
        Color32::from_gray(200)
    } else {
        Color32::from_gray(60)
    };
    let top_bar = Rect::from_min_max(vp_rect.min, Pos2::new(vp_rect.right(), vp_rect.top() + r_sz));
    let left_bar = Rect::from_min_max(vp_rect.min, Pos2::new(vp_rect.left() + r_sz, vp_rect.bottom()));
    painter.rect_filled(top_bar, 0.0, bg);
    painter.rect_filled(left_bar, 0.0, bg);

    let step_px = if zoom >= 2.0 {
        25
    } else if zoom >= 0.75 {
        50
    } else {
        100
    };
    let start_x = (((vp_rect.left() + r_sz - canvas_rect.left()) / zoom) / step_px as f32).floor() as i32 * step_px;
    let end_x = (((vp_rect.right() - canvas_rect.left()) / zoom) / step_px as f32).ceil() as i32 * step_px;
    let mut px = start_x;
    while px <= end_x {
        let sx = canvas_rect.left() + px as f32 * zoom;
        if sx >= vp_rect.left() + r_sz && sx <= vp_rect.right() {
            painter.line_segment(
                [Pos2::new(sx, vp_rect.top() + 12.0), Pos2::new(sx, vp_rect.top() + r_sz)],
                Stroke::new(1.0_f32, fg),
            );
            painter.text(
                Pos2::new(sx + 2.0, vp_rect.top() + 2.0),
                Align2::LEFT_TOP,
                format!("{px}"),
                FontId::proportional(9.5),
                fg,
            );
        }
        px += step_px;
    }

    let start_y = (((vp_rect.top() + r_sz - canvas_rect.top()) / zoom) / step_px as f32).floor() as i32 * step_px;
    let end_y = (((vp_rect.bottom() - canvas_rect.top()) / zoom) / step_px as f32).ceil() as i32 * step_px;
    let mut py = start_y;
    while py <= end_y {
        let sy = canvas_rect.top() + py as f32 * zoom;
        if sy >= vp_rect.top() + r_sz && sy <= vp_rect.bottom() {
            painter.line_segment(
                [Pos2::new(vp_rect.left() + 12.0, sy), Pos2::new(vp_rect.left() + r_sz, sy)],
                Stroke::new(1.0_f32, fg),
            );
            painter.text(
                Pos2::new(vp_rect.left() + 1.0, sy + 2.0),
                Align2::LEFT_TOP,
                format!("{py}"),
                FontId::proportional(8.5),
                fg,
            );
        }
        py += step_px;
    }

    painter.rect_filled(
        Rect::from_min_size(vp_rect.min, Vec2::new(r_sz, r_sz)),
        0.0,
        bg,
    );
}

fn install_system_fallback_fonts(ctx: &egui::Context) {
    #[cfg(target_os = "windows")]
    {
        let candidates = [
            "C:\\Windows\\Fonts\\malgun.ttf",
            "C:\\Windows\\Fonts\\segoeui.ttf",
        ];
        for path in candidates {
            if let Ok(bytes) = std::fs::read(path) {
                let mut fonts = egui::FontDefinitions::default();
                fonts.font_data.insert(
                    "sys_fallback".to_owned(),
                    egui::FontData::from_owned(bytes).into(),
                );
                if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    fam.push("sys_fallback".to_owned());
                }
                if let Some(fam) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    fam.push("sys_fallback".to_owned());
                }
                ctx.set_fonts(fonts);
                break;
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    let _ = ctx;
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_cli_args() -> (Option<PathBuf>, Option<PathBuf>) {
    let mut args = std::env::args().skip(1);
    let mut screenshot = None;
    let mut open_file = None;
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            if let Some(p) = args.next() {
                screenshot = Some(PathBuf::from(p));
            }
        } else if let Some(rest) = arg.strip_prefix("--screenshot=") {
            screenshot = Some(PathBuf::from(rest));
        } else if !arg.starts_with('-') && open_file.is_none() {
            open_file = Some(PathBuf::from(arg));
        }
    }
    (screenshot, open_file)
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let (screenshot_path, open_file) = parse_cli_args();
    let is_screenshot = screenshot_path.is_some();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Canvas Studio")
        .with_inner_size([WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_HEIGHT])
        .with_min_inner_size([720.0, 480.0]);

    if is_screenshot {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }

    #[allow(unused_mut)]
    let mut native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    #[cfg(target_os = "macos")]
    if is_screenshot {
        use winit::platform::macos::EventLoopBuilderExtMacOS;
        native_options.event_loop_builder = Some(Box::new(|builder| {
            builder.with_activate_ignoring_other_apps(false);
        }));
    }

    eframe::run_native(
        "Canvas Studio",
        native_options,
        Box::new(move |cc| Ok(Box::new(CanvasStudioApp::new(cc, screenshot_path, open_file)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");

        let canvas = document
            .get_element_by_id("canvas_studio_canvas")
            .expect("Failed to find canvas_studio_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("canvas_studio_canvas is not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(CanvasStudioApp::new(cc, None, None)))),
            )
            .await;

        if let Some(loading) = document.get_element_by_id("loading_overlay") {
            match start_result {
                Ok(_) => loading.remove(),
                Err(e) => {
                    loading.set_inner_html(&format!(
                        "<p style='color:#f87171;font-family:sans-serif;text-align:center'>Failed to start Canvas Studio:<br>{e:?}</p>"
                    ));
                }
            }
        }
    });
}
