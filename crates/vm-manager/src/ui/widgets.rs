/// Briques d'interface réutilisées par tous les écrans
use egui::{
    Button, Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, TextEdit, Ui, Vec2,
};

use crate::theme::*;

pub fn card_frame() -> Frame {
    Frame::new()
        .fill(p().bg_tertiary)
        .stroke(Stroke::new(1.0, p().border))
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(Margin::same(16))
}

pub fn inset_frame() -> Frame {
    Frame::new()
        .fill(p().bg_secondary)
        .stroke(Stroke::new(1.0, p().border))
        .corner_radius(CornerRadius::same(RADIUS))
        .inner_margin(Margin::symmetric(12, 10))
}

/// Carte cliquable : le clic n'est capté que là où aucun widget enfant ne l'est
pub fn clickable_card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> egui::Response {
    ui.scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
        let hovered = ui.response().hovered();
        card_frame()
            .stroke(Stroke::new(
                1.0,
                if hovered { p().accent_fg } else { p().border },
            ))
            .show(ui, add);
    })
    .response
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn primary_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(
        Button::new(RichText::new(text).color(Color32::WHITE).strong())
            .fill(p().accent)
            .stroke(Stroke::NONE),
    )
}

pub fn ghost_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(Button::new(text).fill(p().bg_secondary))
}

pub fn danger_button(ui: &mut Ui, text: &str) -> egui::Response {
    ui.add(
        Button::new(RichText::new(text).color(Color32::WHITE).strong())
            .fill(p().error.linear_multiply(0.85))
            .stroke(Stroke::NONE),
    )
}

/// Bouton discret (icône seule) avec infobulle
pub fn icon_button(ui: &mut Ui, icon: &str, tooltip: &str) -> egui::Response {
    ui.add(Button::new(RichText::new(icon).color(p().text_secondary)).frame(false))
        .on_hover_text(tooltip)
}

pub fn copy_button(ui: &mut Ui, value: &str) {
    if icon_button(ui, "🗐", "Copier").clicked() {
        ui.ctx().copy_text(value.to_string());
    }
}

pub fn chip(ui: &mut Ui, text: impl Into<String>, color: Color32) {
    Frame::new()
        .stroke(Stroke::new(1.0, color.linear_multiply(0.5)))
        .corner_radius(CornerRadius::same(10))
        .inner_margin(Margin::symmetric(7, 1))
        .show(ui, |ui| {
            ui.label(RichText::new(text.into()).small().color(color));
        });
}

pub fn color_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 5.0, color);
}

pub fn page_header(ui: &mut Ui, title: &str, subtitle: &str, actions: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .heading()
                    .strong()
                    .color(p().text_strong),
            );
            ui.label(RichText::new(subtitle).color(p().text_secondary));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), actions);
    });
    ui.add_space(8.0);
}

pub fn section_title(ui: &mut Ui, text: &str) {
    ui.label(
        RichText::new(text.to_uppercase())
            .small()
            .strong()
            .color(p().text_muted),
    );
}

/// Libellé au-dessus d'un champ, avec indication facultative
pub fn field_label(ui: &mut Ui, label: &str, hint: Option<&str>) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.label(
            RichText::new(label)
                .small()
                .strong()
                .color(p().text_secondary),
        );
        if let Some(hint) = hint {
            ui.label(
                RichText::new(format!("— {hint}"))
                    .small()
                    .color(p().text_muted),
            );
        }
    });
}

pub fn text_field(
    ui: &mut Ui,
    value: &mut String,
    placeholder: &str,
    mono: bool,
) -> egui::Response {
    let mut edit = TextEdit::singleline(value)
        .hint_text(placeholder)
        .desired_width(f32::INFINITY)
        .margin(Margin::symmetric(8, 6));
    if mono {
        edit = edit.font(egui::TextStyle::Monospace);
    }
    ui.add(edit)
}

