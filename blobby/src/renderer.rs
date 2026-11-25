use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Rounding, Shape, Stroke, Vec2};
use std::f32::consts::TAU;

use crate::physics::{
    Blob, MatchWorld, Side, BALL_RADIUS, BLOBBY_GROUND_Y, BLOBBY_LOWER_RADIUS, BLOBBY_LOWER_SPHERE,
    BLOBBY_UPPER_RADIUS, BLOBBY_UPPER_SPHERE, FIELD_HEIGHT, FIELD_WIDTH, GROUND_PLANE_Y,
    MAX_TOUCHES, NET_RADIUS, NET_TOP_Y, NET_X,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CourtTheme {
    SunnyBeach,
    SunsetBeach,
}

impl CourtTheme {
    pub fn label(self) -> &'static str {
        match self {
            CourtTheme::SunnyBeach => "Sunny Beach",
            CourtTheme::SunsetBeach => "Sunset Cove",
        }
    }
}

pub struct VisualOptions {
    pub left_color: [u8; 3],
    pub right_color: [u8; 3],
    pub left_oscillate: bool,
    pub right_oscillate: bool,
    pub show_shadow: bool,
    pub show_fps: bool,
    pub theme: CourtTheme,
}

impl Default for VisualOptions {
    fn default() -> Self {
        Self {
            left_color: [0, 80, 255],
            right_color: [235, 35, 45],
            left_oscillate: false,
            right_oscillate: false,
            show_shadow: true,
            show_fps: true,
            theme: CourtTheme::SunnyBeach,
        }
    }
}

pub struct CourtTransform {
    pub rect: Rect,
    pub scale: f32,
}

impl CourtTransform {
    pub fn from_available(avail: Rect) -> Self {
        let sx = avail.width() / FIELD_WIDTH;
        let sy = avail.height() / FIELD_HEIGHT;
        let scale = sx.min(sy).max(0.1);
        let w = FIELD_WIDTH * scale;
        let h = FIELD_HEIGHT * scale;
        let min = Pos2::new(
            avail.center().x - w * 0.5,
            avail.center().y - h * 0.5,
        );
        Self {
            rect: Rect::from_min_size(min, Vec2::new(w, h)),
            scale,
        }
    }

    pub fn pt(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.rect.min.x + x * self.scale, self.rect.min.y + y * self.scale)
    }

    pub fn s(&self, v: f32) -> f32 {
        v * self.scale
    }
}

pub fn draw_court(
    painter: &Painter,
    avail: Rect,
    world: &MatchWorld,
    opts: &VisualOptions,
    fps: f32,
    show_hud: bool,
) -> CourtTransform {
    // Dark letterbox background
    painter.rect_filled(avail, 0.0, Color32::from_rgb(14, 22, 38));

    let tf = CourtTransform::from_available(avail);
    let cp = painter.with_clip_rect(tf.rect);

    draw_background(&cp, &tf, opts.theme, world.tick_count);
    draw_net_shadow(&cp, &tf, opts.show_shadow);

    if opts.show_shadow {
        draw_blob_shadow(&cp, &tf, &world.blobs[0]);
        draw_blob_shadow(&cp, &tf, &world.blobs[1]);
        draw_ball_shadow(&cp, &tf, world.ball.x, world.ball.y);
    }

    let left_col = effective_blob_color(opts.left_color, opts.left_oscillate, world.tick_count, 0.0);
    let right_col =
        effective_blob_color(opts.right_color, opts.right_oscillate, world.tick_count, 2.2);

    draw_blob(&cp, &tf, &world.blobs[0], left_col, world.ball.x, world.ball.y);
    draw_blob(&cp, &tf, &world.blobs[1], right_col, world.ball.x, world.ball.y);

    draw_net(&cp, &tf);
    draw_ball(&cp, &tf, world.ball.x, world.ball.y, world.ball.rotation);

    if show_hud {
        draw_hud(&cp, &tf, world, opts, left_col, right_col, fps);
    }

    tf
}

