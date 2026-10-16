/// Fiche d'une VM : réseau, SSH, utilisateurs (gérés en place), notes
use egui::{RichText, Ui};
use vm_core::query::ssh_command;
use vm_core::{SshAuthMethod, Store, Vm};

use super::widgets::*;
use crate::app::{Command, UiState};
use crate::forms::{UserForm, VmForm};
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
                    chip(ui, &vm.os, p().text_secondary);
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
            users(ui, vm, st, &mut action);

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

fn users(ui: &mut Ui, vm: &Vm, st: &mut UiState, action: &mut Option<Action>) {
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
            user_form(ui, st);
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, "🗑", "Supprimer").clicked() {
                        *action = Some(Action::DeleteUser(u.id.clone(), u.username.clone()));
                    }
                    if icon_button(ui, "✏", "Modifier").clicked() && st.user_form.is_none() {
                        st.user_form = Some(UserForm::edit(&vm.id, u));
                    }
                });
            });
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
        user_form(ui, st);
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

fn user_form(ui: &mut Ui, st: &mut UiState) {
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    submit = primary_button(ui, "Enregistrer").clicked();
                    cancel = ghost_button(ui, "Annuler").clicked();
                });
            });
            if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
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
