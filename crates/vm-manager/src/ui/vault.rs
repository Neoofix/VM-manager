/// Coffre des mots de passe : fenêtre du mot de passe maître et écran de
/// déverrouillage d'un inventaire entièrement chiffré
use std::path::Path;

use egui::{Frame, RichText, Ui};

use super::widgets::*;
use super::{titlebar, toasts};
use crate::app::{Command, UiState};
use crate::forms::{VaultForm, VaultMode};
use crate::theme::*;

/// Avertissement affiché partout où un mot de passe maître est choisi
pub const IRRECOVERABLE: &str = "Sans ce mot de passe, les mots de passe enregistrés (et l'inventaire s'il est chiffré) sont irrécupérables : aucune réinitialisation n'est possible. Note-le dans un endroit sûr.";

struct Texts {
    title: &'static str,
    intro: &'static str,
    button: &'static str,
}

fn texts(form: &VaultForm) -> Texts {
    match form.mode {
        VaultMode::Create => Texts {
            title: "Créer le coffre des mots de passe",
            intro: "Les mots de passe des utilisateurs seront chiffrés avec ce mot de passe maître (Argon2id + XChaCha20-Poly1305). Il n'est jamais enregistré.",
            button: "Créer le coffre",
        },
        VaultMode::Unlock => Texts {
            title: "Déverrouiller le coffre",
            intro: "Saisis le mot de passe maître pour afficher et modifier les mots de passe. Le coffre se reverrouille après 10 minutes d'inactivité.",
            button: "Déverrouiller",
        },
        VaultMode::ChangeMaster => Texts {
            title: "Changer le mot de passe maître",
            intro: "Tous les mots de passe enregistrés (et le fichier de données s'il est chiffré) sont rechiffrés avec le nouveau mot de passe.",
            button: "Changer le mot de passe",
        },
        VaultMode::EnableFileEncryption if form.new_vault => Texts {
            title: "Chiffrer tout le fichier de données",
            intro: "Tout l'inventaire (VM, adresses, utilisateurs, notes…) sera chiffré. Le mot de passe maître sera demandé à chaque lancement de l'application, et les exports seront chiffrés eux aussi. Le coffre des mots de passe est créé avec ce mot de passe.",
            button: "Chiffrer le fichier",
        },
        VaultMode::EnableFileEncryption => Texts {
            title: "Chiffrer tout le fichier de données",
            intro: "Tout l'inventaire (VM, adresses, utilisateurs, notes…) sera chiffré avec le mot de passe maître du coffre. Il sera demandé à chaque lancement de l'application, et les exports seront chiffrés eux aussi.",
            button: "Chiffrer le fichier",
        },
        VaultMode::DisableFileEncryption => Texts {
            title: "Désactiver le chiffrement du fichier",
            intro: "L'inventaire sera de nouveau enregistré en clair et s'ouvrira sans mot de passe. Les mots de passe des utilisateurs restent chiffrés par le coffre.",
            button: "Enregistrer en clair",
        },
        VaultMode::OpenFile => Texts {
            title: "Inventaire chiffré",
            intro: "Saisis le mot de passe maître pour ouvrir l'inventaire.",
            button: "Déverrouiller",
        },
        VaultMode::Import { .. } => Texts {
            title: "Importer un fichier chiffré",
            intro: "Ce fichier exporté est chiffré : saisis le mot de passe maître avec lequel il a été enregistré.",
            button: "Importer",
        },
    }
}

/// Champs du formulaire ; renvoie `true` si Entrée est pressée
fn fields(ui: &mut Ui, form: &mut VaultForm) -> bool {
    let (first_label, first_hint) = match form.mode {
        VaultMode::ChangeMaster => ("Mot de passe maître actuel", None),
        _ if form.creates_password() => ("Mot de passe maître", Some("8 caractères minimum")),
        _ => ("Mot de passe maître", None),
    };
    labeled(ui, first_label, first_hint, |ui| {
        let r = password_field(ui, &mut form.password, &mut form.reveal, "", 0.0);
        autofocus(&r, &mut form.focused);
    });
    if form.mode == VaultMode::ChangeMaster {
        labeled(
            ui,
            "Nouveau mot de passe maître",
            Some("8 caractères minimum"),
            |ui| {
                password_field(ui, &mut form.new_password, &mut form.reveal, "", 0.0);
            },
        );
    }
    if form.creates_password() {
        labeled(ui, "Confirmation", None, |ui| {
            password_field(ui, &mut form.confirm, &mut form.reveal, "", 0.0);
        });
    }
    ui.input(|i| i.key_pressed(egui::Key::Enter))
}

