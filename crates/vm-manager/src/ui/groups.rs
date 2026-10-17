/// Groupes : liste dépliable (machines physiques et VM membres), création,
/// modification, suppression
use egui::{RichText, Sense, Stroke, Ui};
use vm_core::{Host, Store, Vm};

use super::vms::{set_host_filter, sorted_groups, sorted_hosts, HostFilter};
use super::widgets::*;
use crate::app::{Command, GroupFilter, UiState};
use crate::forms::GroupForm;
use crate::theme::*;

pub fn show(ui: &mut Ui, store: &Store, st: &mut UiState) {
    let count = store.groups().len();
    page_header(
        ui,
        "Groupes",
        &format!("{count} groupe{}", if count > 1 { "s" } else { "" }),
        |ui| {
            if primary_button(ui, "➕  Créer un groupe").clicked() {
                st.group_form = Some(GroupForm::create());
            }
        },
    );

    if count == 0 {
        if empty_state(
            ui,
            "📁",
            "Aucun groupe",
            "Regroupe tes VM par environnement, client ou hyperviseur.",
            "Créer un groupe",
        ) {
            st.group_form = Some(GroupForm::create());
        }
        return;
    }

    for g in sorted_groups(store) {
        let mut members: Vec<&Vm> = store
            .vms()
            .iter()
            .filter(|v| v.group_id.as_deref() == Some(g.id.as_str()))
            .collect();
        members.sort_by_key(|v| v.name.to_lowercase());
        let hosts: Vec<&Host> = sorted_hosts(store)
            .into_iter()
            .filter(|h| h.group_id.as_deref() == Some(g.id.as_str()))
            .collect();
        let open = st.expanded_groups.contains(&g.id);

        card_frame().show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if icon_button(
                    ui,
                    if open { "⏷" } else { "⏵" },
                    "Afficher les machines et les VM",
                )
                .clicked()
                {
                    if open {
                        st.expanded_groups.remove(&g.id);
                    } else {
                        st.expanded_groups.insert(g.id.clone());
                    }
                }
                color_dot(ui, parse_hex(g.color.as_deref()));
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new(&g.name).strong().color(p().text_strong));
                    let desc = if g.description.is_empty() {
                        "Pas de description"
                    } else {
                        &g.description
                    };
                    ui.label(RichText::new(desc).small().color(p().text_secondary));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if icon_button(ui, "🗑", "Supprimer").clicked() {
                        let message = delete_message(&g.name, hosts.len(), members.len());
                        st.ask(
                            "Supprimer le groupe",
                            message,
                            Command::DeleteGroup(g.id.clone()),
                        );
                    }
                    if icon_button(ui, "✏", "Modifier").clicked() {
                        st.group_form = Some(GroupForm::edit(g));
                    }
                    let link = ui
                        .link(format!("{} · {} VM", machines(hosts.len()), members.len()))
                        .on_hover_text("Voir ces VM");
                    if link.clicked() {
                        set_host_filter(ui.ctx(), HostFilter::All);
                        st.open_vms(GroupFilter::Group(g.id.clone()));
                    }
                });
            });

            if open {
                ui.separator();
                if members.is_empty() && hosts.is_empty() {
                    ui.label(
                        RichText::new("Aucune machine ni VM dans ce groupe.").color(p().text_muted),
                    );
                }
                // Arborescence : machines physiques et leurs VM, puis VM sans machine
                for h in &hosts {
                    let on_host: Vec<&Vm> = members
                        .iter()
                        .copied()
                        .filter(|v| v.host_id.as_deref() == Some(h.id.as_str()))
                        .collect();
                    let r = ui
                        .scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🖧").color(p().accent_fg));
                                cell(
                                    ui,
                                    180.0,
                                    RichText::new(&h.name).strong().color(p().text_strong),
                                );
                                let hypervisor = if h.hypervisor.is_empty() {
                                    "—"
                                } else {
                                    &h.hypervisor
                                };
                                cell(
                                    ui,
                                    140.0,
                                    RichText::new(hypervisor).color(p().text_secondary),
                                );
                                ui.label(
                                    RichText::new(format!("{} VM", on_host.len()))
                                        .small()
                                        .color(p().text_muted),
                                );
                            });
                        })
                        .response
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Voir les VM de cette machine");
                    if r.clicked() {
                        set_host_filter(ui.ctx(), HostFilter::Host(h.id.clone()));
                        st.open_vms(GroupFilter::Group(g.id.clone()));
                    }
                    ui.indent(("group-host", &h.id), |ui| {
                        if on_host.is_empty() {
                            ui.label(
                                RichText::new("Aucune VM sur cette machine.")
                                    .small()
                                    .color(p().text_muted),
                            );
                        }
                        for vm in on_host {
                            vm_row(ui, vm, st);
                        }
                    });
                }
                let without_host: Vec<&Vm> = members
                    .iter()
                    .copied()
                    .filter(|v| {
                        !hosts
                            .iter()
                            .any(|h| v.host_id.as_deref() == Some(h.id.as_str()))
                    })
                    .collect();
                if !hosts.is_empty() && !without_host.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("Sans machine physique  ({})", without_host.len()))
                            .small()
                            .strong()
                            .color(p().text_muted),
                    );
                }
                for vm in without_host {
                    vm_row(ui, vm, st);
                }
            }
        });
        ui.add_space(4.0);
    }
}

