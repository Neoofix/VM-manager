/// Machines physiques : liste regroupée par groupe, fiche avec ses VM,
/// affectation de VM, formulaire de création / modification
use egui::{RichText, Sense, Ui};
use vm_core::query::{host_matches, vm_matches};
use vm_core::{Group, Host, Store, Vm};

use super::vms::sorted_groups;
use super::widgets::*;
use crate::app::{Command, GroupFilter, UiState};
use crate::host_form::{AssignState, HostForm, HYPERVISORS};
use crate::theme::*;

const CARD_WIDTH: f32 = 290.0;

fn group_of<'a>(store: &'a Store, group_id: Option<&str>) -> Option<&'a Group> {
    group_id.and_then(|id| store.groups().iter().find(|g| g.id == id))
}

fn host_of<'a>(store: &'a Store, vm: &Vm) -> Option<&'a Host> {
    vm.host_id
        .as_ref()
        .and_then(|id| store.hosts().iter().find(|h| &h.id == id))
}

/// « Xeon E-2236 · 64 Go RAM · 2 To NVMe »
fn specs(host: &Host) -> String {
    let ram = host
        .ram_gb
        .map(|r| format!("{r} Go RAM"))
        .unwrap_or_default();
    [host.cpu.as_str(), ram.as_str(), host.storage.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

fn plural(n: usize) -> &'static str {
    if n > 1 {
        "s"
    } else {
        ""
    }
}

pub fn show(ui: &mut Ui, store: &Store, st: &mut UiState) {
    let all = store.hosts();
    let mut filtered: Vec<&Host> = all
        .iter()
        .filter(|h| match &st.host_group_filter {
            GroupFilter::All => true,
            GroupFilter::Ungrouped => h.group_id.is_none(),
            GroupFilter::Group(id) => h.group_id.as_ref() == Some(id),
        })
        .filter(|h| host_matches(h, group_of(store, h.group_id.as_deref()), &st.host_search))
        .collect();
    filtered.sort_by_key(|h| h.name.to_lowercase());

    let subtitle = if filtered.len() == all.len() {
        format!(
            "{} machine{} physique{}",
            all.len(),
            plural(all.len()),
            plural(all.len())
        )
    } else {
        format!("{} sur {} machines physiques", filtered.len(), all.len())
    };
    page_header(ui, "Machines physiques", &subtitle, |ui| {
        if primary_button(ui, "➕  Nouvelle machine").clicked() {
            let group = match &st.host_group_filter {
                GroupFilter::Group(id) => Some(id.clone()),
                _ => None,
            };
            st.host_form = Some(HostForm::create(group));
        }
    });

    // Filtres
    ui.horizontal(|ui| {
        let search_width = ui.available_width() - 240.0;
        ui.add_sized(
            [search_width, 32.0],
            egui::TextEdit::singleline(&mut st.host_search)
                .hint_text("🔍  Rechercher : nom, IP, hyperviseur, emplacement…")
                .margin(egui::Margin::symmetric(10, 7)),
        );
        let selected = match &st.host_group_filter {
            GroupFilter::All => "Tous les groupes".to_string(),
            GroupFilter::Ungrouped => "Sans groupe".to_string(),
            GroupFilter::Group(id) => group_of(store, Some(id))
                .map(|g| g.name.clone())
                .unwrap_or_default(),
        };
        egui::ComboBox::from_id_salt("host-group-filter")
            .width(220.0)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                let filter = &mut st.host_group_filter;
                ui.selectable_value(filter, GroupFilter::All, "Tous les groupes");
                for g in sorted_groups(store) {
                    ui.selectable_value(filter, GroupFilter::Group(g.id.clone()), &g.name);
                }
                ui.selectable_value(filter, GroupFilter::Ungrouped, "Sans groupe");
            });
    });
    ui.add_space(8.0);

    if all.is_empty() {
        if empty_state(
            ui,
            "🗄",
            "Aucune machine physique",
            "Enregistre tes hyperviseurs pour savoir sur quelle machine tourne chaque VM.",
            "Ajouter une machine",
        ) {
            st.host_form = Some(HostForm::create(None));
        }
        return;
    }
    if filtered.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Aucune machine ne correspond à ces filtres.")
                    .color(p().text_secondary),
            );
        });
        return;
    }

    // Une section par groupe (ordre alphabétique), puis les machines sans groupe
    let mut sections: Vec<(Option<&Group>, Vec<&Host>)> = sorted_groups(store)
        .into_iter()
        .map(|g| {
            let items = filtered
                .iter()
                .copied()
                .filter(|h| h.group_id.as_deref() == Some(g.id.as_str()))
                .collect();
            (Some(g), items)
        })
        .collect();
    sections.push((
        None,
        filtered
            .iter()
            .copied()
            .filter(|h| group_of(store, h.group_id.as_deref()).is_none())
            .collect(),
    ));

    for (group, items) in sections.into_iter().filter(|(_, items)| !items.is_empty()) {
        ui.horizontal(|ui| {
            color_dot(ui, parse_hex(group.and_then(|g| g.color.as_deref())));
            let name = group.map(|g| g.name.as_str()).unwrap_or("Sans groupe");
            ui.label(RichText::new(name).strong().color(p().text_secondary));
            ui.label(RichText::new(format!("({})", items.len())).color(p().text_muted));
        });
        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
            |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                for host in items {
                    let vms = store.vms_on_host(&host.id).len();
                    host_card(ui, host, group, vms, st);
                }
            },
        );
        ui.add_space(12.0);
    }
}

