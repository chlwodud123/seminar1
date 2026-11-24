use eframe::egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

use crate::tools::ToolKind;

/// Draw a crisp procedural vector icon for any of the 22 tools inside `rect`.
pub fn paint_tool_icon(painter: &Painter, rect: Rect, tool: ToolKind, fg: Color32, accent: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height()) * 0.36;
    let stroke = Stroke::new(1.5_f32, fg);
    let thin = Stroke::new(1.2_f32, fg);
    let acc_stroke = Stroke::new(1.5_f32, accent);

    match tool {
        ToolKind::MoveSelectedPixels => {
            // Solid pointer arrow + small 4-way cross
            let pts = vec![
                Pos2::new(c.x - s * 0.7, c.y - s * 0.85),
                Pos2::new(c.x - s * 0.7, c.y + s * 0.65),
                Pos2::new(c.x - s * 0.2, c.y + s * 0.2),
                Pos2::new(c.x + s * 0.45, c.y + s * 0.2),
            ];
            painter.add(eframe::egui::Shape::convex_polygon(pts, fg, Stroke::NONE));
            let m = Pos2::new(c.x + s * 0.45, c.y + s * 0.55);
            painter.line_segment([Pos2::new(m.x - 4.0, m.y), Pos2::new(m.x + 4.0, m.y)], acc_stroke);
            painter.line_segment([Pos2::new(m.x, m.y - 4.0), Pos2::new(m.x, m.y + 4.0)], acc_stroke);
        }
        ToolKind::MoveSelection => {
            // Hollow pointer arrow + dashed corner
            let pts = vec![
                Pos2::new(c.x - s * 0.65, c.y - s * 0.8),
                Pos2::new(c.x - s * 0.65, c.y + s * 0.6),
                Pos2::new(c.x - s * 0.18, c.y + s * 0.18),
                Pos2::new(c.x + s * 0.42, c.y + s * 0.18),
                Pos2::new(c.x - s * 0.65, c.y - s * 0.8),
            ];
            painter.line(pts, stroke);
            let r = Rect::from_min_max(
                Pos2::new(c.x + 1.0, c.y + 2.0),
                Pos2::new(c.x + s * 0.9, c.y + s * 0.9),
            );
            paint_dashed_rect(painter, r, acc_stroke);
        }
        ToolKind::Zoom => {
            let center = Pos2::new(c.x - s * 0.18, c.y - s * 0.18);
            let r = s * 0.62;
            painter.circle_stroke(center, r, stroke);
            painter.line_segment(
                [
                    Pos2::new(center.x + r * 0.72, center.y + r * 0.72),
                    Pos2::new(c.x + s * 0.88, c.y + s * 0.88),
                ],
                Stroke::new(2.2_f32, fg),
            );
            painter.line_segment(
                [Pos2::new(center.x - r * 0.4, center.y), Pos2::new(center.x + r * 0.4, center.y)],
                thin,
            );
            painter.line_segment(
                [Pos2::new(center.x, center.y - r * 0.4), Pos2::new(center.x, center.y + r * 0.4)],
                thin,
            );
        }
        ToolKind::Pan => {
            // 4-way pan arrows
            let l = s * 0.85;
            painter.line_segment([Pos2::new(c.x - l, c.y), Pos2::new(c.x + l, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x, c.y - l), Pos2::new(c.x, c.y + l)], stroke);
            let a = s * 0.28;
            // tips
            painter.line_segment([Pos2::new(c.x - l, c.y), Pos2::new(c.x - l + a, c.y - a)], thin);
            painter.line_segment([Pos2::new(c.x - l, c.y), Pos2::new(c.x - l + a, c.y + a)], thin);
            painter.line_segment([Pos2::new(c.x + l, c.y), Pos2::new(c.x + l - a, c.y - a)], thin);
            painter.line_segment([Pos2::new(c.x + l, c.y), Pos2::new(c.x + l - a, c.y + a)], thin);
            painter.line_segment([Pos2::new(c.x, c.y - l), Pos2::new(c.x - a, c.y - l + a)], thin);
            painter.line_segment([Pos2::new(c.x, c.y - l), Pos2::new(c.x + a, c.y - l + a)], thin);
            painter.line_segment([Pos2::new(c.x, c.y + l), Pos2::new(c.x - a, c.y + l - a)], thin);
            painter.line_segment([Pos2::new(c.x, c.y + l), Pos2::new(c.x + a, c.y + l - a)], thin);
        }
        ToolKind::RectangleSelect => {
            let r = Rect::from_center_size(c, Vec2::new(s * 1.65, s * 1.35));
            paint_dashed_rect(painter, r, stroke);
        }
        ToolKind::EllipseSelect => {
            let n = 16;
            for i in (0..n).step_by(2) {
                let a0 = (i as f32 / n as f32) * std::f32::consts::TAU;
                let a1 = ((i + 1) as f32 / n as f32) * std::f32::consts::TAU;
                let p0 = Pos2::new(c.x + a0.cos() * s * 0.88, c.y + a0.sin() * s * 0.65);
                let p1 = Pos2::new(c.x + a1.cos() * s * 0.88, c.y + a1.sin() * s * 0.65);
                painter.line_segment([p0, p1], stroke);
            }
        }
        ToolKind::LassoSelect => {
            let pts = [
                Pos2::new(c.x - s * 0.7, c.y - s * 0.1),
                Pos2::new(c.x - s * 0.3, c.y - s * 0.75),
                Pos2::new(c.x + s * 0.55, c.y - s * 0.6),
                Pos2::new(c.x + s * 0.75, c.y + s * 0.1),
                Pos2::new(c.x + s * 0.1, c.y + s * 0.45),
                Pos2::new(c.x - s * 0.7, c.y - s * 0.1),
            ];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], thin);
            }
            painter.circle_stroke(Pos2::new(c.x - s * 0.35, c.y + s * 0.68), s * 0.24, acc_stroke);
        }
        ToolKind::MagicWand => {
            painter.line_segment(
                [Pos2::new(c.x - s * 0.75, c.y + s * 0.75), Pos2::new(c.x + s * 0.35, c.y - s * 0.35)],
                Stroke::new(2.2_f32, fg),
            );
            let tip = Pos2::new(c.x + s * 0.45, c.y - s * 0.45);
            for i in 0..6 {
                let a = i as f32 * std::f32::consts::FRAC_PI_3;
                painter.line_segment(
                    [
                        Pos2::new(tip.x + a.cos() * s * 0.18, tip.y + a.sin() * s * 0.18),
                        Pos2::new(tip.x + a.cos() * s * 0.5, tip.y + a.sin() * s * 0.5),
                    ],
                    acc_stroke,
                );
            }
        }
        ToolKind::Paintbrush => {
            // Diagonal handle + teardrop brush tip
            painter.line_segment(
                [Pos2::new(c.x + s * 0.8, c.y - s * 0.8), Pos2::new(c.x - s * 0.1, c.y + s * 0.1)],
                Stroke::new(2.6_f32, fg),
            );
            painter.circle_filled(Pos2::new(c.x - s * 0.35, c.y + s * 0.35), s * 0.32, accent);
            let tip = vec![
                Pos2::new(c.x - s * 0.85, c.y + s * 0.85),
                Pos2::new(c.x - s * 0.55, c.y + s * 0.18),
                Pos2::new(c.x - s * 0.18, c.y + s * 0.55),
            ];
            painter.add(eframe::egui::Shape::convex_polygon(tip, accent, Stroke::NONE));
        }
        ToolKind::Pencil => {
            painter.line_segment(
                [Pos2::new(c.x + s * 0.75, c.y - s * 0.75), Pos2::new(c.x - s * 0.35, c.y + s * 0.35)],
                Stroke::new(2.8_f32, fg),
            );
            let tip = vec![
                Pos2::new(c.x - s * 0.82, c.y + s * 0.82),
                Pos2::new(c.x - s * 0.5, c.y + s * 0.18),
                Pos2::new(c.x - s * 0.18, c.y + s * 0.5),
            ];
            painter.add(eframe::egui::Shape::convex_polygon(tip, fg, Stroke::NONE));
        }
        ToolKind::Eraser => {
            let pts = vec![
                Pos2::new(c.x - s * 0.75, c.y + s * 0.2),
                Pos2::new(c.x + s * 0.1, c.y - s * 0.65),
                Pos2::new(c.x + s * 0.8, c.y + s * 0.05),
                Pos2::new(c.x - s * 0.05, c.y + s * 0.78),
            ];
            painter.add(eframe::egui::Shape::convex_polygon(pts, Color32::from_rgb(235, 145, 160), stroke));
            painter.line_segment(
                [Pos2::new(c.x - s * 0.8, c.y + s * 0.8), Pos2::new(c.x + s * 0.8, c.y + s * 0.8)],
                thin,
            );
        }
        ToolKind::PaintBucket => {
            let bucket = vec![
                Pos2::new(c.x - s * 0.7, c.y),
                Pos2::new(c.x + s * 0.05, c.y - s * 0.72),
                Pos2::new(c.x + s * 0.62, c.y - s * 0.15),
                Pos2::new(c.x - s * 0.12, c.y + s * 0.58),
            ];
            painter.add(eframe::egui::Shape::convex_polygon(bucket, Color32::TRANSPARENT, stroke));
            painter.circle_filled(Pos2::new(c.x + s * 0.65, c.y + s * 0.55), s * 0.25, accent);
            painter.line_segment(
                [Pos2::new(c.x + s * 0.55, c.y - s * 0.1), Pos2::new(c.x + s * 0.65, c.y + s * 0.4)],
                Stroke::new(2.0_f32, accent),
            );
        }
        ToolKind::Gradient => {
            let r = Rect::from_center_size(c, Vec2::new(s * 1.6, s * 1.4));
            let cols = 6;
            let cw = r.width() / cols as f32;
            for i in 0..cols {
                let t = i as f32 / (cols - 1) as f32;
                let v = (40.0 + t * 200.0) as u8;
                let sr = Rect::from_min_size(
                    Pos2::new(r.left() + i as f32 * cw, r.top()),
                    Vec2::new(cw + 0.5, r.height()),
                );
                painter.rect_filled(sr, 0.0, Color32::from_gray(v));
            }
            painter.rect_stroke(r, 1.0, thin, eframe::egui::StrokeKind::Inside);
        }
        ToolKind::ColorPicker => {
            painter.line_segment(
                [Pos2::new(c.x - s * 0.75, c.y + s * 0.75), Pos2::new(c.x + s * 0.3, c.y - s * 0.3)],
                Stroke::new(2.2_f32, fg),
            );
            painter.circle_filled(Pos2::new(c.x + s * 0.5, c.y - s * 0.5), s * 0.32, accent);
            painter.line_segment(
                [Pos2::new(c.x + s * 0.0, c.y - s * 0.45), Pos2::new(c.x + s * 0.45, c.y - s * 0.0)],
                stroke,
            );
        }
        ToolKind::Text => {
            // Bold serif T
            painter.line_segment(
                [Pos2::new(c.x - s * 0.72, c.y - s * 0.7), Pos2::new(c.x + s * 0.72, c.y - s * 0.7)],
                Stroke::new(2.2_f32, fg),
            );
            painter.line_segment(
                [Pos2::new(c.x, c.y - s * 0.7), Pos2::new(c.x, c.y + s * 0.78)],
                Stroke::new(2.2_f32, fg),
            );
            painter.line_segment(
                [Pos2::new(c.x - s * 0.32, c.y + s * 0.78), Pos2::new(c.x + s * 0.32, c.y + s * 0.78)],
                Stroke::new(1.8_f32, fg),
            );
        }
        ToolKind::LineCurve => {
            painter.line_segment(
                [Pos2::new(c.x - s * 0.75, c.y + s * 0.72), Pos2::new(c.x + s * 0.75, c.y - s * 0.72)],
                Stroke::new(2.0_f32, fg),
            );
            painter.circle_filled(Pos2::new(c.x - s * 0.75, c.y + s * 0.72), 2.2, accent);
            painter.circle_filled(Pos2::new(c.x + s * 0.75, c.y - s * 0.72), 2.2, accent);
        }
        ToolKind::Rectangle => {
            let r = Rect::from_center_size(c, Vec2::new(s * 1.65, s * 1.3));
            painter.rect_stroke(r, 0.0, stroke, eframe::egui::StrokeKind::Inside);
        }
        ToolKind::RoundedRectangle => {
            let r = Rect::from_center_size(c, Vec2::new(s * 1.65, s * 1.3));
            painter.rect_stroke(r, 4.0, stroke, eframe::egui::StrokeKind::Inside);
        }
        ToolKind::Ellipse => {
            let n = 24;
            let mut pts = Vec::with_capacity(n + 1);
            for i in 0..=n {
                let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                pts.push(Pos2::new(c.x + a.cos() * s * 0.86, c.y + a.sin() * s * 0.62));
            }
            painter.line(pts, stroke);
        }
        ToolKind::FreeformShape => {
            let pts = vec![
                Pos2::new(c.x - s * 0.7, c.y + s * 0.6),
                Pos2::new(c.x - s * 0.5, c.y - s * 0.6),
                Pos2::new(c.x + s * 0.2, c.y - s * 0.2),
                Pos2::new(c.x + s * 0.75, c.y - s * 0.65),
                Pos2::new(c.x + s * 0.65, c.y + s * 0.65),
                Pos2::new(c.x - s * 0.7, c.y + s * 0.6),
            ];
            painter.line(pts, stroke);
        }
        ToolKind::CloneStamp => {
            // Stamp handle + base
            painter.circle_stroke(Pos2::new(c.x, c.y - s * 0.42), s * 0.34, stroke);
            painter.line_segment(
                [Pos2::new(c.x, c.y - s * 0.08), Pos2::new(c.x, c.y + s * 0.35)],
                Stroke::new(2.4_f32, fg),
            );
            let base = Rect::from_min_max(
                Pos2::new(c.x - s * 0.72, c.y + s * 0.32),
                Pos2::new(c.x + s * 0.72, c.y + s * 0.72),
            );
            painter.rect_filled(base, 1.5, fg);
        }
        ToolKind::Recolor => {
            painter.circle_filled(Pos2::new(c.x - s * 0.32, c.y - s * 0.22), s * 0.44, Color32::from_rgb(230, 80, 75));
            painter.circle_filled(Pos2::new(c.x + s * 0.32, c.y + s * 0.22), s * 0.44, accent);
            painter.circle_stroke(Pos2::new(c.x + s * 0.32, c.y + s * 0.22), s * 0.44, thin);
        }
    }
}

fn paint_dashed_rect(painter: &Painter, r: Rect, stroke: Stroke) {
    let seg = 3.0;
    let mut x = r.left();
    while x < r.right() {
        let x2 = (x + seg).min(r.right());
        painter.line_segment([Pos2::new(x, r.top()), Pos2::new(x2, r.top())], stroke);
        painter.line_segment([Pos2::new(x, r.bottom()), Pos2::new(x2, r.bottom())], stroke);
        x += seg * 2.0;
    }
    let mut y = r.top();
    while y < r.bottom() {
        let y2 = (y + seg).min(r.bottom());
        painter.line_segment([Pos2::new(r.left(), y), Pos2::new(r.left(), y2)], stroke);
        painter.line_segment([Pos2::new(r.right(), y), Pos2::new(r.right(), y2)], stroke);
        y += seg * 2.0;
    }
}
