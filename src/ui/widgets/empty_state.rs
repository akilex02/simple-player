use super::pill_button::{PillButton, PillKind};
use crate::theme::{self, space, text};
use eframe::egui::{self, RichText};

/// Estado vacío: ícono en un círculo de vidrio, título, explicación y, si se
/// da, un botón de acción. Devuelve `true` si se pulsó la acción.
pub fn empty_state(ui: &mut egui::Ui, icon: &str, title: &str, subtitle: &str, cta: Option<&str>) -> bool {
    let mut clicked = false;
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() * 0.18).clamp(24.0, 160.0));
        let (rect, _) = ui.allocate_exact_size(egui::vec2(96.0, 96.0), egui::Sense::hover());
        let accent = theme::accent(ui.ctx());
        ui.painter().circle_filled(rect.center(), 48.0, egui::Color32::from_white_alpha(16));
        ui.painter().circle_stroke(rect.center(), 48.0, egui::Stroke::new(1.0_f32, theme::GLASS_BORDER));
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(40.0), accent);
        ui.add_space(space::LG);
        ui.label(RichText::new(title).font(theme::bold(text::LG)));
        ui.add_space(space::XS);
        ui.label(RichText::new(subtitle).color(theme::TEXT_MUTED));
        if let Some(label) = cta {
            ui.add_space(space::LG);
            clicked = PillButton::new(label, PillKind::Primary).show(ui).clicked();
        }
    });
    clicked
}
