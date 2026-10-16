/// Écrans de l'application
mod dashboard;
mod groups;
mod settings;
mod titlebar;
mod vault;
mod vm_detail;
mod vm_form;
mod vms;
pub mod widgets;

pub use vault::unlock_screen;

use std::path::Path;

use egui::{Button, Color32, CornerRadius, Frame, Margin, RichText, Stroke, Ui};
use vm_core::Store;

use crate::app::{Page, Toast, UiState};
use crate::paths::DataSource;
use crate::theme::*;
use widgets::{danger_button, ghost_button};

const NAV: [(Page, &str, &str); 4] = [
    (Page::Dashboard, "🏠", "Tableau de bord"),
    (Page::Vms, "🖥", "Machines virtuelles"),
    (Page::Groups, "📁", "Groupes"),
    (Page::Settings, "⚙", "Paramètres"),
];

pub fn show(ui: &mut Ui, store: &Store, data_path: &Path, source: DataSource, st: &mut UiState) {
    // Barre de titre en premier : elle occupe toute la largeur, au-dessus de la barre latérale
    titlebar::show(ui, Some((store, &mut *st)));
    sidebar(ui, store, st);

    egui::CentralPanel::no_frame().show(ui, |ui| {
        Frame::new()
            .fill(p().bg_primary)
            .inner_margin(Margin::same(24))
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match st.page {
                        Page::Dashboard => dashboard::show(ui, store, st),
                        Page::Vms => vms::show(ui, store, st),
                        Page::Groups => groups::show(ui, store, st),
                        Page::Settings => settings::show(ui, store, data_path, source, st),
                    });
            });
    });

    // Fenêtres modales, au-dessus de la page
    if st.vm_form.is_some() {
        vm_form::show(ui, store, st);
    } else if st.page == Page::Vms && st.open_vm.is_some() && st.confirm.is_none() {
        vm_detail::show(ui, store, st);
    }
    if st.group_form.is_some() {
        groups::form(ui, st);
    }
    confirm_dialog(ui, st);
    // Coffre en dernier : il s'ouvre par-dessus la fiche ou le formulaire qui l'a demandé
    vault::dialog(ui, st);
    toasts(ui, st);
    titlebar::resize_handles(ui);
}

fn sidebar(ui: &mut Ui, store: &Store, st: &mut UiState) {
    egui::Panel::left("sidebar")
        .exact_size(224.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(p().bg_secondary)
                .stroke(Stroke::new(1.0, p().border))
                .inner_margin(Margin::same(12)),
        )
        .show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                Frame::new()
                    .fill(p().accent.linear_multiply(0.15))
                    .corner_radius(CornerRadius::same(RADIUS))
                    .inner_margin(Margin::same(6))
                    .show(ui, |ui| {
                        ui.label(RichText::new("🖧").size(18.0).color(p().accent_fg))
                    });
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.label(RichText::new("VM Manager").strong().color(p().text_strong));
                    ui.label(
                        RichText::new("Inventaire & mémo")
                            .small()
                            .color(p().text_secondary),
                    );
                });
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            for (page, icon, label) in NAV {
                let active = st.page == page;
                let text = RichText::new(format!("{icon}   {label}")).color(if active {
                    Color32::WHITE
                } else {
                    p().text_secondary
                });
                let button = Button::new(text)
                    .fill(if active {
                        p().accent
                    } else {
                        Color32::TRANSPARENT
                    })
                    .stroke(Stroke::NONE)
                    .min_size(egui::vec2(ui.available_width(), 34.0));
                if ui.add(button).clicked() {
                    st.page = page;
                    if page == Page::Vms {
                        st.open_vm = None;
                    }
                }
            }

            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                let groups = store.groups().len();
                ui.label(
                    RichText::new(format!(
                        "{} VM · {groups} groupe{}",
                        store.vms().len(),
                        if groups > 1 { "s" } else { "" }
                    ))
                    .small()
                    .color(p().text_secondary),
                );
                ui.separator();
            });
        });
}

fn confirm_dialog(ui: &Ui, st: &mut UiState) {
    let Some(confirm) = &st.confirm else { return };
    let mut decision = None;
    let closed = widgets::modal(
        ui,
        "confirm",
        &confirm.title,
        None,
        440.0,
        |ui| {
            ui.horizontal_top(|ui| {
                ui.label(RichText::new("⚠").size(22.0).color(p().error));
                ui.label(RichText::new(&confirm.message).color(p().text));
            });
        },
        |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if danger_button(ui, confirm.button).clicked() {
                    decision = Some(true);
                }
                if ghost_button(ui, "Annuler").clicked() {
                    decision = Some(false);
                }
            });
        },
    );
    if decision == Some(true) {
        let confirm = st.confirm.take().expect("confirmation présente");
        st.run(confirm.command);
    } else if closed || decision == Some(false) {
        st.confirm = None;
    }
}

fn toasts(ui: &Ui, st: &mut UiState) {
    let now = ui.input(|i| i.time);
    for (success, message) in std::mem::take(&mut st.pending_toasts) {
        let ttl = if success { 3.0 } else { 6.0 };
        st.toasts.push(Toast {
            success,
            message,
            expires_at: now + ttl,
        });
    }
    st.toasts.retain(|t| t.expires_at > now);
    if st.toasts.is_empty() {
        return;
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(250));

    egui::Area::new(egui::Id::unique("toasts"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
        .order(egui::Order::Tooltip)
        .show(ui.ctx(), |ui| {
            ui.set_max_width(340.0);
            let mut dismissed = None;
            for (i, t) in st.toasts.iter().enumerate() {
                let color = if t.success { p().success } else { p().error };
                Frame::new()
                    .fill(p().bg_tertiary)
                    .stroke(Stroke::new(1.0, color.linear_multiply(0.6)))
                    .corner_radius(CornerRadius::same(RADIUS))
                    .inner_margin(Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_width(316.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(if t.success { "✔" } else { "✖" }).color(color));
                            ui.add(
                                egui::Label::new(RichText::new(&t.message).color(p().text_strong))
                                    .wrap(),
                            );
                            if widgets::icon_button(ui, "✖", "Fermer").clicked() {
                                dismissed = Some(i);
                            }
                        });
                    });
            }
            if let Some(i) = dismissed {
                st.toasts.remove(i);
            }
        });
}

/// Écran affiché si l'inventaire n'a pas pu être chargé
pub fn load_error(ui: &mut Ui, error: &str, data_path: &Path) {
    // Sans décorations natives, la barre de titre est le seul moyen de fermer la fenêtre
    titlebar::show(ui, None);
    titlebar::resize_handles(ui);
    egui::CentralPanel::default().show(ui, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(120.0);
            ui.label(RichText::new("⚠").size(40.0).color(p().error));
            ui.label(
                RichText::new("Impossible de charger l'inventaire")
                    .heading()
                    .color(p().text_strong),
            );
            ui.add_space(8.0);
            ui.label(RichText::new(error).color(p().text));
            ui.label(
                RichText::new(data_path.display().to_string())
                    .monospace()
                    .color(p().text_secondary),
            );
            ui.add_space(8.0);
            ui.label(
                RichText::new(
                    "Lance l'application avec --data <fichier> pour utiliser un autre inventaire.",
                )
                .color(p().text_secondary),
            );
        });
    });
}