/// « 1 machine », « 3 machines »
fn machines(n: usize) -> String {
    format!("{n} machine{}", if n > 1 { "s" } else { "" })
}

/// Confirmation de suppression d'un groupe : ses machines et VM sont conservées
fn delete_message(name: &str, hosts: usize, vms: usize) -> String {
    let kept = match (hosts, vms) {
        (0, 0) => return format!("Supprimer « {name} » ?"),
        (0, m) => format!("Ses {m} VM sont conservées et deviennent « non classées »."),
        (1, 0) => "Sa machine physique est conservée et devient sans groupe.".to_string(),
        (1, m) => {
            format!("Sa machine physique et ses {m} VM sont conservées et deviennent sans groupe.")
        }
        (n, 0) => format!("Ses {n} machines physiques sont conservées et deviennent sans groupe."),
        (n, m) => format!(
            "Ses {n} machines physiques et {m} VM sont conservées et deviennent sans groupe."
        ),
    };
    format!("Supprimer « {name} » ?\n{kept}")
}

/// Ligne cliquable d'une VM (nom, IP, description) ouvrant sa fiche
fn vm_row(ui: &mut Ui, vm: &Vm, st: &mut UiState) {
    let r = ui
        .scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
            ui.horizontal(|ui| {
                let ip = if vm.network.ip.is_empty() {
                    "—"
                } else {
                    &vm.network.ip
                };
                cell(ui, 200.0, RichText::new(&vm.name).color(p().text_strong));
                cell(
                    ui,
                    160.0,
                    RichText::new(ip).monospace().color(p().text_secondary),
                );
                ui.add(
                    egui::Label::new(RichText::new(&vm.description).color(p().text_secondary))
                        .truncate(),
                );
            });
        })
        .response
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if r.clicked() {
        st.open_vm(&vm.id);
    }
}

/// Fenêtre de création / modification d'un groupe
pub fn form(ui: &Ui, st: &mut UiState) {
    let Some(form) = &mut st.group_form else {
        return;
    };
    let title = match &form.edit_id {
        Some(_) => format!("Modifier « {} »", form.name.trim()),
        None => "Nouveau groupe".to_string(),
    };
    let mut submit = false;
    let mut enter = false;
    let mut cancel = false;
    let closed = modal(
        ui,
        "group-form",
        &title,
        None,
        480.0,
        |ui| {
            labeled(ui, "Nom *", None, |ui| {
                let r = text_field(ui, &mut form.name, "Production, Lab, Clients…", false);
                autofocus(&r, &mut form.focused);
            });
            labeled(ui, "Description", None, |ui| {
                text_area(ui, &mut form.description, "", 3, false);
            });
            labeled(ui, "Couleur", None, |ui| {
                ui.horizontal(|ui| {
                    for hex in GROUP_COLORS {
                        let selected = form.color.as_deref() == Some(hex);
                        let (rect, r) =
                            ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
                        ui.painter()
                            .circle_filled(rect.center(), 11.0, parse_hex(Some(hex)));
                        if selected {
                            ui.painter().circle_stroke(
                                rect.center(),
                                12.5,
                                Stroke::new(2.0, p().text_strong),
                            );
                        }
                        if r.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                            form.color = Some(hex.to_string());
                        }
                    }
                });
            });
            enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
            error_text(ui, &form.error);
        },
        |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if form.edit_id.is_some() {
                    "Enregistrer"
                } else {
                    "Créer"
                };
                submit = primary_button(ui, label).clicked();
                cancel = ghost_button(ui, "Annuler").clicked();
            });
        },
    );
    if submit || enter {
        st.run(Command::SaveGroup);
    } else if cancel || closed {
        st.group_form = None;
    }
}

/// Cellule de largeur fixe, texte aligné à gauche et tronqué
fn cell(ui: &mut Ui, width: f32, text: RichText) {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 20.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_width(width);
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_delete_messages() {
        assert_eq!(delete_message("Lab", 0, 0), "Supprimer « Lab » ?");
        assert!(delete_message("Lab", 0, 3)
            .ends_with("Ses 3 VM sont conservées et deviennent « non classées »."));
        assert!(
            delete_message("Lab", 1, 2).contains("Sa machine physique et ses 2 VM sont conservées")
        );
        assert!(delete_message("Lab", 2, 0).contains("Ses 2 machines physiques sont conservées"));
        assert!(delete_message("Lab", 2, 5).contains(
            "Ses 2 machines physiques et 5 VM sont conservées et deviennent sans groupe."
        ));
        assert_eq!(machines(1), "1 machine");
        assert_eq!(machines(0), "0 machine");
        assert_eq!(machines(3), "3 machines");
    }
}