/// Fenêtre modale du coffre (création, déverrouillage, changement, chiffrement, import)
pub fn dialog(ui: &Ui, st: &mut UiState) {
    let Some(form) = &mut st.vault_form else {
        return;
    };
    let t = texts(form);
    let subtitle = match &form.mode {
        VaultMode::Import { path, .. } => Some(path.display().to_string()),
        _ => None,
    };
    let mut enter = false;
    let mut submit = false;
    let mut cancel = false;
    let closed = modal(
        ui,
        "vault",
        &format!("🔑  {}", t.title),
        subtitle.as_deref(),
        480.0,
        |ui| {
            ui.add(egui::Label::new(RichText::new(t.intro).color(p().text)).wrap());
            ui.add_space(4.0);
            if form.creates_password() {
                warning_box(ui, IRRECOVERABLE);
                ui.add_space(4.0);
            }
            enter = fields(ui, form);
            error_text(ui, &form.error);
        },
        |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, t.button).clicked() {
                    submit = true;
                }
                if ghost_button(ui, "Annuler").clicked() {
                    cancel = true;
                }
            });
        },
    );
    // Toujours au premier plan : la fiche de VM qui l'a ouverte (clic dans la même
    // image) ne doit pas la recouvrir
    ui.ctx().move_to_top(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::unique("vault"),
    ));
    if submit || enter {
        st.run(Command::SubmitVault);
    } else if cancel || closed {
        st.vault_form = None;
    }
}

/// Écran plein fenêtre affiché au lancement quand le fichier de données est chiffré
pub fn unlock_screen(ui: &mut Ui, data_path: &Path, st: &mut UiState) {
    // Sans décorations natives, la barre de titre est le seul moyen de fermer la fenêtre
    titlebar::show(ui, None);
    let mut submit = false;
    egui::CentralPanel::no_frame().show(ui, |ui| {
        Frame::new().fill(p().bg_primary).show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            let Some(form) = &mut st.vault_form else {
                return;
            };
            ui.vertical_centered(|ui| {
                ui.add_space((ui.available_height() * 0.5 - 200.0).max(24.0));
                card_frame().show(ui, |ui| {
                    ui.set_width(380.0);
                    ui.vertical_centered(|ui| {
                        ui.label(RichText::new("🔒").size(40.0).color(p().accent_fg));
                        ui.label(
                            RichText::new("Inventaire chiffré")
                                .heading()
                                .color(p().text_strong),
                        );
                        ui.label(
                            RichText::new("Saisis le mot de passe maître pour l'ouvrir.")
                                .color(p().text_secondary),
                        );
                        let shown = std::path::absolute(data_path)
                            .unwrap_or_else(|_| data_path.to_path_buf());
                        ui.add(
                            egui::Label::new(
                                RichText::new(shown.display().to_string())
                                    .small()
                                    .monospace()
                                    .color(p().text_muted),
                            )
                            .truncate(),
                        );
                    });
                    ui.add_space(8.0);
                    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                        if fields(ui, form) {
                            submit = true;
                        }
                        error_text(ui, &form.error);
                        ui.add_space(6.0);
                        ui.vertical_centered_justified(|ui| {
                            if primary_button(ui, "Déverrouiller").clicked() {
                                submit = true;
                            }
                        });
                    });
                });
                ui.add_space(10.0);
                ui.label(
                    RichText::new(
                        "Mot de passe maître oublié : l'inventaire chiffré est irrécupérable.",
                    )
                    .small()
                    .color(p().text_muted),
                );
            });
        });
    });
    if submit {
        st.run(Command::SubmitVault);
    }
    toasts(ui, st);
    titlebar::resize_handles(ui);
}