fn draw_background(painter: &Painter, tf: &CourtTransform, theme: CourtTheme, tick: u64) {
    let t = tick as f32 * 0.008;

    // 1. Sky vertical gradient bands
    let (sky_top, sky_mid, sky_hor, ocean_top, ocean_bot, sand_top, sand_bot) = match theme {
        CourtTheme::SunnyBeach => (
            Color32::from_rgb(18, 86, 214),
            Color32::from_rgb(52, 134, 240),
            Color32::from_rgb(132, 194, 250),
            Color32::from_rgb(24, 96, 212),
            Color32::from_rgb(48, 134, 232),
            Color32::from_rgb(206, 194, 118),
            Color32::from_rgb(172, 158, 86),
        ),
        CourtTheme::SunsetBeach => (
            Color32::from_rgb(56, 36, 108),
            Color32::from_rgb(186, 82, 104),
            Color32::from_rgb(252, 178, 96),
            Color32::from_rgb(82, 54, 120),
            Color32::from_rgb(164, 92, 108),
            Color32::from_rgb(202, 152, 104),
            Color32::from_rgb(154, 112, 74),
        ),
    };

    let horizon_y = 286.0;
    let shore_y = 334.0;

    // Sky gradient (48 smooth bands)
    let bands = 48;
    for i in 0..bands {
        let f0 = i as f32 / bands as f32;
        let f1 = (i + 1) as f32 / bands as f32;
        let col = if f0 < 0.55 {
            lerp_color(sky_top, sky_mid, f0 / 0.55)
        } else {
            lerp_color(sky_mid, sky_hor, (f0 - 0.55) / 0.45)
        };
        let r = Rect::from_min_max(tf.pt(0.0, horizon_y * f0), tf.pt(FIELD_WIDTH, horizon_y * f1 + 1.0));
        painter.rect_filled(r, 0.0, col);
    }

    // Sun glow
    let sun_pos = match theme {
        CourtTheme::SunnyBeach => tf.pt(708.0, 74.0),
        CourtTheme::SunsetBeach => tf.pt(400.0, 236.0),
    };
    let sun_col = match theme {
        CourtTheme::SunnyBeach => Color32::from_rgba_unmultiplied(255, 252, 220, 42),
        CourtTheme::SunsetBeach => Color32::from_rgba_unmultiplied(255, 210, 120, 72),
    };
    painter.circle_filled(sun_pos, tf.s(54.0), sun_col);
    painter.circle_filled(sun_pos, tf.s(30.0), sun_col);
    painter.circle_filled(
        sun_pos,
        tf.s(18.0),
        Color32::from_rgba_unmultiplied(255, 254, 235, 165),
    );

    // Soft drifting clouds (spaced cleanly away from sun)
    let cloud_col = match theme {
        CourtTheme::SunnyBeach => Color32::from_rgba_unmultiplied(255, 255, 255, 95),
        CourtTheme::SunsetBeach => Color32::from_rgba_unmultiplied(255, 215, 180, 85),
    };
    for (cx, cy, cw) in [
        ((330.0 + t * 4.0) % 920.0 - 60.0, 72.0, 0.95),
        ((515.0 + t * 3.0) % 920.0 - 60.0, 118.0, 0.78),
    ] {
        painter.circle_filled(tf.pt(cx, cy), tf.s(22.0 * cw), cloud_col);
        painter.circle_filled(tf.pt(cx + 22.0 * cw, cy - 6.0), tf.s(26.0 * cw), cloud_col);
        painter.circle_filled(tf.pt(cx + 46.0 * cw, cy + 2.0), tf.s(19.0 * cw), cloud_col);
    }

    // Distant rocky cliff silhouette on the left (inspired by strand2 layout)
    let cliff_col = match theme {
        CourtTheme::SunnyBeach => Color32::from_rgb(92, 84, 76),
        CourtTheme::SunsetBeach => Color32::from_rgb(68, 46, 62),
    };
    let cliff_pts = vec![
        tf.pt(0.0, 294.0),
        tf.pt(0.0, 192.0),
        tf.pt(68.0, 198.0),
        tf.pt(142.0, 206.0),
        tf.pt(194.0, 214.0),
        tf.pt(205.0, 294.0),
    ];
    painter.add(Shape::convex_polygon(cliff_pts, cliff_col, Stroke::NONE));

    let sea_stack = vec![
        tf.pt(225.0, 294.0),
        tf.pt(242.0, 260.0),
        tf.pt(254.0, 264.0),
        tf.pt(262.0, 294.0),
    ];
    painter.add(Shape::convex_polygon(sea_stack, cliff_col, Stroke::NONE));

    // 2. Ocean water strip
    for i in 0..8 {
        let f0 = i as f32 / 8.0;
        let f1 = (i + 1) as f32 / 8.0;
        let y0 = horizon_y + (shore_y - horizon_y) * f0;
        let y1 = horizon_y + (shore_y - horizon_y) * f1 + 1.0;
        painter.rect_filled(
            Rect::from_min_max(tf.pt(0.0, y0), tf.pt(FIELD_WIDTH, y1)),
            0.0,
            lerp_color(ocean_top, ocean_bot, f0),
        );
    }

    // Animated ocean glints
    let glint_col = Color32::from_rgba_unmultiplied(235, 248, 255, 110);
    for i in 0..14 {
        let gx = ((i as f32 * 63.0 + (t * 14.0 + i as f32).sin() * 12.0) % 780.0) + 10.0;
        let gy = horizon_y + 8.0 + ((i * 19) % 34) as f32;
        painter.line_segment(
            [tf.pt(gx, gy), tf.pt(gx + 16.0, gy)],
            Stroke::new(tf.s(1.6), glint_col),
        );
    }

    // 3. Sandy beach court (from shore_y to bottom 600)
    let sand_bands = 16;
    for i in 0..sand_bands {
        let f0 = i as f32 / sand_bands as f32;
        let f1 = (i + 1) as f32 / sand_bands as f32;
        let y0 = shore_y + (FIELD_HEIGHT - shore_y) * f0;
        let y1 = shore_y + (FIELD_HEIGHT - shore_y) * f1 + 1.0;
        painter.rect_filled(
            Rect::from_min_max(tf.pt(0.0, y0), tf.pt(FIELD_WIDTH, y1)),
            0.0,
            lerp_color(sand_top, sand_bot, f0),
        );
    }

    // Gentle shoreline wave curve
    let foam_col = Color32::from_rgba_unmultiplied(240, 248, 255, 140);
    let mut wave_pts = Vec::with_capacity(24);
    for i in 0..=24 {
        let wx = i as f32 * (FIELD_WIDTH / 24.0);
        let wy = shore_y + (wx * 0.015 + t * 2.0).sin() * 3.0;
        wave_pts.push(tf.pt(wx, wy));
    }
    painter.add(Shape::line(wave_pts, Stroke::new(tf.s(2.4), foam_col)));

    // Subtle ground court horizon line at y=500 (physics ground plane visual cue)
    painter.line_segment(
        [tf.pt(18.0, GROUND_PLANE_Y + 4.0), tf.pt(FIELD_WIDTH - 18.0, GROUND_PLANE_Y + 4.0)],
        Stroke::new(tf.s(1.5), Color32::from_rgba_unmultiplied(255, 250, 225, 70)),
    );

    // 4. Stylized palm trees on left and right background edges
    draw_palm_tree(painter, tf, 62.0, 380.0, -18.0, 1.05, t);
    draw_palm_tree(painter, tf, 148.0, 354.0, 14.0, 0.85, t + 1.2);
    draw_palm_tree(painter, tf, 610.0, 344.0, -10.0, 0.85, t + 0.6);
    draw_palm_tree(painter, tf, 692.0, 352.0, 12.0, 0.95, t + 2.1);
}

