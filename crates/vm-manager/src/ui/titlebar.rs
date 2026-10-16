/// Barre de titre de l'application, aux couleurs du thème.
///
/// La fenêtre est créée sans décorations natives (voir `main.rs`) : cette barre
/// permet donc de la déplacer, de l'agrandir (double clic), de la réduire et de
/// la fermer, et [`resize_handles`] la rend redimensionnable par ses bords.
use egui::viewport::ResizeDirection;
use egui::{
    Button, CornerRadius, CursorIcon, Frame, Layout, PointerButton, Pos2, Rect, RichText, Sense,
    Shape, Stroke, StrokeKind, Ui, Vec2, ViewportCommand,
};

use super::widgets::{color_dot, section_title};
use super::NAV;
use crate::app::{Command, Page, UiState};
use crate::forms::VmForm;
use crate::theme::{self, *};

pub const HEIGHT: f32 = 38.0;
const CONTROL_WIDTH: f32 = 46.0;
/// Épaisseur de la zone de redimensionnement sur chaque bord
const RESIZE_EDGE: f32 = 6.0;
/// Longueur de la zone d'angle (redimensionnement en diagonale)
const RESIZE_CORNER: f32 = 14.0;

#[derive(Clone, Copy, PartialEq)]
enum Control {
    Minimize,
    Maximize,
    Close,
}

/// Affiche la barre en haut de la fenêtre, au-dessus de la barre latérale et de la page.
/// Sans état (`None`, écran d'erreur de chargement) : seulement le titre et les contrôles.
pub fn show(ui: &mut Ui, st: Option<&mut UiState>) {
    egui::Panel::top("titlebar")
        .exact_size(HEIGHT)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(p().bg_secondary))
        .show(ui, |ui| {
            let bar = ui.max_rect();
            let maximized = ui.input(|i| i.viewport().maximized.unwrap_or(false));
            let resizing = resize_direction(ui).is_some();

            // Fond de la barre : déplacement de la fenêtre et agrandissement au double clic.
            // Les boutons ajoutés ensuite passent devant.
            let sense = if resizing {
                Sense::hover()
            } else {
                Sense::click_and_drag()
            };
            let drag = ui.interact(bar, ui.scope_id().with("titlebar-drag"), sense);
            if drag.double_clicked() {
                ui.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
            } else if drag.drag_started_by(PointerButton::Primary) {
                ui.send_viewport_cmd(ViewportCommand::StartDrag);
            }
            ui.painter().hline(
                bar.x_range(),
                bar.bottom() - 0.5,
                Stroke::new(1.0, p().border),
            );

            ui.with_layout(Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(14.0);
                ui.label(RichText::new("🖧").size(15.0).color(p().accent_fg));
                ui.label(RichText::new("VM Manager").strong().color(p().text_strong));
                if let Some(page) = st.as_ref().map(|st| st.page) {
                    ui.label(RichText::new("›").color(p().text_muted));
                    ui.label(RichText::new(page_name(page)).color(p().text_secondary));
                }

                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        for control in [Control::Close, Control::Maximize, Control::Minimize] {
                            if window_control(ui, control, maximized, resizing).clicked() {
                                ui.send_viewport_cmd(match control {
                                    Control::Minimize => ViewportCommand::Minimized(true),
                                    Control::Maximize => ViewportCommand::Maximized(!maximized),
                                    Control::Close => ViewportCommand::Close,
                                });
                            }
                        }
                    });
                    if let Some(st) = st {
                        ui.add_space(6.0);
                        quick_actions(ui, st);
                    }
                });
            });
        });
}

fn page_name(page: Page) -> &'static str {
    NAV.iter()
        .find(|(p, _, _)| *p == page)
        .map(|(_, _, label)| *label)
        .unwrap_or_default()
}

/// Actions rapides, de droite à gauche : coffre, thème, nouvelle VM
fn quick_actions(ui: &mut Ui, st: &mut UiState) {
    ui.spacing_mut().interact_size.y = 26.0;
    ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);
    ui.spacing_mut().item_spacing.x = 6.0;

    vault_button(ui, st);
    theme_menu(ui, st);

    let new_vm = Button::new(
        RichText::new("➕  Nouvelle VM")
            .color(p().text_strong)
            .strong(),
    )
    .fill(p().accent)
    .stroke(Stroke::NONE);
    if ui
        .add(new_vm)
        .on_hover_text("Ajouter une machine virtuelle")
        .clicked()
    {
        st.vm_form = Some(VmForm::create(None));
        st.page = Page::Vms;
    }
}

// coffre : bouton ajouté plus tard (verrouillage du coffre chiffré)
fn vault_button(_ui: &mut Ui, _st: &mut UiState) {}

/// Menu de choix du thème ; le choix passe par `Command::SetTheme` pour être enregistré
fn theme_menu(ui: &mut Ui, st: &mut UiState) {
    let button = Button::new(RichText::new("🎨").color(p().text_secondary))
        .frame_when_inactive(false)
        .min_size(Vec2::new(32.0, 26.0));
    let active = theme::current().id;
    egui::containers::menu::MenuButton::from_button(button)
        .ui(ui, |ui| {
            ui.set_min_width(200.0);
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.spacing_mut().interact_size.y = 24.0;
            ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
            section_title(ui, "Thème");
            for (i, t) in THEMES.iter().enumerate() {
                let selected = t.id == active;
                let row = ui.horizontal(|ui| {
                    color_dot(ui, t.accent);
                    ui.add(
                        egui::Button::selectable(selected, t.name)
                            .min_size(Vec2::new(170.0, 0.0))
                            .right_text(if selected { "✔" } else { "" }),
                    )
                });
                if row.inner.clicked() {
                    if !selected {
                        st.run(Command::SetTheme(i));
                    }
                    ui.close();
                }
            }
        })
        .0
        .on_hover_text("Changer de thème");
}

