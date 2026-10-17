/// Liste des VM : recherche, filtres par groupe et par machine physique, cartes
/// en arborescence groupe → machine physique → VM
use egui::{RichText, Ui};
use vm_core::query::{ssh_command, vm_matches};
use vm_core::{Group, Host, Store, Vm};

use super::widgets::*;
use crate::app::{GroupFilter, UiState};
use crate::forms::VmForm;
use crate::os_presets::family_of;
use crate::theme::*;

const CARD_WIDTH: f32 = 290.0;

/// Filtre de machine physique de la liste des VM. Gardé dans la mémoire egui
/// (et non dans `UiState`) : il ne concerne que cet écran.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HostFilter {
    #[default]
    All,
    NoHost,
    Host(String),
}

fn host_filter_id() -> egui::Id {
    egui::Id::unique("vm-host-filter")
}

pub fn host_filter(ctx: &egui::Context) -> HostFilter {
    ctx.data(|d| d.get_temp(host_filter_id()))
        .unwrap_or_default()
}

pub fn set_host_filter(ctx: &egui::Context, filter: HostFilter) {
    ctx.data_mut(|d| d.insert_temp(host_filter_id(), filter));
}

pub fn show(ui: &mut Ui, store: &Store, st: &mut UiState) {
    let all = store.vms();
    let group_of = |vm: &Vm| -> Option<&Group> {
        vm.group_id
            .as_ref()
            .and_then(|id| store.groups().iter().find(|g| &g.id == id))
    };

    let host_of = |vm: &Vm| -> Option<&Host> {
        vm.host_id
            .as_ref()
            .and_then(|id| store.hosts().iter().find(|h| &h.id == id))
    };

    // Machines proposées dans le filtre : celles du groupe filtré
    let filter_hosts: Vec<&Host> = sorted_hosts(store)
        .into_iter()
        .filter(|h| match &st.group_filter {
            GroupFilter::All => true,
            GroupFilter::Ungrouped => h.group_id.is_none(),
            GroupFilter::Group(id) => h.group_id.as_ref() == Some(id),
        })
        .collect();
    let mut host_filter = host_filter(ui.ctx());
    if let HostFilter::Host(id) = &host_filter {
        // Machine supprimée ou hors du groupe filtré : filtre réinitialisé
        if !filter_hosts.iter().any(|h| &h.id == id) {
            host_filter = HostFilter::All;
        }
    }

    let mut filtered: Vec<&Vm> = all
        .iter()
        .filter(|v| match &st.group_filter {
            GroupFilter::All => true,
            GroupFilter::Ungrouped => v.group_id.is_none(),
            GroupFilter::Group(id) => v.group_id.as_ref() == Some(id),
        })
        .filter(|v| match &host_filter {
            HostFilter::All => true,
            HostFilter::NoHost => host_of(v).is_none(),
            HostFilter::Host(id) => v.host_id.as_ref() == Some(id),
        })
        .filter(|v| vm_matches(v, group_of(v), host_of(v), &st.search))
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
            let host = match &host_filter {
                HostFilter::Host(id) => store.hosts().iter().find(|h| &h.id == id),
                _ => None,
            };
            st.vm_form = Some(VmForm::create_on(group, host));
        }
    });

    // Filtres
    ui.horizontal(|ui| {
        const COMBO_WIDTH: f32 = 200.0;
        let search_width = ui.available_width() - 2.0 * (COMBO_WIDTH + 20.0);
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
            .width(COMBO_WIDTH)
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
        let selected = match &host_filter {
            HostFilter::All => "Toutes les machines".to_string(),
            HostFilter::NoHost => "Sans machine".to_string(),
            HostFilter::Host(id) => filter_hosts
                .iter()
                .find(|h| &h.id == id)
                .map(|h| format!("🖧 {}", h.name))
                .unwrap_or_default(),
        };
        egui::ComboBox::from_id_salt("host-filter")
            .width(COMBO_WIDTH)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut host_filter, HostFilter::All, "Toutes les machines");
                ui.selectable_value(&mut host_filter, HostFilter::NoHost, "Sans machine");
                for h in &filter_hosts {
                    ui.selectable_value(
                        &mut host_filter,
                        HostFilter::Host(h.id.clone()),
                        format!("🖧 {}", host_label(h)),
                    );
                }
            });
    });
    set_host_filter(ui.ctx(), host_filter);
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

    // Arborescence : un groupe (ordre alphabétique, puis « Non classées »), ses
    // machines physiques avec leurs VM, puis les VM du groupe sans machine
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

        let group_id = group.map(|g| g.id.as_str());
        let hosts: Vec<&Host> = sorted_hosts(store)
            .into_iter()
            .filter(|h| h.group_id.as_deref() == group_id)
            .collect();
        let mut shown_hosts = 0;
        for host in &hosts {
            let on_host: Vec<&Vm> = items
                .iter()
                .copied()
                .filter(|v| v.host_id.as_deref() == Some(host.id.as_str()))
                .collect();
            if on_host.is_empty() {
                continue;
            }
            shown_hosts += 1;
            let title = RichText::new(format!("🖧  {}  ({})", host_label(host), on_host.len()))
                .strong()
                .color(p().text);
            egui::CollapsingHeader::new(title)
                .id_salt(("vm-host-section", &host.id))
                .default_open(true)
                .show(ui, |ui| {
                    cards(ui, &on_host, group, host_of, st);
                });
        }

        let without_host: Vec<&Vm> = items
            .iter()
            .copied()
            .filter(|v| {
                !hosts
                    .iter()
                    .any(|h| v.host_id.as_deref() == Some(h.id.as_str()))
            })
            .collect();
        if !without_host.is_empty() {
            if shown_hosts > 0 {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("Sans machine physique  ({})", without_host.len()))
                        .small()
                        .strong()
                        .color(p().text_muted),
                );
            }
            cards(ui, &without_host, group, host_of, st);
        }
        ui.add_space(12.0);
    }
}

/// Cartes alignées en haut, renvoyées à la ligne quand la place manque.
/// `host_of` : machine physique affichée sur chaque carte.
fn cards<'a>(
    ui: &mut Ui,
    vms: &[&'a Vm],
    group: Option<&Group>,
    host_of: impl Fn(&'a Vm) -> Option<&'a Host>,
    st: &mut UiState,
) {
    ui.with_layout(
        egui::Layout::left_to_right(egui::Align::Min).with_main_wrap(true),
        |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(12.0, 12.0);
            for vm in vms {
                vm_card(ui, vm, group, host_of(vm), st);
            }
        },
    );
}

pub fn sorted_groups(store: &Store) -> Vec<&Group> {
    let mut groups: Vec<&Group> = store.groups().iter().collect();
    groups.sort_by_key(|g| g.name.to_lowercase());
    groups
}

pub fn sorted_hosts(store: &Store) -> Vec<&Host> {
    let mut hosts: Vec<&Host> = store.hosts().iter().collect();
    hosts.sort_by_key(|h| h.name.to_lowercase());
    hosts
}

/// « nom — hyperviseur » (nom seul si l'hyperviseur n'est pas renseigné)
pub fn host_label(host: &Host) -> String {
    if host.hypervisor.is_empty() {
        host.name.clone()
    } else {
        format!("{} — {}", host.name, host.hypervisor)
    }
}

fn vm_card(ui: &mut Ui, vm: &Vm, group: Option<&Group>, host: Option<&Host>, st: &mut UiState) {
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
                        if let Some(h) = host {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(format!("🖧 {}", h.name))
                                                .small()
                                                .color(p().text_secondary),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(format!("Machine physique : {}", host_label(h)));
                                },
                            );
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
