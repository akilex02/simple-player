use crate::theme::{self, with_alpha};
use eframe::egui;

/// Interruptor de píldora: se enciende con el acento dinámico.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(44.0, 24.0), egui::Sense::click());
    response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_with_time(response.id, *on, theme::motion::HOVER);
        let accent = theme::accent(ui.ctx());
        let painter = ui.painter();
        let fill = theme::lerp_color(egui::Color32::from_white_alpha(30), with_alpha(accent, 200), t);
        painter.rect_filled(rect, rect.height() / 2.0, fill);
        painter.rect_stroke(rect, rect.height() / 2.0, egui::Stroke::new(1.0_f32, theme::GLASS_BORDER));
        let knob_x = rect.left() + 12.0 + t * (rect.width() - 24.0);
        painter.circle_filled(egui::pos2(knob_x, rect.center().y), 8.0, egui::Color32::WHITE);
    }
    response
}
