use crate::canvas::SelectionMask;

/// All Adjustments & Effects extracted from `Pinta.Effects.dll`, with exact default parameters.
#[derive(Clone, Debug)]
pub enum EffectSpec {
    // --- Adjustments ---
    AutoLevel,
    BlackAndWhite,
    BrightnessContrast {
        brightness: i32, // -100..=100 (default 0)
        contrast: i32,   // -100..=100 (default 0)
    },
    HueSaturation {
        hue: i32,        // -180..=180 (default 0)
        saturation: i32, // 0..=200 (default 100, RVA 0xe2e0)
        lightness: i32,  // -100..=100 (default 0)
    },
    InvertColors,
    Levels {
        in_black: i32,  // 0..=254
        in_white: i32,  // 1..=255
        gamma: f32,     // 0.1..=10.0
        out_black: i32, // 0..=255
        out_white: i32, // 0..=255
    },
    Posterize {
        red: i32,   // 2..=64 (default 16, RVA 0x29fe)
        green: i32, // 2..=64 (default 16)
        blue: i32,  // 2..=64 (default 16)
        linked: bool,
    },
    Sepia,

    // --- Blurs ---
    GaussianBlur {
        radius: i32, // 1..=40 (default 2, RVA 0x11f79)
    },
    MotionBlur {
        angle: f32,    // -180..=180 (default 25.0, RVA 0x13035)
        distance: i32, // 1..=60 (default 10)
        centered: bool,
    },
    RadialBlur {
        angle: f32, // 0..=30 (default 2.0, RVA 0x141c6)
    },
    ZoomBlur {
        amount: i32, // 0..=100 (default 10, RVA 0x16298)
    },

    // --- Photo ---
    Sharpen {
        amount: i32, // 1..=20 (default 2, RVA 0xbd97)
    },
    Glow {
        radius: i32,     // 1..=20 (default 6, RVA 0x11fbb)
        brightness: i32, // -100..=100 (default 10)
        contrast: i32,   // -100..=100 (default 10)
    },
    Vignette {
        radius: i32,  // 10..=100 (default 50, RVA 0xce2a)
        density: f32, // 0.1..=4.0 (default 1.0)
    },

    // --- Distort ---
    Pixelate {
        cell_size: i32, // 1..=100 (default 2, RVA 0xb316)
    },
    Bulge {
        amount: i32, // -200..=100 (default 45, RVA 0x1002e)
    },
    Twist {
        amount: i32, // -100..=100 (default 30, RVA 0x15198)
        size: i32,   // 1..=200 (default 100)
    },
    FrostedGlass {
        radius: i32, // 1..=15 (default 1, RVA 0x11f42)
    },

    // --- Noise ---
    AddNoise {
        intensity: i32,  // 0..=100 (default 64, RVA 0xf977)
        saturation: i32, // 0..=200 (default 100)
        coverage: f32,   // 0..=100 (default 100.0)
    },

    // --- Stylize ---
    EdgeDetect {
        angle: f32, // 0..=360 (default 45.0, RVA 0x8348)
    },
    Emboss {
        angle: f32, // 0..=360 (default 0.0, RVA 0x115ac)
    },
    Relief {
        angle: f32, // 0..=360 (default 45.0, RVA 0xbc9c)
    },

    // --- Artistic ---
    OilPainting {
        brush_size: i32, // 1..=8 (default 3, RVA 0x13505)
        coarseness: i32, // 3..=255 (default 50)
    },
    PencilSketch {
        pencil_tip_size: i32, // 1..=20 (default 2, RVA 0x13d75)
    },

    // --- Render ---
    Clouds {
        scale: i32, // 10..=500 (default 250, RVA 0x10bec)
        power: i32, // 0..=100 (default 50)
    },
    MandelbrotFractal {
        zoom: f32, // 1..=100 (default 10.0, RVA 0x12d40)
    },
    JuliaFractal {
        zoom: f32, // 0.5..=20 (default 1.0, RVA 0x126f0)
    },
}

