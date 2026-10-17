/// Formulaire de création / modification d'une VM (onglets Général, Réseau, SSH)
use egui::{Color32, RichText, Ui};
use vm_core::{Host, SshAuthMethod, Store};

use super::vm_detail::auth_label;
use super::vms::{host_label, sorted_groups, sorted_hosts};
use super::widgets::*;
use crate::app::{Command, UiState};
use crate::forms::VmTab;
use crate::os_presets::OS_FAMILIES;
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
                        p().text_secondary
                    });
                    let button = egui::Button::new(text)
                        .fill(if active { p().accent } else { p().bg_secondary })
                        .stroke(egui::Stroke::NONE);
                    if ui.add(button).clicked() {
                        form.tab = tab;
                    }
                }
            });
            ui.add_space(10.0);

            match form.tab {
                VmTab::General => {
                    form.sync_host(store.hosts());
                    ui.columns(2, |c| {
                        labeled(&mut c[0], "Nom *", None, |ui| {
                            let r = text_field(ui, &mut form.name, "web-01", false);
                            autofocus(&r, &mut form.focused);
                        });
                        labeled(&mut c[1], "Machine physique", None, |ui| {
                            let selected = form
                                .host_id
                                .as_ref()
                                .and_then(|id| store.hosts().iter().find(|h| &h.id == id))
                                .map(host_label)
                                .unwrap_or_else(|| "Aucune".into());
                            let mut choice: Option<Option<&Host>> = None;
                            egui::ComboBox::from_id_salt("vm-host")
                                .width(ui.available_width())
                                .selected_text(selected)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(form.host_id.is_none(), "Aucune")
                                        .clicked()
                                    {
                                        choice = Some(None);
                                    }
                                    for h in sorted_hosts(store) {
                                        let active = form.host_id.as_deref() == Some(h.id.as_str());
                                        if ui.selectable_label(active, host_label(h)).clicked() {
                                            choice = Some(Some(h));
                                        }
                                    }
                                });
                            if let Some(host) = choice {
                                form.set_host(host);
                            }
                        });
                    });
                    ui.columns(2, |c| {
                        let locked = form.host_id.is_some();
                        c[0].vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 4.0;
                                ui.label(
                                    RichText::new("Groupe")
                                        .small()
                                        .strong()
                                        .color(p().text_secondary),
                                );
                                if locked {
                                    ui.label(
                                        RichText::new("(groupe de la machine)")
                                            .small()
                                            .color(p().text_muted),
                                    );
                                }
                            });
                            let selected = form
                                .group_id
                                .as_ref()
                                .and_then(|id| store.groups().iter().find(|g| &g.id == id))
                                .map(|g| g.name.clone())
                                .unwrap_or_else(|| "Non classée".into());
                            ui.add_enabled_ui(!locked, |ui| {
                                egui::ComboBox::from_id_salt("vm-group")
                                    .width(ui.available_width())
                                    .selected_text(selected)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut form.group_id,
                                            None,
                                            "Non classée",
                                        );
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
                        labeled(&mut c[1], "Système d'exploitation", None, |ui| {
                            os_field(ui, &mut form.os);
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
                    labeled(
                        ui,
                        "Étiquettes",
                        Some("séparées par des virgules"),
                        |ui| {
                            text_field(ui, &mut form.tags, "web, docker", false);
                        },
                    );
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
                            .color(p().warning),
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

/// Système d'exploitation : saisie libre + menu des systèmes prédéfinis par famille
fn os_field(ui: &mut Ui, os: &mut String) {
    const BUTTON_WIDTH: f32 = 34.0;
    let width = ui.available_width() - BUTTON_WIDTH - ui.spacing().item_spacing.x;
    // Menus compacts : la famille Linux compte près de 30 entrées
    let compact = |ui: &mut Ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.spacing_mut().interact_size.y = 24.0;
        ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);
    };
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(os)
                .hint_text("Debian 13, Windows Server 2025…")
                .desired_width(width)
                .margin(egui::Margin::symmetric(8, 6)),
        );
        ui.spacing_mut().button_padding.x = 6.0;
        let button = egui::Button::new(RichText::new("⏷").color(p().text_secondary))
            .fill(p().bg_secondary)
            .min_size(egui::vec2(BUTTON_WIDTH, 30.0));
        egui::containers::menu::MenuButton::from_button(button)
            .ui(ui, |ui| {
                compact(ui);
                ui.set_min_width(170.0);
                for family in &OS_FAMILIES {
                    ui.menu_button(format!("{}  {}", family.icon, family.name), |ui| {
                        compact(ui);
                        ui.set_min_width(220.0);
                        let max_height = ui.ctx().content_rect().height() - 40.0;
                        egui::ScrollArea::vertical()
                            .max_height(max_height)
                            .show(ui, |ui| {
                                for system in family.systems {
                                    let selected = os.trim() == *system;
                                    if ui.selectable_label(selected, *system).clicked() {
                                        *os = system.to_string();
                                        ui.close();
                                    }
                                }
                            });
                    });
                }
            })
            .0
            .on_hover_text("Systèmes prédéfinis");
    });
}
