use crate::theme::{self, with_alpha};
use eframe::egui;

/// Entrada de navegación del sidebar: ícono + texto; la activa lleva píldora
/// con el color de acento.
pub fn nav_item(ui: &mut egui::Ui, icon: &str, label: &str, active: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), theme::motion::HOVER);
    let accent = theme::accent(ui.ctx());

    let painter = ui.painter();
    let rounding = rect.height() / 2.0;
    if active {
        painter.rect_filled(rect, rounding, with_alpha(accent, 48));
        painter.rect_stroke(rect, rounding, egui::Stroke::new(1.0_f32, with_alpha(accent, 130)));
    } else if hover > 0.0 {
        painter.rect_filled(rect, rounding, egui::Color32::from_white_alpha((20.0 * hover) as u8));
    }

    let color = if active { accent } else { theme::lerp_color(theme::TEXT_MUTED, theme::TEXT_MAIN, hover) };
    painter.text(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(theme::text::LG),
        color,
    );
    painter.text(
        egui::pos2(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        if active { theme::bold(theme::text::BASE) } else { egui::FontId::proportional(theme::text::BASE) },
        color,
    );
    response
}
