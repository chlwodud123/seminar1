use eframe::egui::Color32;

/// Maximum number of recent colors (Pinta.Core.PaletteManager::.ctor RVA 0x1d690).
pub const MAX_RECENT_COLORS: usize = 10;

/// Exact 34 default palette colors extracted from
/// `Pinta.Core.PaletteHelper::<EnumerateDefaultColors>d__5::MoveNext` (RVA 0x27444).
/// Ordered in 17 pairs: top row (even index) & bottom row (odd index).
pub const DEFAULT_PALETTE: [[u8; 4]; 34] = [
    [255, 255, 255, 255], // White
    [0, 0, 0, 255],       // Black
    [160, 160, 160, 255], // Light Gray
    [128, 128, 128, 255], // Gray
    [64, 64, 64, 255],    // Dark Gray
    [48, 48, 48, 255],    // Charcoal
    [255, 0, 0, 255],     // Red
    [255, 127, 127, 255], // Pink
    [255, 106, 0, 255],   // Orange
    [255, 178, 127, 255], // Peach
    [255, 216, 0, 255],   // Yellow
    [255, 233, 127, 255], // Light Yellow
    [182, 255, 0, 255],   // Lime
    [218, 255, 127, 255], // Light Lime
    [76, 255, 0, 255],    // Green
    [165, 255, 127, 255], // Light Green
    [0, 255, 33, 255],    // Spring
    [127, 255, 142, 255], // Mint
    [0, 255, 144, 255],   // Seafoam
    [127, 255, 197, 255], // Light Seafoam
    [0, 255, 255, 255],   // Cyan
    [127, 255, 255, 255], // Light Cyan
    [0, 148, 255, 255],   // Sky
    [127, 201, 255, 255], // Light Sky
    [0, 38, 255, 255],    // Blue
    [127, 146, 255, 255], // Periwinkle
    [72, 0, 255, 255],    // Indigo
    [161, 127, 255, 255], // Lavender
    [178, 0, 255, 255],   // Violet
    [214, 127, 255, 255], // Light Violet
    [255, 0, 220, 255],   // Magenta
    [255, 127, 237, 255], // Light Magenta
    [255, 0, 110, 255],   // Crimson
    [255, 127, 182, 255], // Rose
];

#[derive(Clone, Debug)]
pub struct PaletteState {
    pub primary: [u8; 4],
    pub secondary: [u8; 4],
    pub colors: Vec<[u8; 4]>,
    pub recent: Vec<[u8; 4]>,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self {
            primary: [0, 0, 0, 255],
            secondary: [255, 255, 255, 255],
            colors: DEFAULT_PALETTE.to_vec(),
            recent: vec![
                [0, 0, 0, 255],
                [255, 255, 255, 255],
                [255, 0, 0, 255],
                [0, 148, 255, 255],
                [76, 255, 0, 255],
                [255, 216, 0, 255],
            ],
        }
    }
}

impl PaletteState {
    pub fn set_primary(&mut self, color: [u8; 4]) {
        self.primary = color;
        self.push_recent(color);
    }

    pub fn set_secondary(&mut self, color: [u8; 4]) {
        self.secondary = color;
        self.push_recent(color);
    }

    pub fn swap(&mut self) {
        std::mem::swap(&mut self.primary, &mut self.secondary);
    }

    pub fn reset(&mut self) {
        self.primary = [0, 0, 0, 255];
        self.secondary = [255, 255, 255, 255];
    }

    pub fn push_recent(&mut self, color: [u8; 4]) {
        if let Some(pos) = self.recent.iter().position(|c| *c == color) {
            self.recent.remove(pos);
        }
        self.recent.insert(0, color);
        self.recent.truncate(MAX_RECENT_COLORS);
    }
}

#[inline]
pub fn rgba_to_color32(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

#[inline]
pub fn color32_to_rgba(c: Color32) -> [u8; 4] {
    c.to_srgba_unmultiplied()
}