fn host_card(ui: &mut Ui, host: &Host, group: Option<&Group>, vms: usize, st: &mut UiState) {
    let response = ui
        .allocate_ui_with_layout(
            egui::vec2(CARD_WIDTH, 150.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                clickable_card(ui, |ui| {
                    ui.set_width(CARD_WIDTH - 34.0);
                    ui.set_max_width(CARD_WIDTH - 34.0);
                    ui.set_min_height(118.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🗄").size(18.0).color(p().accent_fg));
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&host.name).strong().color(p().text_strong),
                                )
                                .truncate(),
                            );
                            let sub = if host.description.is_empty() {
                                "—"
                            } else {
                                &host.description
                            };
                            ui.add(
                                egui::Label::new(
                                    RichText::new(sub).small().color(p().text_secondary),
                                )
                                .truncate(),
                            );
                        });
                        if let Some(g) = group {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                                color_dot(ui, parse_hex(g.color.as_deref()));
                            });
                        }
                    });
                    ui.horizontal(|ui| {
                        let address = [&host.ip, &host.hostname]
                            .into_iter()
                            .find(|s| !s.is_empty());
                        match address {
                            Some(a) => {
                                ui.label(RichText::new(a).monospace().color(p().text_strong))
                            }
                            None => ui.label(RichText::new("Aucune adresse").color(p().text_muted)),
                        };
                        if !host.hypervisor.is_empty() {
                            chip(ui, &host.hypervisor, p().accent_2);
                        }
                    });
                    let specs = specs(host);
                    let specs = if specs.is_empty() {
                        RichText::new("Matériel non renseigné").color(p().text_muted)
                    } else {
                        RichText::new(specs).color(p().text_secondary)
                    };
                    ui.add(egui::Label::new(specs.small()).truncate());
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("🖥 {vms} VM"))
                                .small()
                                .color(p().text_secondary),
                        );
                        if !host.location.is_empty() {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(format!("📍 {}", host.location))
                                        .small()
                                        .color(p().text_secondary),
                                )
                                .truncate(),
                            );
                        }
                    });
                })
            },
        )
        .inner;
    if response.clicked() {
        st.open_host(&host.id);
    }
}

enum Action {
    Edit,
    Delete,
    Assign,
    OpenVm(String),
    Detach(String),
}