impl EffectSpec {
    pub fn name(&self) -> &'static str {
        match self {
            EffectSpec::AutoLevel => "Auto Level",
            EffectSpec::BlackAndWhite => "Black and White",
            EffectSpec::BrightnessContrast { .. } => "Brightness / Contrast",
            EffectSpec::HueSaturation { .. } => "Hue / Saturation",
            EffectSpec::InvertColors => "Invert Colors",
            EffectSpec::Levels { .. } => "Levels",
            EffectSpec::Posterize { .. } => "Posterize",
            EffectSpec::Sepia => "Sepia",
            EffectSpec::GaussianBlur { .. } => "Gaussian Blur",
            EffectSpec::MotionBlur { .. } => "Motion Blur",
            EffectSpec::RadialBlur { .. } => "Radial Blur",
            EffectSpec::ZoomBlur { .. } => "Zoom Blur",
            EffectSpec::Sharpen { .. } => "Sharpen",
            EffectSpec::Glow { .. } => "Glow",
            EffectSpec::Vignette { .. } => "Vignette",
            EffectSpec::Pixelate { .. } => "Pixelate",
            EffectSpec::Bulge { .. } => "Bulge",
            EffectSpec::Twist { .. } => "Twist",
            EffectSpec::FrostedGlass { .. } => "Frosted Glass",
            EffectSpec::AddNoise { .. } => "Add Noise",
            EffectSpec::EdgeDetect { .. } => "Edge Detect",
            EffectSpec::Emboss { .. } => "Emboss",
            EffectSpec::Relief { .. } => "Relief",
            EffectSpec::OilPainting { .. } => "Oil Painting",
            EffectSpec::PencilSketch { .. } => "Pencil Sketch",
            EffectSpec::Clouds { .. } => "Clouds",
            EffectSpec::MandelbrotFractal { .. } => "Mandelbrot Fractal",
            EffectSpec::JuliaFractal { .. } => "Julia Fractal",
        }
    }

    pub fn is_instant(&self) -> bool {
        matches!(
            self,
            EffectSpec::AutoLevel
                | EffectSpec::BlackAndWhite
                | EffectSpec::InvertColors
                | EffectSpec::Sepia
        )
    }
}

#[inline]
fn clamp_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

#[inline]
fn get_px(src: &[u8], w: u32, h: u32, x: i32, y: i32) -> [u8; 4] {
    let cx = x.clamp(0, w as i32 - 1) as u32;
    let cy = y.clamp(0, h as i32 - 1) as u32;
    let i = ((cy * w + cx) * 4) as usize;
    [src[i], src[i + 1], src[i + 2], src[i + 3]]
}

#[inline]
fn sample_bilinear(src: &[u8], w: u32, h: u32, fx: f32, fy: f32) -> [u8; 4] {
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let tx = fx - fx.floor();
    let ty = fy - fy.floor();
    let p00 = get_px(src, w, h, x0, y0);
    let p10 = get_px(src, w, h, x0 + 1, y0);
    let p01 = get_px(src, w, h, x0, y0 + 1);
    let p11 = get_px(src, w, h, x0 + 1, y0 + 1);
    let mut out = [0u8; 4];
    for c in 0..4 {
        let top = p00[c] as f32 * (1.0 - tx) + p10[c] as f32 * tx;
        let bot = p01[c] as f32 * (1.0 - tx) + p11[c] as f32 * tx;
        out[c] = clamp_u8(top * (1.0 - ty) + bot * ty);
    }
    out
}

