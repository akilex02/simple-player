pub mod card;
pub mod chip;
pub mod cover;
pub mod empty_state;
pub mod glass;
pub mod gradient;
pub mod icon_button;
pub mod nav_item;
pub mod pill_button;
pub mod slider;

use eframe::egui;

/// Clic sin entrar en el orden de Tab: para filas y tarjetas de listas largas.
pub fn click_without_focus() -> egui::Sense {
    egui::Sense { click: true, drag: false, focusable: false }
}

/// Aro de foco de teclado alrededor de un control.
pub fn paint_focus_ring(painter: &egui::Painter, rect: egui::Rect, rounding: f32, color: egui::Color32) {
    painter.rect_stroke(rect.expand(3.0), rounding + 3.0, egui::Stroke::new(2.0_f32, color));
}