/// Bouton de contrôle de la fenêtre, icône dessinée au pinceau
fn window_control(
    ui: &mut Ui,
    control: Control,
    maximized: bool,
    resizing: bool,
) -> egui::Response {
    // Dans une zone de redimensionnement, le clic appartient au bord de la fenêtre
    let sense = if resizing {
        Sense::hover()
    } else {
        Sense::click()
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(CONTROL_WIDTH, HEIGHT - 1.0), sense);
    let hot = response.hovered() && !resizing;
    let painter = ui.painter();
    let fg = match (control, hot) {
        (Control::Close, true) => {
            painter.rect_filled(rect, CornerRadius::ZERO, p().error);
            p().bg_secondary
        }
        (_, true) => {
            painter.rect_filled(rect, CornerRadius::ZERO, p().bg_hover);
            p().text_strong
        }
        _ => p().text_secondary,
    };

    let stroke = Stroke::new(1.0, fg);
    let c = Pos2::new(
        painter.round_to_pixel_center(rect.center().x),
        painter.round_to_pixel_center(rect.center().y),
    );
    let s = 5.0;
    match control {
        Control::Minimize => {
            painter.hline(c.x - s..=c.x + s, c.y, stroke);
        }
        Control::Maximize if maximized => {
            // Deux fenêtres superposées : « restaurer »
            let front = Rect::from_min_size(c + Vec2::new(-s, -s + 2.0), Vec2::splat(8.0));
            painter.rect_stroke(front, CornerRadius::ZERO, stroke, StrokeKind::Middle);
            painter.add(Shape::line(
                vec![
                    front.left_top() + Vec2::new(2.0, 0.0),
                    front.left_top() + Vec2::new(2.0, -2.0),
                    front.right_top() + Vec2::new(2.0, -2.0),
                    front.right_bottom() + Vec2::new(2.0, -2.0),
                    front.right_bottom() + Vec2::new(0.0, -2.0),
                ],
                stroke,
            ));
        }
        Control::Maximize => {
            let square = Rect::from_center_size(c, Vec2::splat(2.0 * s));
            painter.rect_stroke(square, CornerRadius::ZERO, stroke, StrokeKind::Middle);
        }
        Control::Close => {
            painter.line_segment([c + Vec2::new(-s, -s), c + Vec2::new(s, s)], stroke);
            painter.line_segment([c + Vec2::new(-s, s), c + Vec2::new(s, -s)], stroke);
        }
    }

    response.on_hover_text(match control {
        Control::Minimize => "Réduire",
        Control::Maximize if maximized => "Restaurer",
        Control::Maximize => "Agrandir",
        Control::Close => "Fermer",
    })
}

/// Bord de la fenêtre sous le pointeur (fenêtre ni agrandie ni en plein écran)
fn resize_direction(ui: &Ui) -> Option<ResizeDirection> {
    let (pos, fixed) = ui.input(|i| {
        let v = i.viewport();
        (
            i.pointer.hover_pos(),
            v.maximized.unwrap_or(false) || v.fullscreen.unwrap_or(false),
        )
    });
    if fixed {
        return None;
    }
    let pos = pos?;
    let r = ui.ctx().viewport_rect();
    let (left, right) = (pos.x - r.left(), r.right() - pos.x);
    let (top, bottom) = (pos.y - r.top(), r.bottom() - pos.y);
    let edge = |d: f32| (0.0..=RESIZE_EDGE).contains(&d);
    let corner = |d: f32| (0.0..=RESIZE_CORNER).contains(&d);
    use ResizeDirection::*;
    Some(match () {
        _ if (edge(top) && corner(left)) || (edge(left) && corner(top)) => NorthWest,
        _ if (edge(top) && corner(right)) || (edge(right) && corner(top)) => NorthEast,
        _ if (edge(bottom) && corner(left)) || (edge(left) && corner(bottom)) => SouthWest,
        _ if (edge(bottom) && corner(right)) || (edge(right) && corner(bottom)) => SouthEast,
        _ if edge(top) => North,
        _ if edge(bottom) => South,
        _ if edge(left) => West,
        _ if edge(right) => East,
        _ => return None,
    })
}

/// Poignées de redimensionnement sur les bords et les angles de la fenêtre.
/// À appeler après le reste de l'interface, pour que le curseur choisi l'emporte.
pub fn resize_handles(ui: &Ui) {
    let Some(direction) = resize_direction(ui) else {
        return;
    };
    use ResizeDirection::*;
    ui.ctx().set_cursor_icon(match direction {
        North => CursorIcon::ResizeNorth,
        South => CursorIcon::ResizeSouth,
        East => CursorIcon::ResizeEast,
        West => CursorIcon::ResizeWest,
        NorthEast => CursorIcon::ResizeNorthEast,
        NorthWest => CursorIcon::ResizeNorthWest,
        SouthEast => CursorIcon::ResizeSouthEast,
        SouthWest => CursorIcon::ResizeSouthWest,
    });
    if ui.input(|i| i.pointer.primary_pressed()) {
        ui.send_viewport_cmd(ViewportCommand::BeginResize(direction));
    }
}