fn draw_palm_tree(
    painter: &Painter,
    tf: &CourtTransform,
    base_x: f32,
    base_y: f32,
    lean_x: f32,
    scale: f32,
    t: f32,
) {
    let trunk_h = 165.0 * scale;
    let sway = (t * 1.4).sin() * 4.5 * scale;
    let top_x = base_x + lean_x * scale + sway;
    let top_y = base_y - trunk_h;

    // Ground shadow of palm (flat perspective ellipse)
    draw_ellipse(
        painter,
        tf.pt(base_x + 22.0 * scale, base_y + 4.0),
        tf.s(34.0 * scale),
        tf.s(7.5 * scale),
        Color32::from_rgba_unmultiplied(45, 40, 20, 48),
    );

    // Curved trunk using segmented polyline
    let segs = 7;
    let trunk_col = Color32::from_rgb(76, 52, 28);
    for i in 0..segs {
        let a = i as f32 / segs as f32;
        let b = (i + 1) as f32 / segs as f32;
        let p0 = tf.pt(
            base_x + (top_x - base_x) * a * a,
            base_y + (top_y - base_y) * a,
        );
        let p1 = tf.pt(
            base_x + (top_x - base_x) * b * b,
            base_y + (top_y - base_y) * b,
        );
        let thickness = (11.0 - a * 5.0) * scale;
        painter.line_segment([p0, p1], Stroke::new(tf.s(thickness), trunk_col));
    }

    // Palm fronds
    let frond_dark = Color32::from_rgb(16, 108, 28);
    let frond_light = Color32::from_rgb(38, 154, 42);
    for (idx, angle_deg) in [-155.0f32, -125.0, -75.0, -35.0, 10.0, 45.0, 145.0]
        .iter()
        .enumerate()
    {
        let rad = angle_deg.to_radians() + sway * 0.015;
        let len = (68.0 + (idx % 3) as f32 * 14.0) * scale;
        let mid_x = top_x + rad.cos() * len * 0.55;
        let mid_y = top_y + rad.sin() * len * 0.45 - 8.0 * scale;
        let end_x = top_x + rad.cos() * len;
        let end_y = top_y + rad.sin() * len + 26.0 * scale;

        let col = if idx % 2 == 0 { frond_dark } else { frond_light };
        let pts = vec![
            tf.pt(top_x, top_y),
            tf.pt(mid_x, mid_y),
            tf.pt(end_x, end_y),
        ];
        painter.add(Shape::line(pts, Stroke::new(tf.s(5.5 * scale), col)));

        // Extra frond flare
        let pts2 = vec![
            tf.pt(top_x, top_y),
            tf.pt(mid_x + 4.0, mid_y + 6.0),
            tf.pt(end_x - 3.0, end_y + 4.0),
        ];
        painter.add(Shape::line(pts2, Stroke::new(tf.s(3.2 * scale), frond_light)));
    }
}

