//! Couleurs de marque (`branding/palette.toml`, `[ui.dark]`) gardées là où
//! elles portent un sens : VU-mètres, étiquettes de strips, routes A/B, mute,
//! périphérique par défaut. Tout le reste (fenêtre, cartes, menus, texte)
//! suit le thème COSMIC du système, clair ou sombre.

use cosmic::iced::Color;

const fn hex(rgb: u32) -> Color {
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// Texte posé sur un aplat de couleur de marque.
pub const ON_ACCENT: Color = hex(0x2B2140);
pub const MIMI_PINK: Color = hex(0xFF8FB8);
pub const MIMI_LAVENDER: Color = hex(0xB9A3F5);
pub const CABLE: Color = hex(0x7FE0C2);
pub const HARDWARE: Color = hex(0x8CCBFF);
pub const VIRTUAL: Color = hex(0xB9A3F5);
pub const STAR: Color = hex(0xFFE07A);
pub const MUTE: Color = hex(0xFF5C7A);
pub const METER_BG: Color = hex(0x15111F);
pub const METER_LOW: Color = hex(0x7FE0A6);
pub const METER_MID: Color = hex(0xFFD86B);
pub const METER_HIGH: Color = hex(0xFF6B8B);

/// Couleur assombrie (VU-mètre d'un strip coupé).
pub fn dim(c: Color) -> Color {
    Color { a: 0.35, ..c }
}
