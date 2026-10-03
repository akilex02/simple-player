use super::glass::{paint_glass, GlassKind};
use crate::theme;
use eframe::egui;

/// Tarjeta de vidrio clicable que se eleva al pasar el mouse. El contenido se
/// dibuja dentro de un margen interno de `MD`.
pub fn card(ui: &mut egui::Ui, size: egui::Vec2, add_contents: impl FnOnce(&mut egui::Ui)) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if !ui.is_rect_visible(rect) {
        return response;
    }

    let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), theme::motion::HOVER * 1.5);
    let lifted = rect.translate(egui::vec2(0.0, -3.0 * hover));
    paint_glass(ui.painter(), lifted, GlassKind::Standard, theme::radius::LG, hover);

    let inner = lifted.shrink(theme::space::MD);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner).layout(egui::Layout::top_down(egui::Align::Center)));
    child.set_clip_rect(lifted);
    add_contents(&mut child);
    response
}
