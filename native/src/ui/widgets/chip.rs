use crate::theme::{self, with_alpha};
use eframe::egui;

/// Etiqueta pequeña seleccionable (filtros, modos).
pub fn chip(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let accent = theme::accent(ui.ctx());
    let color = if selected { accent } else { theme::TEXT_MUTED };
    let galley = ui.painter().layout_no_wrap(label.to_string(), theme::bold(theme::text::SM), color);
    let size = egui::vec2(galley.size().x + 24.0, 28.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);

    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), theme::motion::HOVER);
    let painter = ui.painter();
    let rounding = rect.height() / 2.0;
    let fill = if selected {
        with_alpha(accent, 45)
    } else {
        egui::Color32::from_white_alpha((14.0 + 16.0 * hover) as u8)
    };
    painter.rect_filled(rect, rounding, fill);
    let stroke = if selected { accent } else { theme::lerp_color(theme::GLASS_BORDER, theme::TEXT_MUTED, hover) };
    painter.rect_stroke(rect, rounding, egui::Stroke::new(1.0_f32, stroke));
    painter.galley(egui::pos2(rect.center().x - galley.size().x / 2.0, rect.center().y - galley.size().y / 2.0), galley, color);
    response
}
