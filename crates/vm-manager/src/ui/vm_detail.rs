/// Fiche d'une VM : machine physique, réseau, SSH, utilisateurs (gérés en place), notes
use egui::{RichText, Ui};
use vm_core::query::ssh_command;
use vm_core::{SshAuthMethod, Store, Vm, VmUser};

use super::widgets::*;
use crate::app::{Command, UiState};
use crate::forms::{generate_password, UserForm, VmForm};
use crate::os_presets::family_of;
use crate::theme::*;

pub fn auth_label(m: SshAuthMethod) -> &'static str {
    match m {
        SshAuthMethod::Key => "Clé SSH",
        SshAuthMethod::Password => "Mot de passe",
        SshAuthMethod::Agent => "Agent SSH",
        SshAuthMethod::Other => "Autre",
    }
}

pub fn show(ui: &Ui, store: &Store, st: &mut UiState) {
    let Some(id) = st.open_vm.clone() else { return };
    let Ok(vm) = store.vm(&id) else {
        st.open_vm = None;
        return;
    };
    let group = vm
        .group_id
        .as_ref()
        .and_then(|gid| store.groups().iter().find(|g| &g.id == gid));
    let host = vm.host_id.as_ref().and_then(|hid| store.host(hid).ok());
    let updated = vm
        .updated_at
        .with_timezone(&chrono::Local)
        .format("Modifiée le %d/%m/%Y à %H:%M")
        .to_string();

    let mut action: Option<Action> = None;
    let mut footer_action: Option<Action> = None;
    let closed = modal(
        ui,
        "vm-detail",
        &vm.name,
        Some(&updated),
        760.0,
        |ui| {
            ui.horizontal_wrapped(|ui| {
                match group {
                    Some(g) => chip(ui, format!("• {}", g.name), parse_hex(g.color.as_deref())),
                    None => chip(ui, "Non classée", p().text_muted),
                }
                if !vm.os.is_empty() {
                    let os = match family_of(&vm.os) {
                        Some(family) => format!("{} {}", family.icon, vm.os),
                        None => vm.os.clone(),
                    };
                    chip(ui, os, p().text_secondary);
                }
                for tag in &vm.tags {
                    chip(ui, tag, p().accent_2);
                }
            });
            if !vm.description.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(&vm.description).color(p().text_strong));
            }
            ui.add_space(10.0);

            if let Some(h) = host {
                section_title(ui, "Machine physique");
                inset_frame().show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🖧").color(p().accent_fg));
                        // Lien vers la fiche de la machine physique
                        let link = ui
                            .link(RichText::new(&h.name).strong())
                            .on_hover_text("Ouvrir la fiche de la machine physique");
                        if link.clicked() {
                            action = Some(Action::OpenHost(h.id.clone()));
                        }
                        if !h.hypervisor.is_empty() {
                            chip(ui, &h.hypervisor, p().text_secondary);
                        }
                        if !h.ip.is_empty() {
                            ui.label(RichText::new(&h.ip).monospace().color(p().text_secondary));
                            copy_button(ui, &h.ip);
                        }
                    });
                });
                ui.add_space(10.0);
            }

            section_title(ui, "Réseau");
            let n = &vm.network;
            let vlan = n.vlan.map(|v| v.to_string()).unwrap_or_default();
            let dns = n.dns.join(", ");
            two_columns(
                ui,
                &[
                    ("Adresse IP", &n.ip, true),
                    ("Nom d'hôte", &n.hostname, true),
                    ("Sous-réseau", &n.subnet, false),
                    ("Passerelle", &n.gateway, false),
                    ("VLAN", &vlan, false),
                    ("DNS", &dns, false),
                    ("MAC", &n.mac, true),
                    ("Pont", &n.bridge, false),
                ],
            );
            ui.add_space(10.0);

            section_title(ui, "Connexion SSH");
            inset_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                if let Some(cmd) = ssh_command(vm, None) {
                    info_row(ui, "Commande", &cmd, true, true);
                }
                let port = vm.ssh.port.to_string();
                grid(
                    ui,
                    "ssh-grid",
                    &[
                        ("Port", &port, false),
                        ("Utilisateur", &vm.ssh.default_user, true),
                        ("Authentification", auth_label(vm.ssh.auth_method), false),
                        ("Clé / secret", &vm.ssh.key_reference, false),
                    ],
                );
                if !vm.ssh.notes.is_empty() {
                    ui.label(RichText::new(&vm.ssh.notes).color(p().text));
                }
            });
            ui.add_space(10.0);

            section_title(ui, &format!("Utilisateurs ({})", vm.users.len()));
            users(ui, store, vm, st, &mut action);

            if !vm.notes.is_empty() {
                ui.add_space(10.0);
                section_title(ui, "Notes");
                inset_frame().fill(p().bg_input).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(&vm.notes).monospace().color(p().text));
                });
            }
        },
        |ui| {
            if ghost_button(ui, "🗑  Supprimer").clicked() {
                footer_action = Some(Action::DeleteVm);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, "✏  Modifier").clicked() {
                    footer_action = Some(Action::Edit);
                }
                if ghost_button(ui, "🗐  Dupliquer").clicked() {
                    footer_action = Some(Action::Duplicate);
                }
            });
        },
    );

    match footer_action.or(action) {
        Some(Action::Edit) => st.vm_form = Some(VmForm::edit(vm)),
        Some(Action::OpenHost(host_id)) => {
            st.open_vm = None;
            st.open_host(&host_id);
        }
        Some(Action::Duplicate) => st.run(Command::DuplicateVm(vm.id.clone())),
        Some(Action::DeleteVm) => st.ask(
            "Supprimer la VM",
            format!(
                "Supprimer définitivement « {} » et ses {} utilisateur(s) ?",
                vm.name,
                vm.users.len()
            ),
            Command::DeleteVm(vm.id.clone()),
        ),
        Some(Action::DeleteUser(user_id, username)) => st.ask(
            "Supprimer l'utilisateur",
            format!("Retirer « {username} » de {} ?", vm.name),
            Command::DeleteUser {
                vm_id: vm.id.clone(),
                user_id,
            },
        ),
        None if closed && st.user_form.is_none() => st.open_vm = None,
        None if closed => st.user_form = None,
        None => {}
    }
}