fn draw_net_shadow(painter: &Painter, tf: &CourtTransform, show_shadow: bool) {
    if !show_shadow {
        return;
    }
    // Diagonal net mesh shadow on the sand (signature beach volley look)
    let shadow_col = Color32::from_rgba_unmultiplied(44, 42, 22, 72);
    let p_top_l = tf.pt(NET_X + 6.0, 398.0);
    let p_top_r = tf.pt(NET_X + 46.0, 398.0);
    let p_bot_l = tf.pt(NET_X + 10.0, 594.0);
    let p_bot_r = tf.pt(NET_X + 122.0, 566.0);

    painter.line_segment([p_top_l, p_top_r], Stroke::new(tf.s(1.6), shadow_col));
    painter.line_segment([p_top_r, p_bot_r], Stroke::new(tf.s(2.4), shadow_col));
    painter.line_segment([p_bot_l, p_bot_r], Stroke::new(tf.s(3.2), shadow_col));

    // Mesh lines inside shadow
    for i in 1..10 {
        let f = i as f32 / 10.0;
        let a = lerp_pos(p_top_l, p_bot_l, f);
        let b = lerp_pos(p_top_r, p_bot_r, f);
        painter.line_segment([a, b], Stroke::new(tf.s(1.0), shadow_col));
    }
    for j in 1..4 {
        let f = j as f32 / 4.0;
        let a = lerp_pos(p_top_l, p_top_r, f);
        let b = lerp_pos(p_bot_l, p_bot_r, f);
        painter.line_segment([a, b], Stroke::new(tf.s(1.0), shadow_col));
    }
}

