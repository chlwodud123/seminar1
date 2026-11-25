#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod physics;
mod renderer;

#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use eframe::egui::{self, Align2, Color32, FontId, Key, Pos2, Rect, Rounding, Stroke, Vec2};

use ai::{BotController, BotDifficulty};
use physics::{
    GameSpeed, MatchWorld, PlayerInput, Ruleset, Side, BASE_TICK_HZ, FIELD_HEIGHT, FIELD_WIDTH,
};
use renderer::{draw_court, CourtTheme, VisualOptions};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Screen {
    MainMenu,
    Options,
    Playing,
    Paused,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PlayMode {
    VsAi,
    Local2P,
    DemoAi,
}

struct VolleyApp {
    screen: Screen,
    mode: PlayMode,
    world: MatchWorld,
    left_bot: BotController,
    right_bot: BotController,
    difficulty: BotDifficulty,
    ruleset: Ruleset,
    score_to_win: u32,
    speed: GameSpeed,
    visuals: VisualOptions,

    // Fixed-timestep simulation timing
    last_time: Option<f64>,
    accumulator: f64,
    fps_smooth: f32,

    // Automated --screenshot <file.png> support (native only)
    #[cfg(not(target_arch = "wasm32"))]
    screenshot_path: Option<PathBuf>,
    #[cfg(not(target_arch = "wasm32"))]
    start_time: Instant,
    #[cfg(not(target_arch = "wasm32"))]
    frame_count: u32,
    #[cfg(not(target_arch = "wasm32"))]
    screenshot_requested_at: Option<Instant>,
    #[cfg(not(target_arch = "wasm32"))]
    screenshot_saved: bool,
}

impl VolleyApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        #[cfg(not(target_arch = "wasm32"))] screenshot_path: Option<PathBuf>,
    ) -> Self {
        let _ = cc;
        let ruleset = Ruleset::Default;
        let score_to_win = 15;
        let mut world = MatchWorld::new(ruleset, score_to_win);
        let mut left_bot = BotController::new(0xBEEF_1234);
        let mut right_bot = BotController::new(0xCAFE_5678);

        // Pre-warm demo match by ~95 ticks so the court is immediately lively on frame 1!
        for _ in 0..95 {
            let l_in = left_bot.decide(&world, Side::Left, BotDifficulty::Medium);
            let r_in = right_bot.decide(&world, Side::Right, BotDifficulty::Medium);
            world.step(l_in, r_in);
        }

        // In --screenshot mode, drive repaints from a background thread so an unfocused /
        // AlwaysOnBottom window on Windows never stalls waiting for WM_PAINT.
        #[cfg(not(target_arch = "wasm32"))]
        if screenshot_path.is_some() {
            let ctx = cc.egui_ctx.clone();
            std::thread::spawn(move || loop {
                ctx.request_repaint();
                std::thread::sleep(std::time::Duration::from_millis(8));
            });
        }

        Self {
            screen: Screen::MainMenu,
            mode: PlayMode::DemoAi,
            world,
            left_bot,
            right_bot,
            difficulty: BotDifficulty::Medium,
            ruleset,
            score_to_win,
            speed: GameSpeed::Default,
            visuals: VisualOptions::default(),
            last_time: None,
            accumulator: 0.0,
            fps_smooth: 60.0,
            #[cfg(not(target_arch = "wasm32"))]
            screenshot_path,
            #[cfg(not(target_arch = "wasm32"))]
            start_time: Instant::now(),
            #[cfg(not(target_arch = "wasm32"))]
            frame_count: 0,
            #[cfg(not(target_arch = "wasm32"))]
            screenshot_requested_at: None,
            #[cfg(not(target_arch = "wasm32"))]
            screenshot_saved: false,
        }
    }

    fn start_match(&mut self, mode: PlayMode) {
        self.mode = mode;
        self.world.reset_match(self.ruleset, self.score_to_win);
        self.screen = Screen::Playing;
    }

    fn return_to_menu(&mut self) {
        self.mode = PlayMode::DemoAi;
        self.world.reset_match(self.ruleset, self.score_to_win);
        self.screen = Screen::MainMenu;
    }

    fn step_simulation(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        let dt = match self.last_time {
            Some(prev) => (now - prev).clamp(0.0, 0.08),
            None => 1.0 / 60.0,
        };
        self.last_time = Some(now);

        if dt > 0.0001 {
            let inst_fps = (1.0 / dt) as f32;
            self.fps_smooth = self.fps_smooth * 0.92 + inst_fps * 0.08;
        }

        if self.screen == Screen::Paused {
            return;
        }

        // In screenshot mode use deterministic 1/60s step so ~120 frames animate crisply
        #[cfg(not(target_arch = "wasm32"))]
        let effective_dt = if self.screenshot_path.is_some() {
            1.0 / 60.0
        } else {
            dt
        };
        #[cfg(target_arch = "wasm32")]
        let effective_dt = dt;

        let tick_dt = 1.0 / (BASE_TICK_HZ * self.speed.multiplier());
        self.accumulator = (self.accumulator + effective_dt).min(tick_dt * 6.0);

        let (human_left, human_right) = read_keyboard_inputs(ctx, self.mode);

        while self.accumulator >= tick_dt {
            self.accumulator -= tick_dt;

            let (l_in, r_in) = match self.screen {
                Screen::MainMenu | Screen::Options => {
                    // Auto-restart background demo if either bot wins
                    if self.world.winner.is_some() {
                        self.world.reset_match(self.ruleset, self.score_to_win);
                    }
                    (
                        self.left_bot.decide(&self.world, Side::Left, BotDifficulty::Medium),
                        self.right_bot.decide(&self.world, Side::Right, BotDifficulty::Medium),
                    )
                }
                Screen::Playing => match self.mode {
                    PlayMode::VsAi => (
                        human_left,
                        self.right_bot.decide(&self.world, Side::Right, self.difficulty),
                    ),
                    PlayMode::Local2P => (human_left, human_right),
                    PlayMode::DemoAi => (
                        self.left_bot.decide(&self.world, Side::Left, self.difficulty),
                        self.right_bot.decide(&self.world, Side::Right, self.difficulty),
                    ),
                },
                Screen::Paused => break,
            };

            self.world.step(l_in, r_in);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn handle_screenshot_lifecycle(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot_path.clone() else {
            return;
        };

        // Check for completed screenshot event
        let captured = ctx.input(|i| {
            i.events.iter().find_map(|ev| {
                if let egui::Event::Screenshot { image, .. } = ev {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });

        if let Some(color_img) = captured {
            if !self.screenshot_saved {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
                let w = color_img.size[0] as u32;
                let h = color_img.size[1] as u32;
                let mut raw = Vec::with_capacity((w * h * 4) as usize);
                for px in &color_img.pixels {
                    raw.extend_from_slice(&px.to_array());
                }
                if let Err(err) =
                    image::save_buffer(&path, &raw, w, h, image::ColorType::Rgba8)
                {
                    eprintln!("Failed to save screenshot to {:?}: {}", path, err);
                    std::process::exit(1);
                }
                self.screenshot_saved = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                std::process::exit(0);
            }
        }

        self.frame_count += 1;
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let ready_to_capture = self.frame_count >= 120 || (self.frame_count >= 5 && elapsed >= 5.0);

        if ready_to_capture && !self.screenshot_saved {
            let should_request = match self.screenshot_requested_at {
                None => true,
                Some(t) => t.elapsed().as_secs_f32() >= 1.0,
            };
            if should_request {
                self.screenshot_requested_at = Some(Instant::now());
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            }
        }

        if elapsed >= 25.0 && !self.screenshot_saved {
            eprintln!("Timed out waiting for screenshot event after {:.1}s", elapsed);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            std::process::exit(1);
        }
    }
}

fn read_keyboard_inputs(ctx: &egui::Context, mode: PlayMode) -> (PlayerInput, PlayerInput) {
    ctx.input(|i| {
        let a = i.key_down(Key::A);
        let d = i.key_down(Key::D);
        let w = i.key_down(Key::W);
        let left = i.key_down(Key::ArrowLeft);
        let right = i.key_down(Key::ArrowRight);
        let up = i.key_down(Key::ArrowUp);
        let space = i.key_down(Key::Space);

        match mode {
            PlayMode::VsAi => {
                // In 1P mode, both A/D/W and Arrow keys control the left player for convenience
                let l = PlayerInput {
                    left: a || left,
                    right: d || right,
                    jump: w || up || space,
                };
                (l, PlayerInput::default())
            }
            PlayMode::Local2P => (
                PlayerInput {
                    left: a,
                    right: d,
                    jump: w,
                },
                PlayerInput {
                    left,
                    right,
                    jump: up,
                },
            ),
            PlayMode::DemoAi => (PlayerInput::default(), PlayerInput::default()),
        }
    })
}

impl eframe::App for VolleyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keep animating smoothly (essential both for arcade 60+ FPS and unfocused --screenshot)
        ctx.request_repaint();

        // Pause / Back hotkey handling
        let esc_pressed = ctx.input(|i| i.key_pressed(Key::Escape));
        let p_pressed = ctx.input(|i| i.key_pressed(Key::P));
        if esc_pressed {
            match self.screen {
                Screen::Playing => self.screen = Screen::Paused,
                Screen::Paused => self.screen = Screen::Playing,
                Screen::Options => self.screen = Screen::MainMenu,
                Screen::MainMenu => {}
            }
        } else if p_pressed && self.screen == Screen::Playing {
            self.screen = Screen::Paused;
        } else if p_pressed && self.screen == Screen::Paused {
            self.screen = Screen::Playing;
        }

        // Advance game physics
        self.step_simulation(ctx);

        // Full-window canvas panel
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(14, 22, 38)))
            .show(ctx, |ui| {
                let avail = ui.available_rect_before_wrap();
                let show_hud = matches!(self.screen, Screen::Playing | Screen::Paused);
                let tf = draw_court(
                    ui.painter(),
                    avail,
                    &self.world,
                    &self.visuals,
                    self.fps_smooth,
                    show_hud,
                );

                match self.screen {
                    Screen::MainMenu => self.draw_main_menu(ui, &tf),
                    Screen::Options => self.draw_options(ui, &tf),
                    Screen::Playing => {
                        if let Some(winner) = self.world.winner {
                            self.draw_victory_overlay(ui, &tf, winner);
                        } else {
                            self.draw_playing_controls_hint(ui, &tf);
                        }
                    }
                    Screen::Paused => self.draw_pause_overlay(ui, &tf),
                }
            });

        #[cfg(not(target_arch = "wasm32"))]
        self.handle_screenshot_lifecycle(ctx);
    }
}