fn rgb_to_hsl(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) * 0.5;
    if (max - min).abs() < 1e-6 {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let mut h = if (max - r).abs() < 1e-6 {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < 1e-6 {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    h /= 6.0;
    (h, s, l)
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    if s <= 1e-6 {
        return (l, l, l);
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    (
        hue_to_rgb(p, q, h + 1.0 / 3.0),
        hue_to_rgb(p, q, h),
        hue_to_rgb(p, q, h - 1.0 / 3.0),
    )
}

fn hash_u32(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846ca68b);
    x ^= x >> 16;
    x
}

fn value_noise(x: f32, y: f32, seed: u32) -> f32 {
    let xi = x.floor() as i32;
    let yi = y.floor() as i32;
    let tx = x - xi as f32;
    let ty = y - yi as f32;
    let u = tx * tx * (3.0 - 2.0 * tx);
    let v = ty * ty * (3.0 - 2.0 * ty);
    let h = |ix: i32, iy: i32| -> f32 {
        let k = (ix as u32)
            .wrapping_mul(374761393)
            .wrapping_add((iy as u32).wrapping_mul(668265263))
            .wrapping_add(seed);
        (hash_u32(k) & 0xFFFF) as f32 / 65535.0
    };
    let a = h(xi, yi) * (1.0 - u) + h(xi + 1, yi) * u;
    let b = h(xi, yi + 1) * (1.0 - u) + h(xi + 1, yi + 1) * u;
    a * (1.0 - v) + b * v
}

/// Fast separable Gaussian blur helper.
fn gaussian_blur_rgba(src: &[u8], w: u32, h: u32, radius: i32) -> Vec<u8> {
    let r = radius.clamp(1, 40) as usize;
    let sigma = (r as f32 * 0.5).max(0.5);
    let mut kernel = vec![0.0f32; r * 2 + 1];
    let mut sum = 0.0f32;
    for i in 0..kernel.len() {
        let d = i as f32 - r as f32;
        let k = (-0.5 * (d / sigma) * (d / sigma)).exp();
        kernel[i] = k;
        sum += k;
    }
    for k in &mut kernel {
        *k /= sum;
    }

    let mut tmp = vec![0u8; src.len()];
    // Horizontal pass
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 4];
            for (ki, &kw) in kernel.iter().enumerate() {
                let sx = (x as i32 + ki as i32 - r as i32).clamp(0, w as i32 - 1) as u32;
                let idx = ((y * w + sx) * 4) as usize;
                for c in 0..4 {
                    acc[c] += src[idx + c] as f32 * kw;
                }
            }
            let out_i = ((y * w + x) * 4) as usize;
            for c in 0..4 {
                tmp[out_i + c] = clamp_u8(acc[c]);
            }
        }
    }
    // Vertical pass
    let mut out = vec![0u8; src.len()];
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 4];
            for (ki, &kw) in kernel.iter().enumerate() {
                let sy = (y as i32 + ki as i32 - r as i32).clamp(0, h as i32 - 1) as u32;
                let idx = ((sy * w + x) * 4) as usize;
                for c in 0..4 {
                    acc[c] += tmp[idx + c] as f32 * kw;
                }
            }
            let out_i = ((y * w + x) * 4) as usize;
            for c in 0..4 {
                out[out_i + c] = clamp_u8(acc[c]);
            }
        }
    }
    out
}