pub fn text_area(ui: &mut Ui, value: &mut String, placeholder: &str, rows: usize, mono: bool) {
    let mut edit = TextEdit::multiline(value)
        .hint_text(placeholder)
        .desired_width(f32::INFINITY)
        .desired_rows(rows)
        .margin(Margin::symmetric(8, 6));
    if mono {
        edit = edit.font(egui::TextStyle::Monospace);
    }
    ui.add(edit);
}

/// Champ libellé + saisie
pub fn labeled(ui: &mut Ui, label: &str, hint: Option<&str>, add: impl FnOnce(&mut Ui)) {
    ui.vertical(|ui| {
        field_label(ui, label, hint);
        add(ui);
    });
}

/// Ligne « libellé : valeur » des fiches, avec bouton de copie facultatif
pub fn info_row(ui: &mut Ui, label: &str, value: &str, copy: bool, mono: bool) {
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(110.0, 18.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.set_width(110.0);
                ui.label(RichText::new(label).color(p().text_secondary));
            },
        );
        if value.is_empty() {
            ui.label(RichText::new("—").color(p().text_muted));
        } else {
            let mut text = RichText::new(value).color(p().text_strong);
            if mono {
                text = text.monospace();
            }
            ui.add(egui::Label::new(text).truncate());
            if copy {
                copy_button(ui, value);
            }
        }
    });
}

pub fn error_text(ui: &mut Ui, error: &Option<String>) {
    if let Some(e) = error {
        ui.add_space(4.0);
        ui.label(RichText::new(format!("⚠ {e}")).color(p().error));
    }
}

/// État vide centré ; renvoie `true` si le bouton est cliqué
pub fn empty_state(ui: &mut Ui, icon: &str, title: &str, hint: &str, button: &str) -> bool {
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.label(RichText::new(icon).size(36.0).color(p().text_secondary));
        ui.add_space(6.0);
        ui.label(RichText::new(title).strong().color(p().text_strong));
        ui.label(RichText::new(hint).color(p().text_secondary));
        ui.add_space(8.0);
        clicked = primary_button(ui, button).clicked();
    });
    clicked
}

/// Fenêtre modale à titre, contenu défilant et pied de boutons.
/// Renvoie `true` si l'utilisateur la ferme (Échap, clic sur le fond, ✕).
pub fn modal(
    ui: &Ui,
    id: &str,
    title: &str,
    subtitle: Option<&str>,
    width: f32,
    body: impl FnOnce(&mut Ui),
    footer: impl FnOnce(&mut Ui),
) -> bool {
    let mut close = false;
    let screen = ui.ctx().content_rect();
    let response = egui::Modal::new(egui::Id::unique(id))
        .backdrop_color(Color32::from_black_alpha(150))
        .frame(
            Frame::new()
                .fill(p().bg_tertiary)
                .stroke(Stroke::new(1.0, p().border))
                .corner_radius(CornerRadius::same(RADIUS_LG))
                .inner_margin(Margin::same(0)),
        )
        .show(ui.ctx(), |ui| {
            ui.set_width(width.min(screen.width() - 48.0));
            Frame::new()
                .inner_margin(Margin::symmetric(24, 18))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(title)
                                    .size(17.0)
                                    .strong()
                                    .color(p().text_strong),
                            );
                            if let Some(sub) = subtitle {
                                ui.label(RichText::new(sub).small().color(p().text_secondary));
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            if icon_button(ui, "✖", "Fermer").clicked() {
                                close = true;
                            }
                        });
                    });
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(24, 12))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height((screen.height() - 220.0).max(200.0))
                        .auto_shrink([false, true])
                        .show(ui, body);
                });
            ui.separator();
            Frame::new()
                .inner_margin(Margin::symmetric(24, 14))
                .show(ui, |ui| {
                    ui.horizontal(footer);
                });
        });
    close || response.should_close()
}

/// Donne le focus clavier au champ à l'ouverture d'un formulaire (une seule fois)
pub fn autofocus(response: &egui::Response, focused: &mut bool) {
    if !*focused {
        response.request_focus();
        response.scroll_to_me(Some(egui::Align::Center));
        *focused = true;
    }
}