impl VolleyApp {
    fn draw_main_menu(&mut self, ui: &mut egui::Ui, tf: &renderer::CourtTransform) {
        let painter = ui.painter().with_clip_rect(tf.rect);

        // Compact translucent menu card in upper-left so both blobs & live rally stay clearly visible
        let panel_rect = Rect::from_min_max(tf.pt(22.0, 22.0), tf.pt(304.0, 408.0));
        painter.rect_filled(
            panel_rect,
            Rounding::same(tf.s(16.0)),
            Color32::from_rgba_unmultiplied(12, 24, 52, 196),
        );
        painter.rect_stroke(
            panel_rect,
            Rounding::same(tf.s(16.0)),
            Stroke::new(tf.s(2.0), Color32::from_rgba_unmultiplied(140, 200, 255, 135)),
        );

        // Stylized neutral vector title ("BEACH BLOB VOLLEY")
        let title_pos = tf.pt(163.0, 60.0);
        painter.text(
            Pos2::new(title_pos.x + 2.0, title_pos.y + 2.0),
            Align2::CENTER_CENTER,
            "BEACH BLOB",
            FontId::proportional(tf.s(30.0)),
            Color32::from_rgba_unmultiplied(0, 0, 0, 185),
        );
        painter.text(
            title_pos,
            Align2::CENTER_CENTER,
            "BEACH BLOB",
            FontId::proportional(tf.s(30.0)),
            Color32::from_rgb(122, 242, 108),
        );

        let sub_pos = tf.pt(163.0, 94.0);
        painter.text(
            Pos2::new(sub_pos.x + 2.0, sub_pos.y + 2.0),
            Align2::CENTER_CENTER,
            "VOLLEY",
            FontId::proportional(tf.s(26.0)),
            Color32::from_rgba_unmultiplied(0, 0, 0, 185),
        );
        painter.text(
            sub_pos,
            Align2::CENTER_CENTER,
            "VOLLEY",
            FontId::proportional(tf.s(26.0)),
            Color32::from_rgb(255, 224, 92),
        );

        // Interactive menu buttons
        let menu_inner = Rect::from_min_max(tf.pt(42.0, 122.0), tf.pt(284.0, 396.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(menu_inner));
        child.spacing_mut().item_spacing = Vec2::new(0.0, tf.s(9.0));

        if arcade_button(&mut child, "Play vs AI (1P)", tf.s(38.0), Color32::from_rgb(38, 128, 235), tf.s(16.5)) {
            self.start_match(PlayMode::VsAi);
        }
        if arcade_button(&mut child, "2-Player Local", tf.s(38.0), Color32::from_rgb(218, 68, 72), tf.s(16.5)) {
            self.start_match(PlayMode::Local2P);
        }
        if arcade_button(&mut child, "Watch AI Demo", tf.s(34.0), Color32::from_rgb(44, 152, 116), tf.s(14.5)) {
            self.start_match(PlayMode::DemoAi);
        }
        if arcade_button(&mut child, "Options", tf.s(34.0), Color32::from_rgb(70, 88, 126), tf.s(14.5)) {
            self.screen = Screen::Options;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if arcade_button(&mut child, "Exit", tf.s(32.0), Color32::from_rgb(56, 62, 78), tf.s(14.0)) {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        }

        child.add_space(tf.s(4.0));
        child.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "{}  |  {} pts  |  AI: {}",
                    self.ruleset.label(),
                    self.score_to_win,
                    self.difficulty.label()
                ))
                .size(tf.s(10.5))
                .color(Color32::from_rgb(186, 210, 240)),
            );
        });

        // Subtle control legend pill at bottom-left of beach
        let hint_rect = Rect::from_min_max(tf.pt(18.0, 564.0), tf.pt(368.0, 590.0));
        painter.rect_filled(
            hint_rect,
            Rounding::same(tf.s(8.0)),
            Color32::from_rgba_unmultiplied(12, 24, 48, 150),
        );
        painter.text(
            hint_rect.center(),
            Align2::CENTER_CENTER,
            "P1: A / D / W   |   P2: Left / Right / Up",
            FontId::proportional(tf.s(12.5)),
            Color32::from_rgb(230, 240, 255),
        );
    }

    fn draw_options(&mut self, ui: &mut egui::Ui, tf: &renderer::CourtTransform) {
        let painter = ui.painter().with_clip_rect(tf.rect);
        let box_rect = Rect::from_min_max(tf.pt(88.0, 42.0), tf.pt(712.0, 558.0));
        painter.rect_filled(
            box_rect,
            Rounding::same(tf.s(16.0)),
            Color32::from_rgba_unmultiplied(14, 24, 46, 228),
        );
        painter.rect_stroke(
            box_rect,
            Rounding::same(tf.s(16.0)),
            Stroke::new(tf.s(2.0), Color32::from_rgb(110, 178, 255)),
        );

        let inner = box_rect.shrink(tf.s(24.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        child.visuals_mut().override_text_color = Some(Color32::WHITE);

        child.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("OPTIONS")
                    .size(tf.s(26.0))
                    .strong()
                    .color(Color32::from_rgb(255, 226, 108)),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("Back").size(tf.s(15.0)))
                            .min_size(Vec2::new(tf.s(90.0), tf.s(32.0))),
                    )
                    .clicked()
                {
                    self.screen = Screen::MainMenu;
                }
            });
        });

        child.add_space(tf.s(8.0));
        child.separator();
        child.add_space(tf.s(8.0));

        egui::Grid::new("options_grid")
            .num_columns(2)
            .spacing(Vec2::new(tf.s(22.0), tf.s(12.0)))
            .show(&mut child, |ui| {
                ui.label(egui::RichText::new("AI Difficulty").size(tf.s(15.0)));
                ui.horizontal(|ui| {
                    for d in [BotDifficulty::Weak, BotDifficulty::Medium, BotDifficulty::Strong] {
                        ui.selectable_value(&mut self.difficulty, d, d.label());
                    }
                });
                ui.end_row();

                ui.label(egui::RichText::new("Ruleset").size(tf.s(15.0)));
                ui.horizontal(|ui| {
                    for r in [Ruleset::Default, Ruleset::Classic] {
                        ui.selectable_value(&mut self.ruleset, r, r.label());
                    }
                });
                ui.end_row();

                ui.label(egui::RichText::new("Score to Win").size(tf.s(15.0)));
                ui.horizontal(|ui| {
                    for s in [5u32, 11, 15, 21] {
                        ui.selectable_value(&mut self.score_to_win, s, format!("{}", s));
                    }
                });
                ui.end_row();

                ui.label(egui::RichText::new("Game Speed").size(tf.s(15.0)));
                ui.horizontal_wrapped(|ui| {
                    for sp in [
                        GameSpeed::VerySlow,
                        GameSpeed::Slow,
                        GameSpeed::Default,
                        GameSpeed::Fast,
                        GameSpeed::VeryFast,
                    ] {
                        ui.selectable_value(&mut self.speed, sp, sp.label());
                    }
                });
                ui.end_row();

                ui.label(egui::RichText::new("Court Theme").size(tf.s(15.0)));
                ui.horizontal(|ui| {
                    for th in [CourtTheme::SunnyBeach, CourtTheme::SunsetBeach] {
                        ui.selectable_value(&mut self.visuals.theme, th, th.label());
                    }
                });
                ui.end_row();

                ui.label(egui::RichText::new("Display").size(tf.s(15.0)));
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.visuals.show_shadow, "Show Shadows");
                    ui.checkbox(&mut self.visuals.show_fps, "Show FPS");
                });
                ui.end_row();

                ui.label(egui::RichText::new("Left Blob Color").size(tf.s(15.0)));
                ui.horizontal(|ui| {
                    color_swatches(
                        ui,
                        &mut self.visuals.left_color,
                        &[
                            [0, 80, 255],
                            [28, 180, 120],
                            [150, 75, 240],
                            [255, 165, 30],
                        ],
                    );
                    ui.checkbox(&mut self.visuals.left_oscillate, "Morph");
                });
                ui.end_row();

                ui.label(egui::RichText::new("Right Blob Color").size(tf.s(15.0)));
                ui.horizontal(|ui| {
                    color_swatches(
                        ui,
                        &mut self.visuals.right_color,
                        &[
                            [235, 35, 45],
                            [255, 130, 35],
                            [220, 60, 170],
                            [45, 175, 225],
                        ],
                    );
                    ui.checkbox(&mut self.visuals.right_oscillate, "Morph");
                });
                ui.end_row();
            });

        child.add_space(tf.s(18.0));
        child.label(
            egui::RichText::new("Controls: Left Player A / D / W   |   Right Player Left / Right / Up   |   Esc: Pause")
                .size(tf.s(13.0))
                .color(Color32::from_rgb(185, 210, 242)),
        );
    }

    fn draw_playing_controls_hint(&self, ui: &mut egui::Ui, tf: &renderer::CourtTransform) {
        let painter = ui.painter().with_clip_rect(tf.rect);
        let hint = match self.mode {
            PlayMode::VsAi => "Move: A/D or Arrows   Jump: W/Up/Space   Pause: Esc",
            PlayMode::Local2P => "Blue: A/D/W   |   Red: Left/Right/Up   |   Pause: Esc",
            PlayMode::DemoAi => "AI vs AI Demo   |   Press Esc for Menu",
        };
        painter.text(
            tf.pt(14.0, FIELD_HEIGHT - 12.0),
            Align2::LEFT_BOTTOM,
            hint,
            FontId::proportional(tf.s(13.0)),
            Color32::from_rgba_unmultiplied(255, 255, 255, 185),
        );
    }

    fn draw_pause_overlay(&mut self, ui: &mut egui::Ui, tf: &renderer::CourtTransform) {
        let painter = ui.painter().with_clip_rect(tf.rect);
        painter.rect_filled(tf.rect, 0.0, Color32::from_rgba_unmultiplied(8, 14, 28, 140));

        let card = Rect::from_center_size(tf.pt(FIELD_WIDTH * 0.5, FIELD_HEIGHT * 0.5), Vec2::new(tf.s(300.0), tf.s(250.0)));
        painter.rect_filled(card, Rounding::same(tf.s(16.0)), Color32::from_rgba_unmultiplied(18, 30, 56, 238));
        painter.rect_stroke(card, Rounding::same(tf.s(16.0)), Stroke::new(tf.s(2.0), Color32::from_rgb(130, 192, 255)));

        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(card.shrink(tf.s(24.0))));
        child.spacing_mut().item_spacing = Vec2::new(0.0, tf.s(12.0));
        child.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new("GAME PAUSED")
                    .size(tf.s(24.0))
                    .strong()
                    .color(Color32::WHITE),
            );
        });
        child.add_space(tf.s(6.0));
        if arcade_button(&mut child, "Continue", tf.s(40.0), Color32::from_rgb(44, 152, 116), tf.s(16.0)) {
            self.screen = Screen::Playing;
        }
        if arcade_button(&mut child, "Restart Match", tf.s(40.0), Color32::from_rgb(48, 122, 224), tf.s(16.0)) {
            self.world.reset_match(self.ruleset, self.score_to_win);
            self.screen = Screen::Playing;
        }
        if arcade_button(&mut child, "Main Menu", tf.s(40.0), Color32::from_rgb(166, 62, 72), tf.s(16.0)) {
            self.return_to_menu();
        }
    }

    fn draw_victory_overlay(&mut self, ui: &mut egui::Ui, tf: &renderer::CourtTransform, winner: Side) {
        let painter = ui.painter().with_clip_rect(tf.rect);
        painter.rect_filled(tf.rect, 0.0, Color32::from_rgba_unmultiplied(8, 14, 28, 155));

        let card = Rect::from_center_size(tf.pt(FIELD_WIDTH * 0.5, FIELD_HEIGHT * 0.5), Vec2::new(tf.s(360.0), tf.s(265.0)));
        painter.rect_filled(card, Rounding::same(tf.s(18.0)), Color32::from_rgba_unmultiplied(18, 30, 58, 240));
        painter.rect_stroke(card, Rounding::same(tf.s(18.0)), Stroke::new(tf.s(2.5), Color32::from_rgb(255, 215, 86)));

        let win_name = match winner {
            Side::Left => "BLUE PLAYER WINS!",
            Side::Right => "RED PLAYER WINS!",
        };

        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(card.shrink(tf.s(24.0))));
        child.spacing_mut().item_spacing = Vec2::new(0.0, tf.s(12.0));
        child.vertical_centered(|ui| {
            ui.label(
                egui::RichText::new(win_name)
                    .size(tf.s(26.0))
                    .strong()
                    .color(Color32::from_rgb(255, 226, 98)),
            );
            ui.label(
                egui::RichText::new(format!(
                    "Final Score:  {:02}  -  {:02}",
                    self.world.scores[0], self.world.scores[1]
                ))
                .size(tf.s(20.0))
                .color(Color32::WHITE),
            );
        });
        child.add_space(tf.s(10.0));
        if arcade_button(&mut child, "Play Again", tf.s(42.0), Color32::from_rgb(44, 152, 116), tf.s(17.0)) {
            self.world.reset_match(self.ruleset, self.score_to_win);
        }
        if arcade_button(&mut child, "Main Menu", tf.s(40.0), Color32::from_rgb(72, 92, 130), tf.s(16.0)) {
            self.return_to_menu();
        }
    }
}