fn draw_net(painter: &Painter, tf: &CourtTransform) {
    // 3D-shaded white/silver net pole from NET_TOP_Y to bottom of screen
    let left_x = NET_X - NET_RADIUS;
    let right_x = NET_X + NET_RADIUS;
    let base_rect = Rect::from_min_max(tf.pt(left_x, NET_TOP_Y), tf.pt(right_x, FIELD_HEIGHT));
    painter.rect_filled(base_rect, 0.0, Color32::from_rgb(172, 178, 186));

    // Lighter left highlight strip
    let hi_rect = Rect::from_min_max(
        tf.pt(left_x + 2.0, NET_TOP_Y),
        tf.pt(left_x + 6.5, FIELD_HEIGHT),
    );
    painter.rect_filled(hi_rect, 0.0, Color32::from_rgb(236, 240, 245));

    // Subtle right shade strip
    let sh_rect = Rect::from_min_max(
        tf.pt(right_x - 3.5, NET_TOP_Y),
        tf.pt(right_x, FIELD_HEIGHT),
    );
    painter.rect_filled(sh_rect, 0.0, Color32::from_rgb(112, 118, 128));

    // Rounded top cap sphere at (NET_X, NET_TOP_Y)
    painter.circle_filled(
        tf.pt(NET_X, NET_TOP_Y),
        tf.s(NET_RADIUS),
        Color32::from_rgb(212, 218, 226),
    );
    painter.circle_filled(
        tf.pt(NET_X - 2.0, NET_TOP_Y - 2.0),
        tf.s(NET_RADIUS * 0.48),
        Color32::from_rgb(248, 250, 255),
    );
}

fn draw_blob_shadow(painter: &Painter, tf: &CourtTransform, blob: &Blob) {
    let height = (BLOBBY_GROUND_Y - blob.y).max(0.0);
    let factor = (1.0 - height / 360.0).clamp(0.32, 1.0);
    let alpha = (95.0 * factor) as u8;
    let rx = 36.0 * factor;
    let ry = 10.0 * factor;
    draw_ellipse(
        painter,
        tf.pt(blob.x + 10.0 + height * 0.08, GROUND_PLANE_Y + 2.0),
        tf.s(rx),
        tf.s(ry),
        Color32::from_rgba_unmultiplied(35, 32, 18, alpha),
    );
}

fn draw_ball_shadow(painter: &Painter, tf: &CourtTransform, bx: f32, by: f32) {
    let height = (GROUND_PLANE_Y - BALL_RADIUS - by).max(0.0);
    let factor = (1.0 - height / 520.0).clamp(0.25, 1.0);
    let alpha = (88.0 * factor) as u8;
    let rx = BALL_RADIUS * 0.95 * factor;
    let ry = 8.5 * factor;
    draw_ellipse(
        painter,
        tf.pt(bx + 12.0 + height * 0.06, GROUND_PLANE_Y + 2.0),
        tf.s(rx),
        tf.s(ry),
        Color32::from_rgba_unmultiplied(35, 32, 18, alpha),
    );
}

