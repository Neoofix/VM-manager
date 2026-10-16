/// Thèmes de couleurs, changeables à chaud depuis les Paramètres.
///
/// Chaque thème est défini par 7 couleurs de base (fonds, texte, accent, états) ;
/// les nuances intermédiaires (bordures, survol, texte secondaire…) en sont
/// dérivées pour que tous les thèmes restent cohérents.
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;

use egui::{Color32, CornerRadius, FontFamily, FontId, Margin, Stroke, TextStyle};

pub const RADIUS: u8 = 8;
pub const RADIUS_LG: u8 = 12;

/// Couleurs de base d'un thème
pub struct Theme {
    pub id: &'static str,
    pub name: &'static str,
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub text: Color32,
    pub accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
}

const fn hex(v: u32) -> Color32 {
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

#[rustfmt::skip]
pub const THEMES: [Theme; 7] = [
    Theme { id: "one-half-dark", name: "One Half Dark",
        bg_primary: hex(0x282c34), bg_secondary: hex(0x21252b), text: hex(0xabb2bf),
        accent: hex(0x61afef), success: hex(0x98c379), warning: hex(0xe5c07b), error: hex(0xe06c75) },
    Theme { id: "fluent-dark", name: "Fluent Dark",
        bg_primary: hex(0x0a0a0f), bg_secondary: hex(0x111117), text: hex(0xe2e8f0),
        accent: hex(0x0078d4), success: hex(0x16a34a), warning: hex(0xd97706), error: hex(0xdc2626) },
    Theme { id: "gruvbox-dark", name: "Gruvbox Dark",
        bg_primary: hex(0x282828), bg_secondary: hex(0x1d2021), text: hex(0xebdbb2),
        accent: hex(0x458588), success: hex(0xb8bb26), warning: hex(0xfabd2f), error: hex(0xfb4934) },
    Theme { id: "nord", name: "Nord",
        bg_primary: hex(0x2e3440), bg_secondary: hex(0x272c36), text: hex(0xeceff4),
        accent: hex(0x5e81ac), success: hex(0xa3be8c), warning: hex(0xebcb8b), error: hex(0xbf616a) },
    Theme { id: "dracula", name: "Dracula",
        bg_primary: hex(0x282a36), bg_secondary: hex(0x21222c), text: hex(0xf8f8f2),
        accent: hex(0x7c5fc9), success: hex(0x50fa7b), warning: hex(0xf1fa8c), error: hex(0xff5555) },
    Theme { id: "catppuccin-mocha", name: "Catppuccin Mocha",
        bg_primary: hex(0x1e1e2e), bg_secondary: hex(0x181825), text: hex(0xcdd6f4),
        accent: hex(0x5a7fd6), success: hex(0xa6e3a1), warning: hex(0xf9e2af), error: hex(0xf38ba8) },
    Theme { id: "tokyo-night", name: "Tokyo Night",
        bg_primary: hex(0x1a1b26), bg_secondary: hex(0x16161e), text: hex(0xc0caf5),
        accent: hex(0x3d59a1), success: hex(0x9ece6a), warning: hex(0xe0af68), error: hex(0xf7768e) },
];

/// Palette complète utilisée par l'interface
pub struct Palette {
    pub bg_primary: Color32,
    pub bg_secondary: Color32,
    pub bg_tertiary: Color32,
    pub bg_input: Color32,
    pub bg_hover: Color32,
    pub text: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub text_strong: Color32,
    /// Accent en aplat (boutons, sélection)
    pub accent: Color32,
    /// Accent utilisé comme couleur de texte ou d'icône sur fond sombre :
    /// éclairci pour rester lisible avec les accents foncés (Fluent, Tokyo Night)
    pub accent_fg: Color32,
    /// Couleur des étiquettes, entre l'accent et la couleur de succès
    pub accent_2: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub error: Color32,
    pub border: Color32,
}

/// Mélange linéaire de `a` vers `b` (t = 0 → a, t = 1 → b)
fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(m(a.r(), b.r()), m(a.g(), b.g()), m(a.b(), b.b()))
}

impl Palette {
    fn from_theme(t: &Theme) -> Palette {
        Palette {
            bg_primary: t.bg_primary,
            bg_secondary: t.bg_secondary,
            bg_tertiary: mix(t.bg_primary, t.text, 0.03),
            bg_input: mix(t.bg_secondary, Color32::BLACK, 0.15),
            bg_hover: mix(t.bg_primary, t.text, 0.08),
            text: t.text,
            text_secondary: mix(t.bg_primary, t.text, 0.62),
            text_muted: mix(t.bg_primary, t.text, 0.4),
            text_strong: mix(t.text, Color32::WHITE, 0.4),
            accent: t.accent,
            accent_fg: mix(t.accent, Color32::WHITE, 0.3),
            accent_2: mix(mix(t.accent, t.success, 0.5), Color32::WHITE, 0.2),
            success: t.success,
            warning: t.warning,
            error: t.error,
            border: mix(t.bg_primary, t.text, 0.16),
        }
    }
}

static PALETTES: LazyLock<Vec<Palette>> =
    LazyLock::new(|| THEMES.iter().map(Palette::from_theme).collect());
