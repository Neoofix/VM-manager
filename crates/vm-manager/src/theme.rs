/// Thème One Half Dark (même palette que Server Power Manager)
use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

pub const BG_PRIMARY: Color32 = Color32::from_rgb(0x28, 0x2c, 0x34);
pub const BG_SECONDARY: Color32 = Color32::from_rgb(0x21, 0x25, 0x2b);
pub const BG_TERTIARY: Color32 = Color32::from_rgb(0x2c, 0x31, 0x3a);
pub const BG_INPUT: Color32 = Color32::from_rgb(0x1e, 0x22, 0x27);
pub const BG_HOVER: Color32 = Color32::from_rgb(0x32, 0x38, 0x42);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xab, 0xb2, 0xbf);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x7f, 0x84, 0x8e);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x5c, 0x63, 0x70);
pub const TEXT_STRONG: Color32 = Color32::from_rgb(0xd7, 0xda, 0xe0);
pub const ACCENT: Color32 = Color32::from_rgb(0x61, 0xaf, 0xef);
pub const ACCENT_2: Color32 = Color32::from_rgb(0x56, 0xb6, 0xc2);
pub const SUCCESS: Color32 = Color32::from_rgb(0x98, 0xc3, 0x79);
pub const WARNING: Color32 = Color32::from_rgb(0xe5, 0xc0, 0x7b);
pub const ERROR: Color32 = Color32::from_rgb(0xe0, 0x6c, 0x75);
pub const BORDER: Color32 = Color32::from_rgb(0x3e, 0x44, 0x51);

pub const RADIUS: u8 = 8;
pub const RADIUS_LG: u8 = 12;

/// Couleurs proposées pour les groupes
pub const GROUP_COLORS: [&str; 8] = [
    "#61afef", "#56b6c2", "#98c379", "#e5c07b", "#d19a66", "#e06c75", "#c678dd", "#abb2bf",
];

/// `#rrggbb` → couleur ; gris atténué si absent ou invalide
pub fn parse_hex(hex: Option<&str>) -> Color32 {
    hex.and_then(|h| h.strip_prefix('#'))
        .filter(|h| h.len() == 6)
        .and_then(|h| u32::from_str_radix(h, 16).ok())
        .map(|v| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
        .unwrap_or(TEXT_MUTED)
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (
                TextStyle::Heading,
                FontId::new(22.0, FontFamily::Proportional),
            ),
            (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
            (
                TextStyle::Button,
                FontId::new(14.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Small,
                FontId::new(12.0, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(13.0, FontFamily::Monospace),
            ),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.window_margin = Margin::same(20);

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.override_text_color = None;
        // Textes d'exemple des champs nettement plus pâles que la saisie
        v.weak_text_color = Some(TEXT_MUTED);
        v.panel_fill = BG_PRIMARY;
        v.window_fill = BG_TERTIARY;
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.window_corner_radius = CornerRadius::same(RADIUS_LG);
        v.menu_corner_radius = CornerRadius::same(RADIUS);
        v.extreme_bg_color = BG_INPUT;
        v.faint_bg_color = BG_SECONDARY;
        v.code_bg_color = BG_INPUT;
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = WARNING;
        v.error_fg_color = ERROR;
        v.selection.bg_fill = ACCENT.linear_multiply(0.45);
        v.selection.stroke = Stroke::new(1.0, ACCENT);

        let radius = CornerRadius::same(RADIUS);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = BG_TERTIARY;
        w.noninteractive.weak_bg_fill = BG_TERTIARY;
        w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
        w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        w.noninteractive.corner_radius = radius;

        w.inactive.bg_fill = BG_INPUT;
        w.inactive.weak_bg_fill = BG_SECONDARY;
        w.inactive.bg_stroke = Stroke::new(1.0, BORDER);
        w.inactive.fg_stroke = Stroke::new(1.0, TEXT_PRIMARY);
        w.inactive.corner_radius = radius;

        w.hovered.bg_fill = BG_HOVER;
        w.hovered.weak_bg_fill = BG_HOVER;
        w.hovered.bg_stroke = Stroke::new(1.0, ACCENT.linear_multiply(0.6));
        w.hovered.fg_stroke = Stroke::new(1.5, TEXT_STRONG);
        w.hovered.corner_radius = radius;
        w.hovered.expansion = 0.0;

        w.active.bg_fill = BG_HOVER;
        w.active.weak_bg_fill = BG_HOVER;
        w.active.bg_stroke = Stroke::new(1.0, ACCENT);
        w.active.fg_stroke = Stroke::new(1.5, TEXT_STRONG);
        w.active.corner_radius = radius;
        w.active.expansion = 0.0;

        w.open.bg_fill = BG_HOVER;
        w.open.weak_bg_fill = BG_HOVER;
        w.open.bg_stroke = Stroke::new(1.0, ACCENT);
        w.open.corner_radius = radius;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors() {
        assert_eq!(parse_hex(Some("#61afef")), ACCENT);
        assert_eq!(parse_hex(Some("nope")), TEXT_MUTED);
        assert_eq!(parse_hex(None), TEXT_MUTED);
        for c in GROUP_COLORS {
            assert_ne!(parse_hex(Some(c)), TEXT_MUTED, "{c}");
        }
    }
}