/// Fiche d'une machine physique : matériel, réseau, VM hébergées
pub fn detail(ui: &Ui, store: &Store, st: &mut UiState) {
    let Some(id) = st.open_host.clone() else {
        return;
    };
    let Ok(host) = store.host(&id) else {
        st.open_host = None;
        return;
    };
    let group = group_of(store, host.group_id.as_deref());
    let mut vms = store.vms_on_host(&host.id);
    vms.sort_by_key(|v| v.name.to_lowercase());
    let updated = host
        .updated_at
        .with_timezone(&chrono::Local)
        .format("Modifiée le %d/%m/%Y à %H:%M")
        .to_string();

    let mut action: Option<Action> = None;
    let mut footer_action: Option<Action> = None;
    let closed = modal(
        ui,
        "host-detail",
        &host.name,
        Some(&updated),
        720.0,
        |ui| {
            ui.horizontal_wrapped(|ui| {
                match group {
                    Some(g) => chip(ui, format!("• {}", g.name), parse_hex(g.color.as_deref())),
                    None => chip(ui, "Sans groupe", p().text_muted),
                }
                if !host.hypervisor.is_empty() {
                    chip(ui, &host.hypervisor, p().accent_2);
                }
            });
            if !host.description.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(&host.description).color(p().text_strong));
            }
            ui.add_space(10.0);

            section_title(ui, "Réseau et matériel");
            let ram = host.ram_gb.map(|r| format!("{r} Go")).unwrap_or_default();
            inset_frame().show(ui, |ui| {
                ui.set_width(ui.available_width());
                grid(
                    ui,
                    &[
                        ("Adresse IP", &host.ip, true),
                        ("Nom d'hôte", &host.hostname, true),
                        ("Processeur", &host.cpu, false),
                        ("Mémoire vive", &ram, false),
                        ("Stockage", &host.storage, false),
                        ("Emplacement", &host.location, false),
                    ],
                );
            });
            ui.add_space(10.0);

            section_title(ui, &format!("Machines virtuelles ({})", vms.len()));
            if vms.is_empty() {
                ui.label(
                    RichText::new("Aucune VM sur cette machine pour l'instant.")
                        .color(p().text_muted),
                );
            }
            for vm in &vms {
                let r = ui
                    .scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
                        inset_frame().show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("🖥").color(p().accent_fg));
                                cell(ui, 200.0, RichText::new(&vm.name).color(p().text_strong));
                                let ip = if vm.network.ip.is_empty() {
                                    "—"
                                } else {
                                    &vm.network.ip
                                };
                                cell(
                                    ui,
                                    150.0,
                                    RichText::new(ip).monospace().color(p().text_secondary),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ghost_button(ui, "Retirer")
                                            .on_hover_text(
                                                "Détacher la VM de cette machine (elle garde son groupe)",
                                            )
                                            .clicked()
                                        {
                                            action = Some(Action::Detach(vm.id.clone()));
                                        }
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&vm.os).color(p().text_secondary),
                                            )
                                            .truncate(),
                                        );
                                    },
                                );
                            });
                        });
                    })
                    .response
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text("Ouvrir la fiche de la VM");
                if r.clicked() {
                    action = Some(Action::OpenVm(vm.id.clone()));
                }
            }
            let assign = ui.add(
                egui::Button::new("➕  Affecter des VM")
                    .fill(p().bg_secondary)
                    .min_size(egui::vec2(ui.available_width(), 32.0)),
            );
            if assign.clicked() {
                action = Some(Action::Assign);
            }

            if !host.notes.is_empty() {
                ui.add_space(10.0);
                section_title(ui, "Notes");
                inset_frame().fill(p().bg_input).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.label(RichText::new(&host.notes).monospace().color(p().text));
                });
            }
        },
        |ui| {
            if ghost_button(ui, "🗑  Supprimer").clicked() {
                footer_action = Some(Action::Delete);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, "✏  Modifier").clicked() {
                    footer_action = Some(Action::Edit);
                }
            });
        },
    );

    match footer_action.or(action) {
        Some(Action::Edit) => st.host_form = Some(HostForm::edit(host)),
        Some(Action::Delete) => {
            let message = if vms.is_empty() {
                format!("Supprimer la machine physique « {} » ?", host.name)
            } else if vms.len() == 1 {
                format!(
                    "Supprimer la machine physique « {} » ?\nSa VM est conservée et n'est plus rattachée à une machine.",
                    host.name
                )
            } else {
                format!(
                    "Supprimer la machine physique « {} » ?\nSes {} VM sont conservées et ne sont plus rattachées à une machine.",
                    host.name,
                    vms.len()
                )
            };
            st.ask(
                "Supprimer la machine physique",
                message,
                Command::DeleteHost(host.id.clone()),
            );
        }
        Some(Action::Assign) => st.assign_host = Some(AssignState::new(&host.id)),
        Some(Action::OpenVm(vm_id)) => {
            st.open_host = None;
            st.open_vm(&vm_id);
        }
        Some(Action::Detach(vm_id)) => st.run(Command::DetachVm { vm_id }),
        None if closed => st.open_host = None,
        None => {}
    }
}

