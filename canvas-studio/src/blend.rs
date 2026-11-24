/// All 14 layer blend modes from `Pinta.Core.BlendMode`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Normal,
    Multiply,
    ColorBurn,
    ColorDodge,
    Overlay,
    Difference,
    Lighten,
    Darken,
    Screen,
    Xor,
    HardLight,
    SoftLight,
    Color,
    Luminosity,
}

impl BlendMode {
    pub const ALL: [BlendMode; 14] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::ColorBurn,
        BlendMode::ColorDodge,
        BlendMode::Overlay,
        BlendMode::Difference,
        BlendMode::Lighten,
        BlendMode::Darken,
        BlendMode::Screen,
        BlendMode::Xor,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::Color,
        BlendMode::Luminosity,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::ColorBurn => "Color Burn",
            BlendMode::ColorDodge => "Color Dodge",
            BlendMode::Overlay => "Overlay",
            BlendMode::Difference => "Difference",
            BlendMode::Lighten => "Lighten",
            BlendMode::Darken => "Darken",
            BlendMode::Screen => "Screen",
            BlendMode::Xor => "Xor",
            BlendMode::HardLight => "Hard Light",
            BlendMode::SoftLight => "Soft Light",
            BlendMode::Color => "Color",
            BlendMode::Luminosity => "Luminosity",
        }
    }
}

#[inline]
fn blend_channel(mode: BlendMode, d: f32, s: f32) -> f32 {
    match mode {
        BlendMode::Normal => s,
        BlendMode::Multiply => d * s,
        BlendMode::ColorBurn => {
            if d >= 1.0 {
                1.0
            } else if s <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - d) / s).min(1.0)
            }
        }
        BlendMode::ColorDodge => {
            if d <= 0.0 {
                0.0
            } else if s >= 1.0 {
                1.0
            } else {
                (d / (1.0 - s)).min(1.0)
            }
        }
        BlendMode::Overlay => {
            if d < 0.5 {
                2.0 * d * s
            } else {
                1.0 - 2.0 * (1.0 - d) * (1.0 - s)
            }
        }
        BlendMode::Difference => (d - s).abs(),
        BlendMode::Lighten => d.max(s),
        BlendMode::Darken => d.min(s),
        BlendMode::Screen => 1.0 - (1.0 - d) * (1.0 - s),
        BlendMode::HardLight => {
            if s < 0.5 {
                2.0 * d * s
            } else {
                1.0 - 2.0 * (1.0 - d) * (1.0 - s)
            }
        }
        BlendMode::SoftLight => {
            if s <= 0.5 {
                d - (1.0 - 2.0 * s) * d * (1.0 - d)
            } else {
                let g = if d <= 0.25 {
                    ((16.0 * d - 12.0) * d + 4.0) * d
                } else {
                    d.sqrt()
                };
                d + (2.0 * s - 1.0) * (g - d)
            }
        }
        BlendMode::Xor | BlendMode::Color | BlendMode::Luminosity => s,
    }
}

#[inline]
fn lum(r: f32, g: f32, b: f32) -> f32 {
    0.3 * r + 0.59 * g + 0.11 * b
}

#[inline]
fn clip_color(mut r: f32, mut g: f32, mut b: f32) -> (f32, f32, f32) {
    let l = lum(r, g, b);
    let n = r.min(g).min(b);
    let x = r.max(g).max(b);
    if n < 0.0 && (l - n) > 1e-6 {
        r = l + ((r - l) * l) / (l - n);
        g = l + ((g - l) * l) / (l - n);
        b = l + ((b - l) * l) / (l - n);
    }
    if x > 1.0 && (x - l) > 1e-6 {
        r = l + ((r - l) * (1.0 - l)) / (x - l);
        g = l + ((g - l) * (1.0 - l)) / (x - l);
        b = l + ((b - l) * (1.0 - l)) / (x - l);
    }
    (r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0))
}