fn draw_blob(
    painter: &Painter,
    tf: &CourtTransform,
    blob: &Blob,
    base_color: Color32,
    ball_x: f32,
    ball_y: f32,
) {
    // Subtle squash-and-stretch + walk bob
    let bob = (blob.anim_phase.sin().abs()) * 2.2;
    let sx = 1.0 + blob.squash * 0.65;
    let sy = 1.0 - blob.squash * 0.55;

    let cx = blob.x;
    let cy = blob.y - bob;

    let upper_y = cy - BLOBBY_UPPER_SPHERE * sy;
    let lower_y = cy + BLOBBY_LOWER_SPHERE * sy;
    let r_up_x = BLOBBY_UPPER_RADIUS * sx;
    let r_up_y = BLOBBY_UPPER_RADIUS * sy;
    let r_lo_x = BLOBBY_LOWER_RADIUS * sx;
    let r_lo_y = BLOBBY_LOWER_RADIUS * sy;

    let dark_col = shade_color(base_color, 0.56);
    let mid_col = base_color;
    let bright_col = tint_color(base_color, 0.38);

    // Smooth teardrop/blob hull connecting upper & lower spheres
    let outline_pts = build_blob_outline(tf, cx, upper_y, r_up_x, r_up_y, lower_y, r_lo_x, r_lo_y);
    painter.add(Shape::convex_polygon(
        outline_pts.clone(),
        dark_col,
        Stroke::new(tf.s(1.6), shade_color(base_color, 0.35)),
    ));

    // Inner lit body slightly shifted up-left for 3D spherical volume
    let inner_pts = build_blob_outline(
        tf,
        cx - 1.8,
        upper_y - 1.4,
        r_up_x * 0.88,
        r_up_y * 0.88,
        lower_y - 1.8,
        r_lo_x * 0.88,
        r_lo_y * 0.88,
    );
    painter.add(Shape::convex_polygon(inner_pts, mid_col, Stroke::NONE));

    // Upper chest/head bright diffuse patch
    draw_ellipse(
        painter,
        tf.pt(cx - 4.0, upper_y - 2.0),
        tf.s(r_up_x * 0.65),
        tf.s(r_up_y * 0.62),
        bright_col,
    );
    draw_ellipse(
        painter,
        tf.pt(cx - 4.0, lower_y - 3.0),
        tf.s(r_lo_x * 0.62),
        tf.s(r_lo_y * 0.52),
        tint_color(base_color, 0.22),
    );

    // Glossy specular highlights (matching classic glossy blob look)
    draw_ellipse(
        painter,
        tf.pt(cx - 7.5, upper_y - 9.0),
        tf.s(6.5),
        tf.s(4.2),
        Color32::from_rgba_unmultiplied(255, 255, 255, 195),
    );
    draw_ellipse(
        painter,
        tf.pt(cx + 8.0, lower_y + 13.0),
        tf.s(5.0),
        tf.s(3.2),
        Color32::from_rgba_unmultiplied(255, 255, 255, 120),
    );

    // Expressive eyes tracking the ball
    let dx = ball_x - cx;
    let dy = ball_y - upper_y;
    let d = (dx * dx + dy * dy).sqrt().max(1.0);
    let look_x = (dx / d) * 3.0;
    let look_y = (dy / d) * 2.2;
    let face_offset = if cx < NET_X { 6.0 } else { -6.0 };

    for eye_dx in [-5.5f32, 5.5f32] {
        let ex = cx + face_offset + eye_dx;
        let ey = upper_y - 1.0;
        painter.circle_filled(tf.pt(ex, ey), tf.s(4.6), Color32::WHITE);
        painter.circle_filled(
            tf.pt(ex + look_x * 0.65, ey + look_y * 0.65),
            tf.s(2.3),
            Color32::from_rgb(20, 24, 34),
        );
    }
}

fn build_blob_outline(
    tf: &CourtTransform,
    cx: f32,
    up_y: f32,
    r_up_x: f32,
    r_up_y: f32,
    lo_y: f32,
    r_lo_x: f32,
    r_lo_y: f32,
) -> Vec<Pos2> {
    let mut pts = Vec::with_capacity(32);
    // Upper dome from right-side to left-side (angles -0.15..PI+0.15 upward)
    let steps = 14;
    for i in 0..=steps {
        let a = std::f32::consts::PI * (i as f32 / steps as f32); // 0..PI
        let px = cx + a.cos() * r_up_x;
        let py = up_y - a.sin() * r_up_y;
        pts.push(tf.pt(px, py));
    }
    // Lower belly from left-side around bottom to right-side
    for i in 0..=steps {
        let a = std::f32::consts::PI + std::f32::consts::PI * (i as f32 / steps as f32);
        let px = cx + a.cos() * r_lo_x;
        let py = lo_y - a.sin() * r_lo_y;
        pts.push(tf.pt(px, py));
    }
    pts
}

