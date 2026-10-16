/// Tableau de bord : compteurs, répartition par groupe, dernières modifications
use std::collections::HashSet;

use egui::{CornerRadius, RichText, Sense, Ui};
use vm_core::Store;

use super::widgets::*;
use crate::app::{GroupFilter, Page, UiState};
use crate::theme::*;

pub fn show(ui: &mut Ui, store: &Store, st: &mut UiState) {
    let vms = store.vms();
    page_header(
        ui,
        "Tableau de bord",
        "Vue d'ensemble de l'inventaire",
        |_| {},
    );

    let users: usize = vms.iter().map(|v| v.users.len()).sum();
    let vlans: HashSet<u16> = vms.iter().filter_map(|v| v.network.vlan).collect();
    let tiles = [
        ("🖥", vms.len(), "Machines virtuelles", Page::Vms),
        ("📁", store.groups().len(), "Groupes", Page::Groups),
        ("👤", users, "Comptes utilisateurs", Page::Vms),
        ("🔀", vlans.len(), "VLAN distincts", Page::Vms),
    ];
    ui.columns(4, |cols| {
        for (col, (icon, value, label, page)) in cols.iter_mut().zip(tiles) {
            let r = clickable_card(col, |ui| {
                ui.set_width(ui.available_width());
                ui.horizontal(|ui| {
                    egui::Frame::new()
                        .fill(p().accent.linear_multiply(0.12))
                        .corner_radius(CornerRadius::same(RADIUS))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.label(RichText::new(icon).size(20.0).color(p().accent_fg))
                        });
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            RichText::new(value.to_string())
                                .size(24.0)
                                .strong()
                                .color(p().text_strong),
                        );
                        ui.label(RichText::new(label).small().color(p().text_secondary));
                    });
                });
            });
            if r.clicked() {
                st.page = page;
                st.group_filter = GroupFilter::All;
            }
        }
    });
    ui.add_space(8.0);

    ui.columns(2, |cols| {
        // Répartition par groupe
        card_frame().show(&mut cols[0], |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Répartition par groupe")
                    .strong()
                    .color(p().text_strong),
            );
            ui.add_space(6.0);
            if vms.is_empty() {
                ui.label(RichText::new("Aucune VM enregistrée.").color(p().text_muted));
                return;
            }
            let mut rows: Vec<(String, egui::Color32, usize, GroupFilter)> = store
                .groups()
                .iter()
                .map(|g| {
                    let count = vms
                        .iter()
                        .filter(|v| v.group_id.as_deref() == Some(&g.id))
                        .count();
                    (
                        g.name.clone(),
                        parse_hex(g.color.as_deref()),
                        count,
                        GroupFilter::Group(g.id.clone()),
                    )
                })
                .collect();
            rows.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
            let ungrouped = vms.iter().filter(|v| v.group_id.is_none()).count();
            if ungrouped > 0 {
                rows.push((
                    "Non classées".into(),
                    p().text_muted,
                    ungrouped,
                    GroupFilter::Ungrouped,
                ));
            }
            let max = rows.iter().map(|r| r.2).max().unwrap_or(1).max(1);
            for (name, color, count, filter) in rows {
                let r = ui
                    .scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
                        ui.horizontal(|ui| {
                            color_dot(ui, color);
                            ui.label(RichText::new(&name).color(p().text_strong));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(count.to_string()).color(p().text_secondary),
                                    );
                                },
                            );
                        });
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 6.0),
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 3.0, p().bg_secondary);
                        let mut bar = rect;
                        bar.set_width(rect.width() * count as f32 / max as f32);
                        ui.painter().rect_filled(bar, 3.0, color);
                    })
                    .response
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if r.clicked() {
                    st.open_vms(filter);
                }
                ui.add_space(4.0);
            }
        });

        // Dernières modifications
        card_frame().show(&mut cols[1], |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                RichText::new("Modifiées récemment")
                    .strong()
                    .color(p().text_strong),
            );
            ui.add_space(6.0);
            let mut recent: Vec<_> = vms.iter().collect();
            recent.sort_by_key(|v| std::cmp::Reverse(v.updated_at));
            if recent.is_empty() {
                ui.label(RichText::new("Rien pour l'instant.").color(p().text_muted));
            }
            for vm in recent.into_iter().take(6) {
                let r = ui
                    .scope_builder(egui::UiBuilder::new().sense(Sense::click()), |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&vm.name).strong().color(p().text_strong));
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let when = vm.updated_at.with_timezone(&chrono::Local);
                                    ui.label(
                                        RichText::new(when.format("%d/%m/%Y %H:%M").to_string())
                                            .small()
                                            .color(p().text_muted),
                                    );
                                    let ip = if vm.network.ip.is_empty() {
                                        "—"
                                    } else {
                                        &vm.network.ip
                                    };
                                    ui.label(
                                        RichText::new(ip).monospace().color(p().text_secondary),
                                    );
                                },
                            );
                        });
                    })
                    .response
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if r.clicked() {
                    st.open_vm(&vm.id);
                }
                ui.separator();
            }
        });
    });
}