#[inline]
fn set_lum(r: f32, g: f32, b: f32, target_l: f32) -> (f32, f32, f32) {
    let d = target_l - lum(r, g, b);
    clip_color(r + d, g + d, b + d)
}

/// Composite straight-alpha `src` over straight-alpha `dst` with layer `opacity` (`0.0..=1.0`).
#[inline]
pub fn composite_pixel(dst: [u8; 4], src: [u8; 4], opacity: f32, mode: BlendMode) -> [u8; 4] {
    let sa = (src[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    if sa <= 0.0001 {
        return dst;
    }
    let da = dst[3] as f32 / 255.0;

    let sr = src[0] as f32 / 255.0;
    let sg = src[1] as f32 / 255.0;
    let sb = src[2] as f32 / 255.0;

    if mode == BlendMode::Xor {
        let out_a = sa * (1.0 - da) + da * (1.0 - sa);
        if out_a <= 0.0001 {
            return [0, 0, 0, 0];
        }
        let dr = dst[0] as f32 / 255.0;
        let dg = dst[1] as f32 / 255.0;
        let db = dst[2] as f32 / 255.0;
        let r = (sr * sa * (1.0 - da) + dr * da * (1.0 - sa)) / out_a;
        let g = (sg * sa * (1.0 - da) + dg * da * (1.0 - sa)) / out_a;
        let b = (sb * sa * (1.0 - da) + db * da * (1.0 - sa)) / out_a;
        return [
            (r.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            (g.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            (b.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
            (out_a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        ];
    }

    if da <= 0.0001 {
        return [src[0], src[1], src[2], (sa * 255.0 + 0.5) as u8];
    }

    let dr = dst[0] as f32 / 255.0;
    let dg = dst[1] as f32 / 255.0;
    let db = dst[2] as f32 / 255.0;

    let (br, bg, bb) = match mode {
        BlendMode::Color => set_lum(sr, sg, sb, lum(dr, dg, db)),
        BlendMode::Luminosity => set_lum(dr, dg, db, lum(sr, sg, sb)),
        _ => (
            blend_channel(mode, dr, sr),
            blend_channel(mode, dg, sg),
            blend_channel(mode, db, sb),
        ),
    };

    // Standard Porter-Duff source-over with blended RGB where both alphas overlap:
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0001 {
        return [0, 0, 0, 0];
    }
    let inv_out = 1.0 / out_a;
    let r = ((1.0 - sa) * da * dr + (1.0 - da) * sa * sr + sa * da * br) * inv_out;
    let g = ((1.0 - sa) * da * dg + (1.0 - da) * sa * sg + sa * da * bg) * inv_out;
    let b = ((1.0 - sa) * da * db + (1.0 - da) * sa * sb + sa * da * bb) * inv_out;

    [
        (r.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        (g.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        (b.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
        (out_a.clamp(0.0, 1.0) * 255.0 + 0.5) as u8,
    ]
}

/// Fast straight-alpha source-over blend for painting onto a layer.
#[inline]
pub fn paint_over(dst: [u8; 4], src: [u8; 4], overwrite: bool) -> [u8; 4] {
    if overwrite {
        return src;
    }
    let sa = src[3] as f32 / 255.0;
    if sa <= 0.0 {
        return dst;
    }
    if sa >= 1.0 {
        return src;
    }
    let da = dst[3] as f32 / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.0001 {
        return [0, 0, 0, 0];
    }
    let inv = 1.0 / out_a;
    let r = (src[0] as f32 * sa + dst[0] as f32 * da * (1.0 - sa)) * inv;
    let g = (src[1] as f32 * sa + dst[1] as f32 * da * (1.0 - sa)) * inv;
    let b = (src[2] as f32 * sa + dst[2] as f32 * da * (1.0 - sa)) * inv;
    [
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
        (out_a * 255.0 + 0.5).clamp(0.0, 255.0) as u8,
    ]
}
