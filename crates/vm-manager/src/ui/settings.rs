/// Paramètres : emplacement du fichier de données, export / import
use std::path::Path;

use egui::{RichText, Ui};
use vm_core::Store;

use super::widgets::*;
use crate::app::{Command, Confirm, UiState};
use crate::paths::{DataSource, DATA_ENV};
use crate::theme::{self, *};

pub fn show(ui: &mut Ui, store: &Store, data_path: &Path, source: DataSource, st: &mut UiState) {
    page_header(
        ui,
        "Paramètres",
        "Stockage et sauvegarde de l'inventaire",
        |_| {},
    );
    ui.set_max_width(820.0);

    if let Some(backup) = store.recovered_backup() {
        card_frame()
            .stroke(egui::Stroke::new(1.0, p().warning.linear_multiply(0.6)))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(format!(
                        "⚠ Le fichier de données était illisible au démarrage. Il a été mis de côté sous {} et un inventaire vide a été créé.",
                        backup.display()
                    ))
                    .color(p().warning),
                );
            });
        ui.add_space(4.0);
    }

    appearance(ui, st);
    ui.add_space(4.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("📄  Fichier de données").strong().color(p().text_strong));
        ui.label(
            RichText::new("Tout l'inventaire est enregistré localement dans un fichier JSON, à chaque modification.")
                .color(p().text_secondary),
        );
        let shown = std::path::absolute(data_path).unwrap_or_else(|_| data_path.to_path_buf());
        let shown = shown.display().to_string();
        inset_frame().fill(p().bg_input).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(&shown).monospace().color(p().text_strong)).truncate());
                copy_button(ui, &shown);
            });
        });
        ui.label(RichText::new(format!("Source : {}", source.label())).small().color(p().text_secondary));
        ui.label(
            RichText::new(format!(
                "Par défaut le fichier est créé à côté de l'exécutable. Pour en utiliser un autre : \
                 vm-manager --data <fichier>  ou la variable {DATA_ENV}."
            ))
            .small()
            .color(p().text_muted),
        );
    });
    ui.add_space(4.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("Export / import").strong().color(p().text_strong));
        ui.label(
            RichText::new(
                "L'import en fusion ajoute les groupes et VM absents (même nom ou même identifiant = ignoré). \
                 Le remplacement efface l'inventaire actuel.",
            )
            .color(p().text_secondary),
        );
        ui.horizontal(|ui| {
            if primary_button(ui, "⬇  Exporter").clicked() {
                st.run(Command::Export);
            }
            if ghost_button(ui, "⬆  Importer (fusion)").clicked() {
                st.run(Command::Import { replace: false });
            }
            if ghost_button(ui, "⬆  Importer (remplacer)").clicked() {
                st.confirm = Some(Confirm {
                    title: "Remplacer l'inventaire".into(),
                    message: "Tout l'inventaire actuel sera remplacé par le fichier choisi.\nPense à exporter d'abord si besoin.".into(),
                    button: "Choisir le fichier",
                    command: Command::Import { replace: true },
                });
            }
        });
    });
    ui.add_space(4.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("À propos").strong().color(p().text_strong));
        ui.label(
            RichText::new(format!(
                "VM Manager {} — mémo d'inventaire purement informatif : aucune action n'est effectuée sur les VM ni sur l'hyperviseur.",
                env!("CARGO_PKG_VERSION")
            ))
            .color(p().text_secondary),
        );
        ui.label(
            RichText::new("Le fichier JSON n'est pas chiffré : n'y note jamais de mot de passe ni de clé privée, seulement où les trouver.")
                .color(p().warning),
        );
    });
}

const TILE_WIDTH: f32 = 190.0;

/// Choix du thème : une vignette d'aperçu par thème, appliqué dès le clic
fn appearance(ui: &mut Ui, st: &mut UiState) {
    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(
            RichText::new("🎨  Apparence")
                .strong()
                .color(p().text_strong),
        );
        ui.label(
            RichText::new(
                "Le thème s'applique immédiatement et est mémorisé pour les prochains lancements.",
            )
            .color(p().text_secondary),
        );
        ui.add_space(4.0);
        let active = theme::current().id;
        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
            |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                for (i, t) in THEMES.iter().enumerate() {
                    let tile = ui
                        .allocate_ui_with_layout(
                            egui::vec2(TILE_WIDTH + 24.0, 80.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| theme_tile(ui, t, t.id == active),
                        )
                        .inner;
                    if tile.clicked() && t.id != active {
                        st.run(Command::SetTheme(i));
                    }
                }
            },
        );
    });
}

fn theme_tile(ui: &mut Ui, t: &Theme, selected: bool) -> egui::Response {
    let stroke = if selected {
        egui::Stroke::new(2.0, p().accent_fg)
    } else {
        egui::Stroke::new(1.0, p().border)
    };
    ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| {
        egui::Frame::new()
            .fill(t.bg_primary)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(RADIUS))
            .inner_margin(egui::Margin::same(10))
            .show(ui, |ui| {
                ui.set_width(TILE_WIDTH);
                ui.set_max_width(TILE_WIDTH);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.name).strong().color(t.text));
                    if selected {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("✔").color(t.success));
                        });
                    }
                });
                // Bandeau secondaire + pastilles des couleurs du thème
                egui::Frame::new()
                    .fill(t.bg_secondary)
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(6, 5))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            for c in [t.accent, t.success, t.warning, t.error, t.text] {
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 16.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().circle_filled(rect.center(), 7.0, c);
                            }
                        });
                    });
            });
    })
    .response
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}
