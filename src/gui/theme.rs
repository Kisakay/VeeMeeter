//! Thème de la fenêtre : couleurs pleines (pas de dégradés), coins arrondis.
//! Les valeurs suivent la palette `ui.dark` du kit de marque (`branding/palette.toml`).

use eframe::egui::{self, Color32, CornerRadius, Stroke, Visuals};

pub struct Theme {
    pub bg: Color32,
    pub panel: Color32,
    pub card: Color32,
    pub card_hi: Color32,
    pub border: Color32,
    pub selection: Color32,
    pub accent: Color32,
    pub accent2: Color32,
    pub text: Color32,
    pub muted: Color32,
    /// Texte posé sur un aplat accent / mute / bus.
    pub on_accent: Color32,
    pub meter_low: Color32,
    pub meter_mid: Color32,
    pub meter_high: Color32,
    pub meter_bg: Color32,
    pub mute: Color32,
    pub hw: Color32,
    pub virt: Color32,
    pub star: Color32,
}

const fn hex(rgb: u32) -> Color32 {
    Color32::from_rgb((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// `branding/palette.toml`, section `[ui.dark]` (contrastes WCAG vérifiés là-bas).
pub const THEME: Theme = Theme {
    bg: hex(0x1E1830),
    panel: hex(0x272040),
    card: hex(0x312852),
    card_hi: hex(0x3B3160),
    border: hex(0x4A3F73),
    selection: hex(0x5B3FB8),
    accent: hex(0xFF8FB8),
    accent2: hex(0xB9A3F5),
    text: hex(0xF7F1FF),
    muted: hex(0xB8ADD6),
    on_accent: hex(0x2B2140),
    meter_low: hex(0x7FE0A6),
    meter_mid: hex(0xFFD86B),
    meter_high: hex(0xFF6B8B),
    meter_bg: hex(0x15111F),
    mute: hex(0xFF5C7A),
    hw: hex(0x8CCBFF),
    virt: hex(0xB9A3F5),
    star: hex(0xFFE07A),
};

pub fn apply(ctx: &egui::Context) {
    let t = &THEME;
    let mut v = Visuals::dark();
    v.panel_fill = t.panel;
    v.window_fill = t.panel;
    v.extreme_bg_color = t.meter_bg;
    v.faint_bg_color = t.card;
    v.override_text_color = Some(t.text);
    v.hyperlink_color = t.accent;
    v.selection.bg_fill = t.selection;
    v.selection.stroke = Stroke::new(1.0, t.accent);
    v.window_corner_radius = CornerRadius::same(14);
    v.menu_corner_radius = CornerRadius::same(12);
    v.window_stroke = Stroke::new(1.0, t.border);
    let r = CornerRadius::same(9);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_fill = t.card;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, t.border);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, t.muted);
    v.widgets.inactive.bg_fill = t.card_hi;
    v.widgets.inactive.weak_bg_fill = t.card_hi;
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, t.text);
    v.widgets.hovered.bg_fill = t.border;
    v.widgets.hovered.weak_bg_fill = t.border;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, t.accent2);
    v.widgets.active.bg_fill = t.accent;
    v.widgets.active.weak_bg_fill = t.accent;
    v.widgets.active.fg_stroke = Stroke::new(1.5, t.on_accent);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 6.0);
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.spacing.slider_width = 200.0;
    });
}