fn draw_ball(painter: &Painter, tf: &CourtTransform, bx: f32, by: f32, rot: f32) {
    // If ball flies above top of screen, draw crisp tracking indicator arrow at top edge
    if by + BALL_RADIUS < 0.0 {
        let ax = bx.clamp(24.0, FIELD_WIDTH - 24.0);
        let pts = vec![
            tf.pt(ax, 8.0),
            tf.pt(ax - 11.0, 24.0),
            tf.pt(ax + 11.0, 24.0),
        ];
        painter.add(Shape::convex_polygon(
            pts,
            Color32::from_rgb(255, 232, 85),
            Stroke::new(tf.s(1.8), Color32::from_rgb(40, 44, 58)),
        ));
        return;
    }

    let center = tf.pt(bx, by);
    let r = tf.s(BALL_RADIUS);

    // Base shaded volleyball sphere
    painter.circle_filled(center, r, Color32::from_rgb(182, 186, 194));
    painter.circle_filled(
        Pos2::new(center.x - r * 0.12, center.y - r * 0.14),
        r * 0.84,
        Color32::from_rgb(232, 236, 242),
    );
    painter.circle_filled(
        Pos2::new(center.x - r * 0.28, center.y - r * 0.30),
        r * 0.44,
        Color32::from_rgb(252, 253, 255),
    );

    // Rotating volleyball seam lines (3 curved panels)
    let seam_col = Color32::from_rgba_unmultiplied(92, 98, 112, 175);
    for k in 0..3 {
        let base_a = rot + k as f32 * (TAU / 3.0);
        let mut pts = Vec::with_capacity(9);
        for s in 0..=8 {
            let f = s as f32 / 8.0; // 0..1 from center to rim
            let curve = base_a + f * 0.45;
            let px = center.x + curve.cos() * r * 0.94 * f;
            let py = center.y + curve.sin() * r * 0.94 * f;
            pts.push(Pos2::new(px, py));
        }
        painter.add(Shape::line(pts, Stroke::new(tf.s(1.8), seam_col)));

        // Cross panel stitch
        let a1 = base_a + 0.18;
        let a2 = base_a + (TAU / 3.0) - 0.18;
        let p1 = Pos2::new(center.x + a1.cos() * r * 0.56, center.y + a1.sin() * r * 0.56);
        let p2 = Pos2::new(center.x + a2.cos() * r * 0.56, center.y + a2.sin() * r * 0.56);
        painter.line_segment([p1, p2], Stroke::new(tf.s(1.3), seam_col));
    }

    // Crisp rim outline
    painter.circle_stroke(
        center,
        r,
        Stroke::new(tf.s(1.5), Color32::from_rgba_unmultiplied(55, 62, 78, 180)),
    );
}

fn draw_hud(
    painter: &Painter,
    tf: &CourtTransform,
    world: &MatchWorld,
    opts: &VisualOptions,
    left_col: Color32,
    right_col: Color32,
    fps: f32,
) {
    // Scoreboards at top-left and top-right (classic Blobby Volley 2 layout)
    let l_txt = if world.serving_side == Side::Left {
        format!("{:02} !", world.scores[0])
    } else {
        format!("{:02}", world.scores[0])
    };
    let r_txt = if world.serving_side == Side::Right {
        format!("! {:02}", world.scores[1])
    } else {
        format!("{:02}", world.scores[1])
    };

    let font_score = FontId::proportional(tf.s(38.0));
    draw_shadow_text(painter, tf.pt(32.0, 22.0), Align2::LEFT_TOP, &l_txt, font_score.clone(), Color32::WHITE);
    draw_shadow_text(
        painter,
        tf.pt(FIELD_WIDTH - 32.0, 22.0),
        Align2::RIGHT_TOP,
        &r_txt,
        font_score,
        Color32::WHITE,
    );

    // Touch indicator pips (max 3 touches per side)
    for i in 0..MAX_TOUCHES {
        let filled_l = i < world.touches[0];
        let filled_r = i < world.touches[1];
        let lx = 38.0 + i as f32 * 16.0;
        let rx = FIELD_WIDTH - 38.0 - i as f32 * 16.0;
        let py = 72.0;

        painter.circle_filled(
            tf.pt(lx, py),
            tf.s(4.8),
            if filled_l { left_col } else { Color32::from_rgba_unmultiplied(255, 255, 255, 65) },
        );
        painter.circle_stroke(tf.pt(lx, py), tf.s(4.8), Stroke::new(tf.s(1.2), Color32::WHITE));

        painter.circle_filled(
            tf.pt(rx, py),
            tf.s(4.8),
            if filled_r { right_col } else { Color32::from_rgba_unmultiplied(255, 255, 255, 65) },
        );
        painter.circle_stroke(tf.pt(rx, py), tf.s(4.8), Stroke::new(tf.s(1.2), Color32::WHITE));
    }

    // Optional FPS counter (bottom-right, clean pill)
    if opts.show_fps {
        let fps_txt = format!("{:.0} FPS", fps.clamp(0.0, 999.0));
        draw_shadow_text(
            painter,
            tf.pt(FIELD_WIDTH - 14.0, FIELD_HEIGHT - 12.0),
            Align2::RIGHT_BOTTOM,
            &fps_txt,
            FontId::monospace(tf.s(14.0)),
            Color32::from_rgba_unmultiplied(255, 255, 255, 215),
        );
    }

    // Serve prompt when ball is waiting
    if !world.is_game_running && world.is_ball_valid && world.winner.is_none() {
        let hint = match world.serving_side {
            Side::Left => "BLUE SERVE",
            Side::Right => "RED SERVE",
        };
        draw_shadow_text(
            painter,
            tf.pt(FIELD_WIDTH * 0.5, 24.0),
            Align2::CENTER_TOP,
            hint,
            FontId::proportional(tf.s(18.0)),
            Color32::from_rgb(255, 244, 175),
        );
    }

    // Point scored banner
    if let Some(ref banner) = world.banner {
        let pill = Rect::from_center_size(tf.pt(FIELD_WIDTH * 0.5, 118.0), Vec2::new(tf.s(300.0), tf.s(46.0)));
        painter.rect_filled(
            pill,
            Rounding::same(tf.s(12.0)),
            Color32::from_rgba_unmultiplied(18, 28, 48, 190),
        );
        painter.rect_stroke(
            pill,
            Rounding::same(tf.s(12.0)),
            Stroke::new(
                tf.s(2.0),
                if banner.side == Side::Left { left_col } else { right_col },
            ),
        );
        painter.text(
            pill.center(),
            Align2::CENTER_CENTER,
            &banner.text,
            FontId::proportional(tf.s(22.0)),
            Color32::WHITE,
        );
    }
}

