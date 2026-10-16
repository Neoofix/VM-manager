/// Paramètres : emplacement du fichier de données, export / import
use std::path::Path;

use egui::{RichText, Ui};
use vm_core::Store;

use super::widgets::*;
use crate::app::{Command, Confirm, UiState};
use crate::paths::{DataSource, DATA_ENV};
use crate::theme::*;

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
            .stroke(egui::Stroke::new(1.0, WARNING.linear_multiply(0.6)))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.label(
                    RichText::new(format!(
                        "⚠ Le fichier de données était illisible au démarrage. Il a été mis de côté sous {} et un inventaire vide a été créé.",
                        backup.display()
                    ))
                    .color(WARNING),
                );
            });
        ui.add_space(4.0);
    }

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("📄  Fichier de données").strong().color(TEXT_STRONG));
        ui.label(
            RichText::new("Tout l'inventaire est enregistré localement dans un fichier JSON, à chaque modification.")
                .color(TEXT_SECONDARY),
        );
        let shown = std::path::absolute(data_path).unwrap_or_else(|_| data_path.to_path_buf());
        let shown = shown.display().to_string();
        inset_frame().fill(BG_INPUT).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(RichText::new(&shown).monospace().color(TEXT_STRONG)).truncate());
                copy_button(ui, &shown);
            });
        });
        ui.label(RichText::new(format!("Source : {}", source.label())).small().color(TEXT_SECONDARY));
        ui.label(
            RichText::new(format!(
                "Par défaut le fichier est créé à côté de l'exécutable. Pour en utiliser un autre : \
                 vm-manager --data <fichier>  ou la variable {DATA_ENV}."
            ))
            .small()
            .color(TEXT_MUTED),
        );
    });
    ui.add_space(4.0);

    card_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("Export / import").strong().color(TEXT_STRONG));
        ui.label(
            RichText::new(
                "L'import en fusion ajoute les groupes et VM absents (même nom ou même identifiant = ignoré). \
                 Le remplacement efface l'inventaire actuel.",
            )
            .color(TEXT_SECONDARY),
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
        ui.label(RichText::new("À propos").strong().color(TEXT_STRONG));
        ui.label(
            RichText::new(format!(
                "VM Manager {} — mémo d'inventaire purement informatif : aucune action n'est effectuée sur les VM ni sur l'hyperviseur.",
                env!("CARGO_PKG_VERSION")
            ))
            .color(TEXT_SECONDARY),
        );
        ui.label(
            RichText::new("Le fichier JSON n'est pas chiffré : n'y note jamais de mot de passe ni de clé privée, seulement où les trouver.")
                .color(WARNING),
        );
    });
}