static CURRENT: AtomicUsize = AtomicUsize::new(0);

/// Palette du thème actif
pub fn p() -> &'static Palette {
    &PALETTES[CURRENT.load(Ordering::Relaxed)]
}

pub fn current() -> &'static Theme {
    &THEMES[CURRENT.load(Ordering::Relaxed)]
}

/// Index d'un thème à partir de son identifiant ; One Half Dark par défaut
pub fn index_of(id: &str) -> usize {
    THEMES.iter().position(|t| t.id == id).unwrap_or(0)
}

/// Active un thème et met à jour le style egui
pub fn select(ctx: &egui::Context, index: usize) {
    CURRENT.store(index.min(THEMES.len() - 1), Ordering::Relaxed);
    apply(ctx);
}

/// Couleurs proposées pour les groupes
pub const GROUP_COLORS: [&str; 8] = [
    "#61afef", "#56b6c2", "#98c379", "#e5c07b", "#d19a66", "#e06c75", "#c678dd", "#abb2bf",
];

/// `#rrggbb` → couleur ; gris atténué du thème si absent ou invalide
pub fn parse_hex(hex: Option<&str>) -> Color32 {
    hex.and_then(|h| h.strip_prefix('#'))
        .filter(|h| h.len() == 6)
        .and_then(|h| u32::from_str_radix(h, 16).ok())
        .map(|v| Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8))
        .unwrap_or(p().text_muted)
}

pub fn apply(ctx: &egui::Context) {
    let c = p();
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
        v.weak_text_color = Some(c.text_muted);
        v.panel_fill = c.bg_primary;
        v.window_fill = c.bg_tertiary;
        v.window_stroke = Stroke::new(1.0, c.border);
        v.window_corner_radius = CornerRadius::same(RADIUS_LG);
        v.menu_corner_radius = CornerRadius::same(RADIUS);
        v.extreme_bg_color = c.bg_input;
        v.faint_bg_color = c.bg_secondary;
        v.code_bg_color = c.bg_input;
        v.hyperlink_color = c.accent_fg;
        v.warn_fg_color = c.warning;
        v.error_fg_color = c.error;
        v.selection.bg_fill = c.accent.linear_multiply(0.45);
        v.selection.stroke = Stroke::new(1.0, c.accent_fg);

        let radius = CornerRadius::same(RADIUS);
        let w = &mut v.widgets;
        w.noninteractive.bg_fill = c.bg_tertiary;
        w.noninteractive.weak_bg_fill = c.bg_tertiary;
        w.noninteractive.bg_stroke = Stroke::new(1.0, c.border);
        w.noninteractive.fg_stroke = Stroke::new(1.0, c.text);
        w.noninteractive.corner_radius = radius;

        w.inactive.bg_fill = c.bg_input;
        w.inactive.weak_bg_fill = c.bg_secondary;
        w.inactive.bg_stroke = Stroke::new(1.0, c.border);
        w.inactive.fg_stroke = Stroke::new(1.0, c.text);
        w.inactive.corner_radius = radius;

        w.hovered.bg_fill = c.bg_hover;
        w.hovered.weak_bg_fill = c.bg_hover;
        w.hovered.bg_stroke = Stroke::new(1.0, c.accent_fg.linear_multiply(0.6));
        w.hovered.fg_stroke = Stroke::new(1.5, c.text_strong);
        w.hovered.corner_radius = radius;
        w.hovered.expansion = 0.0;

        w.active.bg_fill = c.bg_hover;
        w.active.weak_bg_fill = c.bg_hover;
        w.active.bg_stroke = Stroke::new(1.0, c.accent_fg);
        w.active.fg_stroke = Stroke::new(1.5, c.text_strong);
        w.active.corner_radius = radius;
        w.active.expansion = 0.0;

        w.open.bg_fill = c.bg_hover;
        w.open.weak_bg_fill = c.bg_hover;
        w.open.bg_stroke = Stroke::new(1.0, c.accent_fg);
        w.open.corner_radius = radius;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_colors() {
        assert_eq!(parse_hex(Some("#61afef")), hex(0x61afef));
        assert_eq!(parse_hex(Some("nope")), p().text_muted);
        assert_eq!(parse_hex(None), p().text_muted);
        for c in GROUP_COLORS {
            assert_ne!(parse_hex(Some(c)), p().text_muted, "{c}");
        }
    }

    #[test]
    fn theme_ids_are_unique_and_resolvable() {
        for (i, t) in THEMES.iter().enumerate() {
            assert_eq!(index_of(t.id), i);
        }
        assert_eq!(index_of("inconnu"), 0);
        assert_eq!(PALETTES.len(), THEMES.len());
    }

    #[test]
    fn derived_shades_sit_between_background_and_text() {
        for pal in PALETTES.iter() {
            let lum = |c: Color32| c.r() as u32 + c.g() as u32 + c.b() as u32;
            assert!(lum(pal.bg_primary) < lum(pal.border));
            assert!(lum(pal.border) < lum(pal.text_muted));
            assert!(lum(pal.text_muted) < lum(pal.text_secondary));
            assert!(lum(pal.text_secondary) < lum(pal.text));
        }
    }
}