fn draw_shadow_text(
    painter: &Painter,
    pos: Pos2,
    align: Align2,
    text: &str,
    font: FontId,
    color: Color32,
) {
    painter.text(
        Pos2::new(pos.x + 2.0, pos.y + 2.0),
        align,
        text,
        font.clone(),
        Color32::from_rgba_unmultiplied(0, 0, 0, 155),
    );
    painter.text(pos, align, text, font, color);
}

fn draw_ellipse(painter: &Painter, center: Pos2, rx: f32, ry: f32, color: Color32) {
    let segs = 20;
    let mut pts = Vec::with_capacity(segs);
    for i in 0..segs {
        let a = TAU * (i as f32 / segs as f32);
        pts.push(Pos2::new(center.x + a.cos() * rx, center.y + a.sin() * ry));
    }
    painter.add(Shape::convex_polygon(pts, color, Stroke::NONE));
}

pub fn effective_blob_color(rgb: [u8; 3], oscillate: bool, tick: u64, phase: f32) -> Color32 {
    if !oscillate {
        return Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
    }
    let t = tick as f32 * 0.03 + phase;
    let r = (127.0 + 120.0 * t.sin()) as u8;
    let g = (127.0 + 120.0 * (t + 2.094).sin()) as u8;
    let b = (127.0 + 120.0 * (t + 4.188).sin()) as u8;
    Color32::from_rgb(r, g, b)
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    Color32::from_rgb(
        (a.r() as f32 + (b.r() as f32 - a.r() as f32) * t) as u8,
        (a.g() as f32 + (b.g() as f32 - a.g() as f32) * t) as u8,
        (a.b() as f32 + (b.b() as f32 - a.b() as f32) * t) as u8,
    )
}

fn lerp_pos(a: Pos2, b: Pos2, t: f32) -> Pos2 {
    Pos2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn shade_color(c: Color32, factor: f32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 * factor).clamp(0.0, 255.0) as u8,
        (c.g() as f32 * factor).clamp(0.0, 255.0) as u8,
        (c.b() as f32 * factor).clamp(0.0, 255.0) as u8,
    )
}

fn tint_color(c: Color32, amount: f32) -> Color32 {
    Color32::from_rgb(
        (c.r() as f32 + (255.0 - c.r() as f32) * amount).clamp(0.0, 255.0) as u8,
        (c.g() as f32 + (255.0 - c.g() as f32) * amount).clamp(0.0, 255.0) as u8,
        (c.b() as f32 + (255.0 - c.b() as f32) * amount).clamp(0.0, 255.0) as u8,
    )
}