/// Apply an `EffectSpec` to `src` pixels, respecting optional `selection`, returning a new RGBA buffer.
pub fn apply_effect(
    src: &[u8],
    width: u32,
    height: u32,
    selection: Option<&SelectionMask>,
    primary: [u8; 4],
    secondary: [u8; 4],
    spec: &EffectSpec,
) -> Vec<u8> {
    let mut out = src.to_vec();
    let w = width;
    let h = height;

    match *spec {
        EffectSpec::AutoLevel => {
            let mut min_c = [255u8; 3];
            let mut max_c = [0u8; 3];
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        if src[i + 3] == 0 {
                            continue;
                        }
                        for c in 0..3 {
                            min_c[c] = min_c[c].min(src[i + c]);
                            max_c[c] = max_c[c].max(src[i + c]);
                        }
                    }
                }
            }
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        for c in 0..3 {
                            let span = (max_c[c] as i32 - min_c[c] as i32).max(1) as f32;
                            out[i + c] =
                                clamp_u8((src[i + c].saturating_sub(min_c[c]) as f32 / span) * 255.0);
                        }
                    }
                }
            }
        }
        EffectSpec::BlackAndWhite => {
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        let g = clamp_u8(
                            0.299 * src[i] as f32 + 0.587 * src[i + 1] as f32 + 0.114 * src[i + 2] as f32,
                        );
                        out[i] = g;
                        out[i + 1] = g;
                        out[i + 2] = g;
                    }
                }
            }
        }
        EffectSpec::BrightnessContrast {
            brightness,
            contrast,
        } => {
            let b = brightness as f32 * 2.55;
            let c_val = contrast as f32 * 2.55;
            let factor = (259.0 * (c_val + 255.0)) / (255.0 * (259.0 - c_val).max(1.0));
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        for ch in 0..3 {
                            let v = factor * (src[i + ch] as f32 + b - 128.0) + 128.0;
                            out[i + ch] = clamp_u8(v);
                        }
                    }
                }
            }
        }
        EffectSpec::HueSaturation {
            hue,
            saturation,
            lightness,
        } => {
            let h_shift = hue as f32 / 360.0;
            let s_mul = saturation as f32 / 100.0;
            let l_shift = lightness as f32 / 100.0;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        let (mut hh, mut ss, mut ll) = rgb_to_hsl(
                            src[i] as f32 / 255.0,
                            src[i + 1] as f32 / 255.0,
                            src[i + 2] as f32 / 255.0,
                        );
                        hh = (hh + h_shift).rem_euclid(1.0);
                        ss = (ss * s_mul).clamp(0.0, 1.0);
                        ll = (ll + l_shift).clamp(0.0, 1.0);
                        let (r, g, b) = hsl_to_rgb(hh, ss, ll);
                        out[i] = clamp_u8(r * 255.0);
                        out[i + 1] = clamp_u8(g * 255.0);
                        out[i + 2] = clamp_u8(b * 255.0);
                    }
                }
            }
        }
        EffectSpec::InvertColors => {
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        out[i] = 255 - src[i];
                        out[i + 1] = 255 - src[i + 1];
                        out[i + 2] = 255 - src[i + 2];
                    }
                }
            }
        }
        EffectSpec::Levels {
            in_black,
            in_white,
            gamma,
            out_black,
            out_white,
        } => {
            let ib = in_black.clamp(0, 254) as f32;
            let iw = in_white.clamp(in_black + 1, 255) as f32;
            let inv_g = 1.0 / gamma.clamp(0.1, 10.0);
            let ob = out_black.clamp(0, 255) as f32;
            let ow = out_white.clamp(0, 255) as f32;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        for ch in 0..3 {
                            let t = ((src[i + ch] as f32 - ib) / (iw - ib)).clamp(0.0, 1.0);
                            let v = ob + t.powf(inv_g) * (ow - ob);
                            out[i + ch] = clamp_u8(v);
                        }
                    }
                }
            }
        }
        EffectSpec::Posterize {
            red, green, blue, ..
        } => {
            let lv = [
                red.clamp(2, 64) as f32,
                green.clamp(2, 64) as f32,
                blue.clamp(2, 64) as f32,
            ];
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        for ch in 0..3 {
                            let step = 255.0 / (lv[ch] - 1.0);
                            let q = ((src[i + ch] as f32 / step).round()) * step;
                            out[i + ch] = clamp_u8(q);
                        }
                    }
                }
            }
        }
        EffectSpec::Sepia => {
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        let r = src[i] as f32;
                        let g = src[i + 1] as f32;
                        let b = src[i + 2] as f32;
                        out[i] = clamp_u8(0.393 * r + 0.769 * g + 0.189 * b);
                        out[i + 1] = clamp_u8(0.349 * r + 0.686 * g + 0.168 * b);
                        out[i + 2] = clamp_u8(0.272 * r + 0.534 * g + 0.131 * b);
                    }
                }
            }
        }
        EffectSpec::GaussianBlur { radius } => {
            let blurred = gaussian_blur_rgba(src, w, h, radius);
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        out[i..i + 4].copy_from_slice(&blurred[i..i + 4]);
                    }
                }
            }
        }
        EffectSpec::MotionBlur {
            angle,
            distance,
            centered,
        } => {
            let rad = angle.to_radians();
            let dx = rad.cos();
            let dy = rad.sin();
            let dist = distance.clamp(1, 60);
            let (start, end) = if centered {
                (-dist / 2, dist / 2)
            } else {
                (0, dist)
            };
            let count = (end - start + 1).max(1) as f32;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let mut acc = [0.0f32; 4];
                        for s in start..=end {
                            let sx = (x as f32 + dx * s as f32).round() as i32;
                            let sy = (y as f32 + dy * s as f32).round() as i32;
                            let p = get_px(src, w, h, sx, sy);
                            for c in 0..4 {
                                acc[c] += p[c] as f32;
                            }
                        }
                        let i = ((y * w + x) * 4) as usize;
                        for c in 0..4 {
                            out[i + c] = clamp_u8(acc[c] / count);
                        }
                    }
                }
            }
        }
        EffectSpec::RadialBlur { angle } => {
            let cx = w as f32 * 0.5;
            let cy = h as f32 * 0.5;
            let max_rad = angle.to_radians();
            let samples = 11i32;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let rx = x as f32 - cx;
                        let ry = y as f32 - cy;
                        let mut acc = [0.0f32; 4];
                        for s in -samples / 2..=samples / 2 {
                            let a = max_rad * (s as f32 / (samples as f32 * 0.5));
                            let (sin_a, cos_a) = a.sin_cos();
                            let sx = cx + rx * cos_a - ry * sin_a;
                            let sy = cy + rx * sin_a + ry * cos_a;
                            let p = sample_bilinear(src, w, h, sx, sy);
                            for c in 0..4 {
                                acc[c] += p[c] as f32;
                            }
                        }
                        let i = ((y * w + x) * 4) as usize;
                        for c in 0..4 {
                            out[i + c] = clamp_u8(acc[c] / samples as f32);
                        }
                    }
                }
            }
        }
        EffectSpec::ZoomBlur { amount } => {
            let cx = w as f32 * 0.5;
            let cy = h as f32 * 0.5;
            let strength = (amount.clamp(0, 100) as f32) * 0.004;
            let samples = 12;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let rx = x as f32 - cx;
                        let ry = y as f32 - cy;
                        let mut acc = [0.0f32; 4];
                        for s in 0..samples {
                            let scale = 1.0 - strength * (s as f32 / samples as f32);
                            let p = sample_bilinear(src, w, h, cx + rx * scale, cy + ry * scale);
                            for c in 0..4 {
                                acc[c] += p[c] as f32;
                            }
                        }
                        let i = ((y * w + x) * 4) as usize;
                        for c in 0..4 {
                            out[i + c] = clamp_u8(acc[c] / samples as f32);
                        }
                    }
                }
            }
        }
        EffectSpec::Sharpen { amount } => {
            let blurred = gaussian_blur_rgba(src, w, h, 1);
            let k = amount.clamp(1, 20) as f32 * 0.35;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        for c in 0..3 {
                            let v = src[i + c] as f32 + k * (src[i + c] as f32 - blurred[i + c] as f32);
                            out[i + c] = clamp_u8(v);
                        }
                    }
                }
            }
        }
        EffectSpec::Glow {
            radius,
            brightness,
            contrast,
        } => {
            let blurred = gaussian_blur_rgba(src, w, h, radius);
            let b = brightness as f32 * 2.55;
            let cf = (259.0 * (contrast as f32 * 2.55 + 255.0))
                / (255.0 * (259.0 - contrast as f32 * 2.55).max(1.0));
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        for c in 0..3 {
                            let g = ((cf * (blurred[i + c] as f32 + b - 128.0) + 128.0) / 255.0)
                                .clamp(0.0, 1.0);
                            let s = src[i + c] as f32 / 255.0;
                            let screen = 1.0 - (1.0 - s) * (1.0 - g);
                            out[i + c] = clamp_u8(screen * 255.0);
                        }
                    }
                }
            }
        }
        EffectSpec::Vignette { radius, density } => {
            let cx = w as f32 * 0.5;
            let cy = h as f32 * 0.5;
            let max_r = (cx.min(cy) * (radius.clamp(10, 100) as f32 / 50.0)).max(1.0);
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let dx = x as f32 - cx;
                        let dy = y as f32 - cy;
                        let d = (dx * dx + dy * dy).sqrt() / max_r;
                        let factor = (1.0 - (d * d * 0.5 * density).min(1.0)).clamp(0.0, 1.0);
                        let i = ((y * w + x) * 4) as usize;
                        for c in 0..3 {
                            out[i + c] = clamp_u8(src[i + c] as f32 * factor);
                        }
                    }
                }
            }
        }
        EffectSpec::Pixelate { cell_size } => {
            let cs = cell_size.clamp(1, 100) as u32;
            for by in (0..h).step_by(cs as usize) {
                for bx in (0..w).step_by(cs as usize) {
                    let mut sum = [0u64; 4];
                    let mut cnt = 0u64;
                    let ye = (by + cs).min(h);
                    let xe = (bx + cs).min(w);
                    for y in by..ye {
                        for x in bx..xe {
                            let i = ((y * w + x) * 4) as usize;
                            for c in 0..4 {
                                sum[c] += src[i + c] as u64;
                            }
                            cnt += 1;
                        }
                    }
                    if cnt == 0 {
                        continue;
                    }
                    let avg = [
                        (sum[0] / cnt) as u8,
                        (sum[1] / cnt) as u8,
                        (sum[2] / cnt) as u8,
                        (sum[3] / cnt) as u8,
                    ];
                    for y in by..ye {
                        for x in bx..xe {
                            if selection.map_or(true, |s| s.contains(x, y)) {
                                let i = ((y * w + x) * 4) as usize;
                                out[i..i + 4].copy_from_slice(&avg);
                            }
                        }
                    }
                }
            }
        }
        EffectSpec::Bulge { amount } => {
            let cx = w as f32 * 0.5;
            let cy = h as f32 * 0.5;
            let r_max = cx.min(cy).max(1.0);
            let amt = amount as f32 / 100.0;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let dx = (x as f32 - cx) / r_max;
                        let dy = (y as f32 - cy) / r_max;
                        let r = (dx * dx + dy * dy).sqrt();
                        if r < 1.0 {
                            let factor = (1.0 - amt * (1.0 - r * r)).max(0.05);
                            let sx = cx + dx * factor * r_max;
                            let sy = cy + dy * factor * r_max;
                            let p = sample_bilinear(src, w, h, sx, sy);
                            let i = ((y * w + x) * 4) as usize;
                            out[i..i + 4].copy_from_slice(&p);
                        }
                    }
                }
            }
        }
        EffectSpec::Twist { amount, size } => {
            let cx = w as f32 * 0.5;
            let cy = h as f32 * 0.5;
            let r_max = (cx.min(cy) * (size as f32 / 100.0)).max(1.0);
            let twist_rad = (amount as f32).to_radians() * 2.5;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let dx = x as f32 - cx;
                        let dy = y as f32 - cy;
                        let r = (dx * dx + dy * dy).sqrt();
                        if r < r_max {
                            let t = 1.0 - r / r_max;
                            let a = twist_rad * t * t;
                            let (sin_a, cos_a) = a.sin_cos();
                            let sx = cx + dx * cos_a - dy * sin_a;
                            let sy = cy + dx * sin_a + dy * cos_a;
                            let p = sample_bilinear(src, w, h, sx, sy);
                            let i = ((y * w + x) * 4) as usize;
                            out[i..i + 4].copy_from_slice(&p);
                        }
                    }
                }
            }
        }
        EffectSpec::FrostedGlass { radius } => {
            let r = radius.clamp(1, 15);
            let span = (r * 2 + 1) as u32;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let h1 = hash_u32(x.wrapping_mul(73856093) ^ y.wrapping_mul(19349663));
                        let h2 = hash_u32(h1 ^ 0x9e3779b9);
                        let dx = (h1 % span) as i32 - r;
                        let dy = (h2 % span) as i32 - r;
                        let p = get_px(src, w, h, x as i32 + dx, y as i32 + dy);
                        let i = ((y * w + x) * 4) as usize;
                        out[i..i + 4].copy_from_slice(&p);
                    }
                }
            }
        }
        EffectSpec::AddNoise {
            intensity,
            saturation,
            coverage,
        } => {
            let amp = intensity.clamp(0, 100) as f32 * 2.55;
            let sat = saturation.clamp(0, 200) as f32 / 100.0;
            let cov = (coverage.clamp(0.0, 100.0) / 100.0 * 65535.0) as u32;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |s| s.contains(x, y)) {
                        let seed = x.wrapping_mul(374761393) ^ y.wrapping_mul(668265263);
                        let h0 = hash_u32(seed);
                        if (h0 & 0xFFFF) > cov {
                            continue;
                        }
                        let n1 = ((hash_u32(h0.wrapping_add(1)) & 0xFFFF) as f32 / 32767.5 - 1.0) * amp;
                        let n2 = ((hash_u32(h0.wrapping_add(2)) & 0xFFFF) as f32 / 32767.5 - 1.0) * amp;
                        let n3 = ((hash_u32(h0.wrapping_add(3)) & 0xFFFF) as f32 / 32767.5 - 1.0) * amp;
                        let mono = (n1 + n2 + n3) / 3.0;
                        let dr = mono + (n1 - mono) * sat;
                        let dg = mono + (n2 - mono) * sat;
                        let db = mono + (n3 - mono) * sat;
                        let i = ((y * w + x) * 4) as usize;
                        out[i] = clamp_u8(src[i] as f32 + dr);
                        out[i + 1] = clamp_u8(src[i + 1] as f32 + dg);
                        out[i + 2] = clamp_u8(src[i + 2] as f32 + db);
                    }
                }
            }
        }
        EffectSpec::EdgeDetect { angle } => {
            let rad = angle.to_radians();
            let (s, c) = rad.sin_cos();
            let k = [
                [c + s, s, -c + s],
                [c, 0.0, -c],
                [c - s, -s, -c - s],
            ];
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |sel| sel.contains(x, y)) {
                        let mut acc = [0.0f32; 3];
                        for ky in -1..=1 {
                            for kx in -1..=1 {
                                let p = get_px(src, w, h, x as i32 + kx, y as i32 + ky);
                                let kw = k[(ky + 1) as usize][(kx + 1) as usize];
                                for ch in 0..3 {
                                    acc[ch] += p[ch] as f32 * kw;
                                }
                            }
                        }
                        let i = ((y * w + x) * 4) as usize;
                        for ch in 0..3 {
                            out[i + ch] = clamp_u8(acc[ch].abs());
                        }
                    }
                }
            }
        }
        EffectSpec::Emboss { angle } | EffectSpec::Relief { angle } => {
            let is_relief = matches!(spec, EffectSpec::Relief { .. });
            let rad = angle.to_radians();
            let dx = rad.cos();
            let dy = rad.sin();
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |sel| sel.contains(x, y)) {
                        let p1 = get_px(src, w, h, x as i32 + dx.round() as i32, y as i32 + dy.round() as i32);
                        let p2 = get_px(src, w, h, x as i32 - dx.round() as i32, y as i32 - dy.round() as i32);
                        let i = ((y * w + x) * 4) as usize;
                        if is_relief {
                            for ch in 0..3 {
                                let diff = (p1[ch] as f32 - p2[ch] as f32) * 0.8;
                                out[i + ch] = clamp_u8(src[i + ch] as f32 + diff);
                            }
                        } else {
                            let l1 = 0.299 * p1[0] as f32 + 0.587 * p1[1] as f32 + 0.114 * p1[2] as f32;
                            let l2 = 0.299 * p2[0] as f32 + 0.587 * p2[1] as f32 + 0.114 * p2[2] as f32;
                            let v = clamp_u8(128.0 + (l1 - l2));
                            out[i] = v;
                            out[i + 1] = v;
                            out[i + 2] = v;
                        }
                    }
                }
            }
        }
        EffectSpec::OilPainting {
            brush_size,
            coarseness,
        } => {
            let r = brush_size.clamp(1, 5);
            let bins = coarseness.clamp(4, 64) as usize;
            let mut count = vec![0u32; bins];
            let mut sum_r = vec![0u32; bins];
            let mut sum_g = vec![0u32; bins];
            let mut sum_b = vec![0u32; bins];
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |sel| sel.contains(x, y)) {
                        count.fill(0);
                        sum_r.fill(0);
                        sum_g.fill(0);
                        sum_b.fill(0);
                        let mut best_bin = 0usize;
                        let mut best_cnt = 0u32;
                        for dy in -r..=r {
                            for dx in -r..=r {
                                let p = get_px(src, w, h, x as i32 + dx, y as i32 + dy);
                                let lum = ((p[0] as u32 + p[1] as u32 + p[2] as u32) * (bins as u32 - 1)) / 765;
                                let b = (lum as usize).min(bins - 1);
                                count[b] += 1;
                                sum_r[b] += p[0] as u32;
                                sum_g[b] += p[1] as u32;
                                sum_b[b] += p[2] as u32;
                                if count[b] > best_cnt {
                                    best_cnt = count[b];
                                    best_bin = b;
                                }
                            }
                        }
                        let i = ((y * w + x) * 4) as usize;
                        let c = best_cnt.max(1);
                        out[i] = (sum_r[best_bin] / c) as u8;
                        out[i + 1] = (sum_g[best_bin] / c) as u8;
                        out[i + 2] = (sum_b[best_bin] / c) as u8;
                    }
                }
            }
        }
        EffectSpec::PencilSketch { pencil_tip_size } => {
            // Color-dodge grayscale with blurred inverted grayscale
            let mut gray = vec![0u8; src.len()];
            for i in (0..src.len()).step_by(4) {
                let g = clamp_u8(0.299 * src[i] as f32 + 0.587 * src[i + 1] as f32 + 0.114 * src[i + 2] as f32);
                gray[i] = 255 - g;
                gray[i + 1] = 255 - g;
                gray[i + 2] = 255 - g;
                gray[i + 3] = src[i + 3];
            }
            let blurred = gaussian_blur_rgba(&gray, w, h, pencil_tip_size.clamp(1, 20));
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |sel| sel.contains(x, y)) {
                        let i = ((y * w + x) * 4) as usize;
                        let base = (255 - gray[i]) as f32 / 255.0;
                        let top = blurred[i] as f32 / 255.0;
                        let dodge = if top >= 0.996 {
                            1.0
                        } else {
                            (base / (1.0 - top)).min(1.0)
                        };
                        let v = clamp_u8(dodge * 255.0);
                        out[i] = v;
                        out[i + 1] = v;
                        out[i + 2] = v;
                    }
                }
            }
        }
        EffectSpec::Clouds { scale, power } => {
            let freq = 4.0 / scale.clamp(10, 500) as f32;
            let rough = (power.clamp(0, 100) as f32 / 100.0).clamp(0.15, 0.9);
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |sel| sel.contains(x, y)) {
                        let mut val = 0.0f32;
                        let mut amp = 1.0f32;
                        let mut norm = 0.0f32;
                        let mut f = freq;
                        for oct in 0..5u32 {
                            val += value_noise(x as f32 * f, y as f32 * f, 1337 + oct * 97) * amp;
                            norm += amp;
                            amp *= rough;
                            f *= 2.0;
                        }
                        let t = (val / norm).clamp(0.0, 1.0);
                        let i = ((y * w + x) * 4) as usize;
                        for ch in 0..3 {
                            out[i + ch] = clamp_u8(
                                primary[ch] as f32 * t + secondary[ch] as f32 * (1.0 - t),
                            );
                        }
                        out[i + 3] = 255;
                    }
                }
            }
        }
        EffectSpec::MandelbrotFractal { zoom } => {
            let scale = 3.0 / (zoom.max(0.5) * 0.25);
            let max_iter = 80u32;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |sel| sel.contains(x, y)) {
                        let cr = (x as f32 / w as f32 - 0.5) * scale * (w as f32 / h as f32) - 0.5;
                        let ci = (y as f32 / h as f32 - 0.5) * scale;
                        let mut zr = 0.0f32;
                        let mut zi = 0.0f32;
                        let mut iter = 0u32;
                        while zr * zr + zi * zi <= 4.0 && iter < max_iter {
                            let nr = zr * zr - zi * zi + cr;
                            zi = 2.0 * zr * zi + ci;
                            zr = nr;
                            iter += 1;
                        }
                        let i = ((y * w + x) * 4) as usize;
                        if iter == max_iter {
                            out[i..i + 4].copy_from_slice(&[12, 14, 24, 255]);
                        } else {
                            let t = iter as f32 / max_iter as f32;
                            let (r, g, b) = hsl_to_rgb((0.62 + t * 2.5).fract(), 0.85, 0.2 + t * 0.65);
                            out[i] = clamp_u8(r * 255.0);
                            out[i + 1] = clamp_u8(g * 255.0);
                            out[i + 2] = clamp_u8(b * 255.0);
                            out[i + 3] = 255;
                        }
                    }
                }
            }
        }
        EffectSpec::JuliaFractal { zoom } => {
            let scale = 2.6 / zoom.max(0.3);
            let cr = -0.7269f32;
            let ci = 0.1889f32;
            let max_iter = 80u32;
            for y in 0..h {
                for x in 0..w {
                    if selection.map_or(true, |sel| sel.contains(x, y)) {
                        let mut zr = (x as f32 / w as f32 - 0.5) * scale * (w as f32 / h as f32);
                        let mut zi = (y as f32 / h as f32 - 0.5) * scale;
                        let mut iter = 0u32;
                        while zr * zr + zi * zi <= 4.0 && iter < max_iter {
                            let nr = zr * zr - zi * zi + cr;
                            zi = 2.0 * zr * zi + ci;
                            zr = nr;
                            iter += 1;
                        }
                        let i = ((y * w + x) * 4) as usize;
                        if iter == max_iter {
                            out[i..i + 4].copy_from_slice(&[16, 12, 28, 255]);
                        } else {
                            let t = iter as f32 / max_iter as f32;
                            let (r, g, b) = hsl_to_rgb((0.78 + t * 2.2).fract(), 0.8, 0.22 + t * 0.62);
                            out[i] = clamp_u8(r * 255.0);
                            out[i + 1] = clamp_u8(g * 255.0);
                            out[i + 2] = clamp_u8(b * 255.0);
                            out[i + 3] = 255;
                        }
                    }
                }
            }
        }
    }

    out
}
