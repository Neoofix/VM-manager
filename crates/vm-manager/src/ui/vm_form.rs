/// Formulaire de création / modification d'une VM (onglets Général, Réseau, SSH)
use egui::{Color32, RichText, Ui};
use vm_core::{SshAuthMethod, Store};

use super::vm_detail::auth_label;
use super::vms::sorted_groups;
use super::widgets::*;
use crate::app::{Command, UiState};
use crate::forms::VmTab;
use crate::theme::*;

/// Champ du formulaire réseau : libellé, indication, valeur, exemple
type NetField<'a> = (&'a str, Option<&'a str>, &'a mut String, &'a str);

const TABS: [(VmTab, &str); 3] = [
    (VmTab::General, "ℹ  Général"),
    (VmTab::Network, "🔀  Réseau"),
    (VmTab::Ssh, "🔑  SSH"),
];

pub fn show(ui: &Ui, store: &Store, st: &mut UiState) {
    let Some(form) = &mut st.vm_form else { return };
    let title = match &form.edit_id {
        Some(_) => format!("Modifier « {} »", form.name.trim()),
        None => "Nouvelle VM".to_string(),
    };
    let mut submit = false;
    let mut cancel = false;

    let closed = modal(
        ui,
        "vm-form",
        &title,
        None,
        720.0,
        |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in TABS {
                    let active = form.tab == tab;
                    let text = RichText::new(label).color(if active {
                        Color32::WHITE
                    } else {
                        TEXT_SECONDARY
                    });
                    let button = egui::Button::new(text)
                        .fill(if active { ACCENT } else { BG_SECONDARY })
                        .stroke(egui::Stroke::NONE);
                    if ui.add(button).clicked() {
                        form.tab = tab;
                    }
                }
            });
            ui.add_space(10.0);

            match form.tab {
                VmTab::General => {
                    ui.columns(2, |c| {
                        labeled(&mut c[0], "Nom *", None, |ui| {
                            let r = text_field(ui, &mut form.name, "web-01", false);
                            autofocus(&r, &mut form.focused);
                        });
                        labeled(&mut c[1], "Groupe", None, |ui| {
                            let selected = form
                                .group_id
                                .as_ref()
                                .and_then(|id| store.groups().iter().find(|g| &g.id == id))
                                .map(|g| g.name.clone())
                                .unwrap_or_else(|| "Non classée".into());
                            egui::ComboBox::from_id_salt("vm-group")
                                .width(ui.available_width())
                                .selected_text(selected)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut form.group_id, None, "Non classée");
                                    for g in sorted_groups(store) {
                                        ui.selectable_value(
                                            &mut form.group_id,
                                            Some(g.id.clone()),
                                            &g.name,
                                        );
                                    }
                                });
                        });
                    });
                    labeled(ui, "Description", None, |ui| {
                        text_area(
                            ui,
                            &mut form.description,
                            "Rôle de la machine, services hébergés…",
                            2,
                            false,
                        );
                    });
                    ui.columns(2, |c| {
                        labeled(&mut c[0], "Système d'exploitation", None, |ui| {
                            text_field(ui, &mut form.os, "Debian 13, Windows Server 2025…", false);
                        });
                        labeled(
                            &mut c[1],
                            "Étiquettes",
                            Some("séparées par des virgules"),
                            |ui| {
                                text_field(ui, &mut form.tags, "web, docker", false);
                            },
                        );
                    });
                    labeled(ui, "Notes", None, |ui| {
                        text_area(ui, &mut form.notes, "", 4, true);
                    });
                }
                VmTab::Network => {
                    let rows: [[NetField; 2]; 4] = [
                        [
                            ("Adresse IP", None, &mut form.ip, "192.168.10.21"),
                            ("Nom d'hôte", None, &mut form.hostname, "web-01.lan"),
                        ],
                        [
                            (
                                "Sous-réseau (CIDR)",
                                None,
                                &mut form.subnet,
                                "192.168.10.0/24",
                            ),
                            ("Passerelle", None, &mut form.gateway, "192.168.10.1"),
                        ],
                        [
                            ("VLAN", Some("1 à 4094"), &mut form.vlan, "10"),
                            (
                                "Serveurs DNS",
                                Some("séparés par des virgules"),
                                &mut form.dns,
                                "192.168.10.1, 1.1.1.1",
                            ),
                        ],
                        [
                            ("Adresse MAC", None, &mut form.mac, "bc:24:11:aa:bb:cc"),
                            (
                                "Pont / interface hyperviseur",
                                None,
                                &mut form.bridge,
                                "vmbr0",
                            ),
                        ],
                    ];
                    for row in rows {
                        ui.columns(2, |c| {
                            for (col, (label, hint, value, placeholder)) in c.iter_mut().zip(row) {
                                labeled(col, label, hint, |ui| {
                                    text_field(ui, value, placeholder, true);
                                });
                            }
                        });
                    }
                }
                VmTab::Ssh => {
                    ui.columns(2, |c| {
                        labeled(&mut c[0], "Port SSH", None, |ui| {
                            text_field(ui, &mut form.port, "22", true);
                        });
                        labeled(&mut c[1], "Utilisateur par défaut", None, |ui| {
                            text_field(ui, &mut form.default_user, "deploy", true);
                        });
                    });
                    ui.columns(2, |c| {
                        labeled(&mut c[0], "Authentification", None, |ui| {
                            egui::ComboBox::from_id_salt("vm-auth")
                                .width(ui.available_width())
                                .selected_text(auth_label(form.auth_method))
                                .show_ui(ui, |ui| {
                                    for m in [
                                        SshAuthMethod::Key,
                                        SshAuthMethod::Password,
                                        SshAuthMethod::Agent,
                                        SshAuthMethod::Other,
                                    ] {
                                        ui.selectable_value(
                                            &mut form.auth_method,
                                            m,
                                            auth_label(m),
                                        );
                                    }
                                });
                        });
                        labeled(&mut c[1], "Référence de la clé / du secret", None, |ui| {
                            text_field(
                                ui,
                                &mut form.key_reference,
                                "~/.ssh/id_ed25519 ou Bitwarden › web-01",
                                true,
                            );
                        });
                    });
                    ui.label(
                        RichText::new("Indique où trouver la clé ou le mot de passe, jamais le secret lui-même.")
                            .small()
                            .color(WARNING),
                    );
                    labeled(ui, "Notes SSH", None, |ui| {
                        text_area(
                            ui,
                            &mut form.ssh_notes,
                            "Connexion root désactivée, passer par le bastion…",
                            3,
                            false,
                        );
                    });
                }
            }
            error_text(ui, &form.error);
        },
        |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                submit = primary_button(ui, "Enregistrer").clicked();
                cancel = ghost_button(ui, "Annuler").clicked();
            });
        },
    );

    if submit {
        st.run(Command::SaveVm);
    } else if cancel || closed {
        st.vm_form = None;
    }
}