enum Action {
    Edit,
    Duplicate,
    DeleteVm,
    DeleteUser(String, String),
    OpenHost(String),
}

fn two_columns(ui: &mut Ui, rows: &[(&str, &str, bool)]) {
    inset_frame().show(ui, |ui| {
        ui.set_width(ui.available_width());
        grid(ui, "net-grid", rows);
    });
}

/// Grille à deux colonnes de paires libellé / valeur
fn grid(ui: &mut Ui, id: &str, rows: &[(&str, &str, bool)]) {
    let half = (ui.available_width() - 16.0) / 2.0;
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for pair in rows.chunks(2) {
                for (label, value, copy) in pair {
                    ui.allocate_ui(egui::vec2(half, 20.0), |ui| {
                        ui.set_width(half);
                        info_row(ui, label, value, *copy, true);
                    });
                }
                ui.end_row();
            }
        });
}

fn users(ui: &mut Ui, store: &Store, vm: &Vm, st: &mut UiState, action: &mut Option<Action>) {
    let editing_this_vm = st.user_form.as_ref().is_some_and(|f| f.vm_id == vm.id);
    if vm.users.is_empty() && !editing_this_vm {
        ui.label(RichText::new("Aucun utilisateur enregistré.").color(p().text_muted));
    }

    for u in &vm.users {
        let editing = st
            .user_form
            .as_ref()
            .is_some_and(|f| f.vm_id == vm.id && f.user_id.as_deref() == Some(u.id.as_str()));
        if editing {
            user_form(ui, store, st);
            continue;
        }
        inset_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&u.username)
                        .monospace()
                        .color(p().text_strong),
                );
                copy_button(ui, &u.username);
                if u.sudo {
                    chip(ui, "sudo", p().warning);
                }
                if !u.role.is_empty() {
                    chip(ui, &u.role, p().text_secondary);
                }
                if u.username == vm.ssh.default_user {
                    chip(ui, "SSH par défaut", p().accent_fg);
                }
                if u.password.is_some() {
                    chip(ui, "🔑 mot de passe", p().success);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, "🗑", "Supprimer").clicked() {
                        *action = Some(Action::DeleteUser(u.id.clone(), u.username.clone()));
                    }
                    if icon_button(ui, "✏", "Modifier").clicked() && st.user_form.is_none() {
                        st.user_form = Some(UserForm::edit(&vm.id, u));
                    }
                });
            });
            if u.password.is_some() {
                password_row(ui, store, &vm.id, u, st);
            }
            let details: Vec<&str> = [u.full_name.as_str(), u.notes.as_str()]
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect();
            if !details.is_empty() {
                ui.label(
                    RichText::new(details.join(" — "))
                        .small()
                        .color(p().text_secondary),
                );
            }
        });
    }

    let adding = st
        .user_form
        .as_ref()
        .is_some_and(|f| f.vm_id == vm.id && f.user_id.is_none());
    if adding {
        user_form(ui, store, st);
    } else {
        let enabled = st.user_form.is_none();
        let add = ui.add_enabled(
            enabled,
            egui::Button::new("➕  Ajouter un utilisateur")
                .fill(p().bg_secondary)
                .min_size(egui::vec2(ui.available_width(), 32.0)),
        );
        if add.clicked() {
            st.user_form = Some(UserForm::create(&vm.id));
        }
    }
}