fn arcade_button(ui: &mut egui::Ui, label: &str, height: f32, bg: Color32, font_size: f32) -> bool {
    let desired = Vec2::new(ui.available_width(), height);
    let (rect, resp) = ui.allocate_exact_size(desired, egui::Sense::click());
    let fill = if resp.is_pointer_button_down_on() {
        Color32::from_rgb(
            (bg.r() as f32 * 0.82) as u8,
            (bg.g() as f32 * 0.82) as u8,
            (bg.b() as f32 * 0.82) as u8,
        )
    } else if resp.hovered() {
        Color32::from_rgb(
            (bg.r() as u16 + 24).min(255) as u8,
            (bg.g() as u16 + 24).min(255) as u8,
            (bg.b() as u16 + 24).min(255) as u8,
        )
    } else {
        bg
    };
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(10.0), fill);
    painter.rect_stroke(
        rect,
        Rounding::same(10.0),
        Stroke::new(
            if resp.hovered() { 2.0_f32 } else { 1.4_f32 },
            Color32::from_rgba_unmultiplied(255, 255, 255, if resp.hovered() { 210 } else { 115 }),
        ),
    );
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(font_size),
        Color32::WHITE,
    );
    resp.clicked()
}

fn color_swatches(ui: &mut egui::Ui, current: &mut [u8; 3], swatches: &[[u8; 3]]) {
    for &sw in swatches {
        let selected = *current == sw;
        let col = Color32::from_rgb(sw[0], sw[1], sw[2]);
        let btn = egui::Button::new("")
            .fill(col)
            .stroke(Stroke::new(
                if selected { 2.5_f32 } else { 1.0_f32 },
                if selected { Color32::YELLOW } else { Color32::WHITE },
            ))
            .min_size(Vec2::new(26.0, 22.0));
        if ui.add(btn).clicked() {
            *current = sw;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_screenshot_arg() -> Option<PathBuf> {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            if let Some(path) = args.next() {
                return Some(PathBuf::from(path));
            }
        } else if let Some(rest) = arg.strip_prefix("--screenshot=") {
            if !rest.is_empty() {
                return Some(PathBuf::from(rest));
            }
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    let screenshot_path = parse_screenshot_arg();
    let is_screenshot = screenshot_path.is_some();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([FIELD_WIDTH, FIELD_HEIGHT])
        .with_min_inner_size([640.0, 480.0])
        .with_title("Beach Blob Volley");

    if is_screenshot {
        viewport = viewport.with_active(false);
        #[cfg(target_os = "windows")]
        {
            viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
        }
    }

    let options = eframe::NativeOptions {
        viewport,
        vsync: !is_screenshot,
        persist_window: !is_screenshot,
        #[cfg(target_os = "macos")]
        event_loop_builder: if is_screenshot {
            Some(Box::new(|builder| {
                use eframe::winit::platform::macos::EventLoopBuilderExtMacOS;
                builder.with_activate_ignoring_other_apps(false);
            }))
        } else {
            None
        },
        ..Default::default()
    };

    eframe::run_native(
        "Beach Blob Volley",
        options,
        Box::new(move |cc| Ok(Box::new(VolleyApp::new(cc, screenshot_path)))),
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
            .get_element_by_id("blobby_canvas")
            .expect("Failed to find blobby_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("blobby_canvas is not a HtmlCanvasElement");

        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(VolleyApp::new(cc)))),
            )
            .await;

        if let Some(loading) = document.get_element_by_id("loading_overlay") {
            match start_result {
                Ok(_) => loading.remove(),
                Err(e) => {
                    loading.set_inner_html(&format!(
                        "<p style='color:#f87171;font-family:sans-serif;text-align:center'>Failed to start Beach Blob Volley:<br>{e:?}</p>"
                    ));
                }
            }
        }
    });
}