/// Grille à deux colonnes de paires libellé / valeur
fn grid(ui: &mut Ui, rows: &[(&str, &str, bool)]) {
    let half = (ui.available_width() - 16.0) / 2.0;
    egui::Grid::new("host-grid")
        .num_columns(2)
        .spacing([16.0, 4.0])
        .show(ui, |ui| {
            for pair in rows.chunks(2) {
                for (label, value, copy) in pair {
                    ui.allocate_ui(egui::vec2(half, 20.0), |ui| {
                        ui.set_width(half);
                        info_row(ui, label, value, *copy, *copy);
                    });
                }
                ui.end_row();
            }
        });
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

/// Fenêtre « Affecter des VM » : cases à cocher des VM qui ne sont pas sur la machine
pub fn assign(ui: &Ui, store: &Store, st: &mut UiState) {
    let Some(state) = &mut st.assign_host else {
        return;
    };
    let Ok(host) = store.host(&state.host_id) else {
        st.assign_host = None;
        return;
    };
    let host_group = group_of(store, host.group_id.as_deref());
    let mut candidates: Vec<&Vm> = store
        .vms()
        .iter()
        .filter(|v| v.host_id.as_deref() != Some(host.id.as_str()))
        .collect();
    candidates.sort_by_key(|v| v.name.to_lowercase());
    let total = candidates.len();
    candidates.retain(|v| {
        vm_matches(
            v,
            group_of(store, v.group_id.as_deref()),
            host_of(store, v),
            &state.search,
        )
    });

    let selected = state.selected.len();
    let mut submit = false;
    let mut cancel = false;
    let closed = modal(
        ui,
        "host-assign",
        &format!("Affecter des VM à « {} »", host.name),
        Some("Coche les VM hébergées par cette machine"),
        620.0,
        |ui| {
            let note = match host_group {
                Some(g) => format!(
                    "Les VM affectées passent dans le groupe de la machine : « {} ».",
                    g.name
                ),
                None => "La machine n'a pas de groupe : les VM affectées deviennent non classées."
                    .to_string(),
            };
            warning_box(ui, &note);
            ui.add_space(6.0);
            let r = text_field(
                ui,
                &mut state.search,
                "🔍  Filtrer : nom, IP, groupe, machine…",
                false,
            );
            autofocus(&r, &mut state.focused);
            ui.add_space(6.0);

            if total == 0 {
                ui.label(
                    RichText::new("Toutes les VM de l'inventaire sont déjà sur cette machine.")
                        .color(p().text_muted),
                );
            } else if candidates.is_empty() {
                ui.label(RichText::new("Aucune VM ne correspond.").color(p().text_muted));
            }
            for vm in &candidates {
                let mut checked = state.selected.contains(&vm.id);
                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut checked, RichText::new(&vm.name).color(p().text_strong))
                        .changed()
                    {
                        state.toggle(&vm.id);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match host_of(store, vm) {
                            Some(h) => chip(ui, format!("🗄 {}", h.name), p().text_secondary),
                            None => chip(ui, "Aucune machine", p().text_muted),
                        }
                        match group_of(store, vm.group_id.as_deref()) {
                            Some(g) => chip(ui, &g.name, parse_hex(g.color.as_deref())),
                            None => chip(ui, "Non classée", p().text_muted),
                        }
                        if !vm.network.ip.is_empty() {
                            ui.label(
                                RichText::new(&vm.network.ip)
                                    .monospace()
                                    .color(p().text_secondary),
                            );
                        }
                    });
                });
            }
            error_text(ui, &state.error);
        },
        |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_enabled_ui(selected > 0, |ui| {
                    submit = primary_button(ui, &format!("Affecter ({selected})")).clicked();
                });
                cancel = ghost_button(ui, "Annuler").clicked();
            });
        },
    );
    if submit {
        let command = Command::AssignVms {
            host_id: state.host_id.clone(),
            vm_ids: state.vm_ids(),
        };
        st.run(command);
    } else if cancel || closed {
        st.assign_host = None;
    }
}