/// Mot de passe enregistré : masqué, affichable et copiable si le coffre est déverrouillé
fn password_row(ui: &mut Ui, store: &Store, vm_id: &str, u: &VmUser, st: &mut UiState) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Mot de passe")
                .small()
                .color(p().text_secondary),
        );
        // Copie de la clé : `st` reste modifiable pendant l'affichage
        let Some(key) = st.vault_key.clone() else {
            let unlock = egui::Button::new(
                RichText::new("🔒  déverrouiller le coffre")
                    .small()
                    .color(p().text_secondary),
            )
            .frame(false);
            if ui
                .add(unlock)
                .on_hover_text(
                    "Saisir le mot de passe maître pour afficher ou copier ce mot de passe",
                )
                .clicked()
            {
                st.ask_unlock(store.vault().is_some(), None);
            }
            return;
        };
        match store.user_password(&key, vm_id, &u.id) {
            Ok(Some(password)) => {
                let revealed = st.revealed.contains(&u.id);
                let shown = if revealed { password.as_str() } else { MASK };
                ui.label(RichText::new(shown).monospace().color(p().text_strong));
                let tooltip = if revealed {
                    "Masquer"
                } else {
                    "Afficher en clair"
                };
                if icon_button(ui, "👁", tooltip).clicked() {
                    if revealed {
                        st.revealed.remove(&u.id);
                    } else {
                        st.revealed.insert(u.id.clone());
                    }
                }
                if icon_button(ui, "🗐", "Copier le mot de passe").clicked() {
                    ui.ctx().copy_text(password.to_string());
                    st.success("Mot de passe copié");
                }
            }
            Ok(None) => {}
            Err(e) => {
                ui.label(RichText::new(format!("⚠ {e}")).small().color(p().error));
            }
        }
    });
}

fn user_form(ui: &mut Ui, store: &Store, st: &mut UiState) {
    let vault_open = st.vault_form.is_some();
    let unlocked = st.is_unlocked();
    let Some(form) = &mut st.user_form else {
        return;
    };
    let mut submit = false;
    let mut cancel = false;
    inset_frame()
        .stroke(egui::Stroke::new(1.0, p().accent.linear_multiply(0.6)))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.columns(3, |c| {
                labeled(&mut c[0], "Identifiant *", None, |ui| {
                    let r = text_field(ui, &mut form.username, "deploy", true);
                    autofocus(&r, &mut form.focused);
                });
                labeled(&mut c[1], "Nom complet", None, |ui| {
                    text_field(ui, &mut form.full_name, "", false);
                });
                labeled(&mut c[2], "Rôle", None, |ui| {
                    text_field(ui, &mut form.role, "admin, service…", false);
                });
            });
            let hint = if form.has_password {
                "laisser vide pour conserver l'actuel"
            } else {
                "facultatif, chiffré dans le coffre"
            };
            labeled(ui, "Mot de passe", Some(hint), |ui| {
                ui.add_enabled_ui(!form.remove_password, |ui| {
                    ui.horizontal(|ui| {
                        let placeholder = if form.has_password { "inchangé" } else { "" };
                        password_field(
                            ui,
                            &mut form.password,
                            &mut form.show_password,
                            placeholder,
                            104.0,
                        );
                        if ghost_button(ui, "🎲  Générer")
                            .on_hover_text("Mot de passe aléatoire de 20 caractères")
                            .clicked()
                        {
                            match generate_password() {
                                Ok(password) => {
                                    form.password = password;
                                    form.show_password = true;
                                }
                                Err(e) => form.error = Some(e),
                            }
                        }
                    });
                });
            });
            let pending = !form.password.is_empty() || form.remove_password;
            if pending && !unlocked {
                ui.label(
                    RichText::new(if store.vault().is_some() {
                        "🔒 Le mot de passe maître sera demandé à l'enregistrement."
                    } else {
                        "🔒 Un coffre (mot de passe maître) sera créé à l'enregistrement."
                    })
                    .small()
                    .color(p().text_secondary),
                );
            }
            labeled(ui, "Notes", None, |ui| {
                text_field(
                    ui,
                    &mut form.notes,
                    "Clé autorisée, groupe, expiration…",
                    false,
                );
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut form.sudo, "Droits administrateur (sudo)");
                if form.has_password {
                    ui.checkbox(&mut form.remove_password, "Supprimer le mot de passe");
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    submit = primary_button(ui, "Enregistrer").clicked();
                    cancel = ghost_button(ui, "Annuler").clicked();
                });
            });
            // Entrée dans la fenêtre du coffre ne doit pas revalider ce formulaire
            if !vault_open && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                submit = true;
            }
            error_text(ui, &form.error);
        });
    if submit {
        st.run(Command::SaveUser);
    } else if cancel {
        st.user_form = None;
    }
}
