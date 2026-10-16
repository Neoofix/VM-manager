/// Liste des VM : recherche, filtre par groupe, cartes regroupées par groupe
use egui::{RichText, Ui};
use vm_core::query::{ssh_command, vm_matches};
use vm_core::{Group, Store, Vm};

use super::widgets::*;
use crate::app::{GroupFilter, UiState};
use crate::forms::VmForm;
use crate::os_presets::family_of;
use crate::theme::*;

const CARD_WIDTH: f32 = 290.0;

pub fn show(ui: &mut Ui, store: &Store, st: &mut UiState) {
    let all = store.vms();
    let group_of = |vm: &Vm| -> Option<&Group> {
        vm.group_id
            .as_ref()
            .and_then(|id| store.groups().iter().find(|g| &g.id == id))
    };

    let mut filtered: Vec<&Vm> = all
        .iter()
        .filter(|v| match &st.group_filter {
            GroupFilter::All => true,
            GroupFilter::Ungrouped => v.group_id.is_none(),
            GroupFilter::Group(id) => v.group_id.as_ref() == Some(id),
        })
        .filter(|v| vm_matches(v, group_of(v), &st.search))
        .collect();
    filtered.sort_by_key(|v| v.name.to_lowercase());

    let subtitle = if filtered.len() == all.len() {
        format!("{} VM", all.len())
    } else {
        format!("{} sur {} VM", filtered.len(), all.len())
    };
    page_header(ui, "Machines virtuelles", &subtitle, |ui| {
        if primary_button(ui, "➕  Nouvelle VM").clicked() {
            let group = match &st.group_filter {
                GroupFilter::Group(id) => Some(id.clone()),
                _ => None,
            };
            st.vm_form = Some(VmForm::create(group));
        }
    });

    // Filtres
    ui.horizontal(|ui| {
        let search_width = ui.available_width() - 240.0;
        ui.add_sized(
            [search_width, 32.0],
            egui::TextEdit::singleline(&mut st.search)
                .hint_text("🔍  Rechercher : nom, IP, VLAN, étiquette, utilisateur…")
                .margin(egui::Margin::symmetric(10, 7)),
        );
        let selected = match &st.group_filter {
            GroupFilter::All => "Tous les groupes".to_string(),
            GroupFilter::Ungrouped => "Non classées".to_string(),
            GroupFilter::Group(id) => store
                .groups()
                .iter()
                .find(|g| &g.id == id)
                .map(|g| g.name.clone())
                .unwrap_or_default(),
        };
        egui::ComboBox::from_id_salt("group-filter")
            .width(220.0)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut st.group_filter, GroupFilter::All, "Tous les groupes");
                for g in sorted_groups(store) {
                    ui.selectable_value(
                        &mut st.group_filter,
                        GroupFilter::Group(g.id.clone()),
                        &g.name,
                    );
                }
                ui.selectable_value(&mut st.group_filter, GroupFilter::Ungrouped, "Non classées");
            });
    });
    ui.add_space(8.0);

    if all.is_empty() {
        if empty_state(
            ui,
            "🖥",
            "Aucune VM pour l'instant",
            "Ajoute ta première machine pour garder ses infos réseau, SSH et comptes sous la main.",
            "Ajouter une VM",
        ) {
            st.vm_form = Some(VmForm::create(None));
        }
        return;
    }
    if filtered.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(
                RichText::new("Aucune VM ne correspond à ces filtres.").color(p().text_secondary),
            );
        });
        return;
    }

    // Une section par groupe (ordre alphabétique), puis les VM non classées
    let mut sections: Vec<(Option<&Group>, Vec<&Vm>)> = sorted_groups(store)
        .into_iter()
        .map(|g| {
            let items = filtered
                .iter()
                .copied()
                .filter(|v| v.group_id.as_deref() == Some(g.id.as_str()))
                .collect();
            (Some(g), items)
        })
        .collect();
    sections.push((
        None,
        filtered
            .iter()
            .copied()
            .filter(|v| group_of(v).is_none())
            .collect(),
    ));

    for (group, items) in sections.into_iter().filter(|(_, items)| !items.is_empty()) {
        ui.horizontal(|ui| {
            color_dot(ui, parse_hex(group.and_then(|g| g.color.as_deref())));
            let name = group.map(|g| g.name.as_str()).unwrap_or("Non classées");
            ui.label(RichText::new(name).strong().color(p().text_secondary));
            ui.label(RichText::new(format!("({})", items.len())).color(p().text_muted));
        });
        // Cartes alignées en haut, renvoyées à la ligne quand la place manque
        ui.with_layout(
            egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
            |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
                for vm in items {
                    vm_card(ui, vm, group, st);
                }
            },
        );
        ui.add_space(12.0);
    }
}

pub fn sorted_groups(store: &Store) -> Vec<&Group> {
    let mut groups: Vec<&Group> = store.groups().iter().collect();
    groups.sort_by_key(|g| g.name.to_lowercase());
    groups
}

fn vm_card(ui: &mut Ui, vm: &Vm, group: Option<&Group>, st: &mut UiState) {
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
                        ui.label(RichText::new("🖥").size(18.0).color(p().accent_fg));
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&vm.name).strong().color(p().text_strong),
                                )
                                .truncate(),
                            );
                            let sub = [&vm.description, &vm.os]
                                .into_iter()
                                .find(|s| !s.is_empty())
                                .map(String::as_str)
                                .unwrap_or("—");
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
                        let address = [&vm.network.ip, &vm.network.hostname]
                            .into_iter()
                            .find(|s| !s.is_empty());
                        match address {
                            Some(a) => {
                                ui.label(RichText::new(a).monospace().color(p().text_strong))
                            }
                            None => ui.label(RichText::new("Aucune adresse").color(p().text_muted)),
                        };
                        if let Some(vlan) = vm.network.vlan {
                            chip(ui, format!("VLAN {vlan}"), p().text_secondary);
                        }
                    });
                    if let Some(cmd) = ssh_command(vm, None) {
                        ui.add(
                            egui::Label::new(
                                RichText::new(format!("› {cmd}"))
                                    .monospace()
                                    .small()
                                    .color(p().text_secondary),
                            )
                            .truncate(),
                        );
                    }
                    ui.horizontal_wrapped(|ui| {
                        let n = vm.users.len();
                        ui.label(
                            RichText::new(format!(
                                "👤 {n} utilisateur{}",
                                if n > 1 { "s" } else { "" }
                            ))
                            .small()
                            .color(p().text_secondary),
                        );
                        if let Some(family) = family_of(&vm.os) {
                            chip(
                                ui,
                                format!("{} {}", family.icon, family.name),
                                p().text_secondary,
                            );
                        }
                        for tag in &vm.tags {
                            chip(ui, tag, p().accent_2);
                        }
                    });
                })
            },
        )
        .inner;
    if response.clicked() {
        st.open_vm = Some(vm.id.clone());
    }
}
