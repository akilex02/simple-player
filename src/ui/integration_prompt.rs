//! Aviso de «Registrar en el menú de aplicaciones» (solo como AppImage, una vez, descartable).
use crate::state::AppState;
use crate::theme::{self, radius, space, text};
use crate::ui::shell::player_bar;
use crate::ui::widgets::glass::shadow;
use crate::ui::widgets::pill_button::{PillButton, PillKind};
use eframe::egui::{self, RichText};

enum Choice {
    Register,
    NotNow,
    Never,
}

pub fn show(ctx: &egui::Context, state: &mut AppState) {
    if !state.should_prompt_integration() {
        return;
    }
    let error_id = egui::Id::new("integration_prompt_error");
    let mut choice = None;
    egui::Area::new(egui::Id::new("integration_prompt"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-space::XL, -(player_bar::HEIGHT + space::XL)))
        .show(ctx, |ui| {
            // Opaca (no vidrio): flota sobre la lista y no debe dejar ver las filas de debajo.
            egui::Frame::none()
                .fill(theme::BG_CARD)
                .stroke(egui::Stroke::new(1.0_f32, theme::GLASS_BORDER))
                .rounding(radius::LG)
                .inner_margin(space::LG)
                .shadow(shadow(1.0))
                .show(ui, |ui| {
                ui.set_max_width(380.0);
                ui.label(RichText::new("Registrar en el menú de aplicaciones").font(theme::bold(text::MD)));
                ui.add_space(space::XS);
                ui.label(
                    RichText::new("Así el escritorio muestra el ícono de Simple Player en la barra de tareas y los controles multimedia.")
                        .size(text::SM)
                        .color(theme::TEXT_MUTED),
                );
                ui.add_space(space::MD);
                ui.horizontal(|ui| {
                    if PillButton::new("Registrar", PillKind::Primary).show(ui).clicked() {
                        choice = Some(Choice::Register);
                    }
                    if PillButton::new("Ahora no", PillKind::Secondary).show(ui).clicked() {
                        choice = Some(Choice::NotNow);
                    }
                });
                if PillButton::new("No volver a preguntar", PillKind::Ghost).show(ui).clicked() {
                    choice = Some(Choice::Never);
                }
                if let Some(error) = ctx.data(|d| d.get_temp::<String>(error_id)) {
                    ui.add_space(space::XS);
                    ui.label(RichText::new(error).size(text::SM).color(egui::Color32::from_rgb(255, 120, 120)));
                }
                });
        });
    match choice {
        Some(Choice::Register) => match state.register_desktop() {
            Ok(()) => ctx.data_mut(|d| d.remove::<String>(error_id)),
            Err(e) => ctx.data_mut(|d| d.insert_temp(error_id, e)),
        },
        Some(Choice::NotNow) => state.dismiss_integration_prompt(false),
        Some(Choice::Never) => state.dismiss_integration_prompt(true),
        None => {}
    }
}