/// Fenêtre de création / modification d'une machine physique
pub fn form(ui: &Ui, store: &Store, st: &mut UiState) {
    let Some(form) = &mut st.host_form else {
        return;
    };
    let title = match &form.edit_id {
        Some(_) => format!("Modifier « {} »", form.name.trim()),
        None => "Nouvelle machine physique".to_string(),
    };
    // VM qui suivront un changement de groupe
    let (hosted, group_changed) = match &form.edit_id {
        Some(id) => (
            store.vms_on_host(id).len(),
            store.host(id).is_ok_and(|h| h.group_id != form.group_id),
        ),
        None => (0, false),
    };
    let mut submit = false;
    let mut cancel = false;
    let closed = modal(
        ui,
        "host-form",
        &title,
        None,
        680.0,
        |ui| {
            ui.columns(2, |c| {
                labeled(&mut c[0], "Nom *", None, |ui| {
                    let r = text_field(ui, &mut form.name, "pve-01", false);
                    autofocus(&r, &mut form.focused);
                });
                labeled(&mut c[1], "Groupe", None, |ui| {
                    let selected = group_of(store, form.group_id.as_deref())
                        .map(|g| g.name.clone())
                        .unwrap_or_else(|| "Sans groupe".into());
                    egui::ComboBox::from_id_salt("host-group")
                        .width(ui.available_width())
                        .selected_text(selected)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut form.group_id, None, "Sans groupe");
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
            if group_changed && hosted > 0 {
                ui.label(
                    RichText::new(format!(
                        "Ses {hosted} VM passeront aussi dans ce groupe à l'enregistrement."
                    ))
                    .small()
                    .color(p().warning),
                );
            }
            ui.columns(2, |c| {
                labeled(&mut c[0], "Hyperviseur", None, |ui| {
                    hypervisor_field(ui, &mut form.hypervisor);
                });
                labeled(&mut c[1], "Emplacement", Some("baie, salle…"), |ui| {
                    text_field(ui, &mut form.location, "Salle B · baie 3 · U12", false);
                });
            });
            ui.columns(2, |c| {
                labeled(&mut c[0], "Nom d'hôte", None, |ui| {
                    text_field(ui, &mut form.hostname, "pve-01.lan", true);
                });
                labeled(&mut c[1], "Adresse IP", Some("administration"), |ui| {
                    text_field(ui, &mut form.ip, "192.168.10.2", true);
                });
            });
            ui.columns(3, |c| {
                labeled(&mut c[0], "Processeur", None, |ui| {
                    text_field(ui, &mut form.cpu, "Xeon E-2236, 6 cœurs", false);
                });
                labeled(&mut c[1], "Mémoire vive (Go)", None, |ui| {
                    text_field(ui, &mut form.ram, "64", true);
                });
                labeled(&mut c[2], "Stockage", None, |ui| {
                    text_field(ui, &mut form.storage, "2 × 1 To NVMe", false);
                });
            });
            labeled(ui, "Description", None, |ui| {
                text_area(
                    ui,
                    &mut form.description,
                    "Rôle de la machine, cluster…",
                    2,
                    false,
                );
            });
            labeled(ui, "Notes", None, |ui| {
                text_area(ui, &mut form.notes, "", 3, true);
            });
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
    if submit {
        st.run(Command::SaveHost);
    } else if cancel || closed {
        st.host_form = None;
    }
}

/// Hyperviseur : saisie libre + menu des hyperviseurs courants
fn hypervisor_field(ui: &mut Ui, value: &mut String) {
    const BUTTON_WIDTH: f32 = 34.0;
    let width = ui.available_width() - BUTTON_WIDTH - ui.spacing().item_spacing.x;
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(value)
                .hint_text("Proxmox VE, VMware ESXi…")
                .desired_width(width)
                .margin(egui::Margin::symmetric(8, 6)),
        );
        ui.spacing_mut().button_padding.x = 6.0;
        let button = egui::Button::new(RichText::new("⏷").color(p().text_secondary))
            .fill(p().bg_secondary)
            .min_size(egui::vec2(BUTTON_WIDTH, 30.0));
        egui::containers::menu::MenuButton::from_button(button)
            .ui(ui, |ui| {
                ui.set_min_width(180.0);
                for name in HYPERVISORS {
                    if ui.selectable_label(value.trim() == name, name).clicked() {
                        *value = name.to_string();
                        ui.close();
                    }
                }
            })
            .0
            .on_hover_text("Hyperviseurs courants");
    });
}
