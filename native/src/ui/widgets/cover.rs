use super::gradient::horizontal_gradient_mesh;
use crate::theme;
use crate::ui::textures::{CoverSize, TextureCache};
use eframe::egui::{self, Color32};

/// Carátula cuadrada: imagen, esqueleto con brillo mientras carga, o un
/// placeholder con emoji si no hay portada. Solo pide la textura si el
/// cuadro está a la vista, y al tamaño que de verdad ocupa.
pub fn cover(
    ui: &mut egui::Ui,
    textures: &mut TextureCache,
    cover_path: &Option<String>,
    size: f32,
    rounding: f32,
    fallback_emoji: &str,
    with_shadow: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let painter = ui.painter();
    if with_shadow {
        painter.add(super::glass::shadow(0.4).as_shape(rect, rounding));
    }

    let cover_size = CoverSize::for_physical_px(size * ui.ctx().pixels_per_point());
    let loading = cover_path.is_some();
    let tex = cover_path.as_deref().and_then(|p| textures.get_or_load(p, cover_size));
    let failed = cover_path.as_deref().is_some_and(|p| textures.has_failed(p, cover_size));

    match tex {
        Some(tex) => {
            egui::Image::new((tex.id(), tex.size_vec2())).rounding(rounding).paint_at(ui, rect);
        }
        None if loading && !failed => {
            painter.rect_filled(rect, rounding, theme::BG_CARD_HOVER);
            paint_shimmer(ui, rect, rounding);
        }
        None => {
            painter.rect_filled(rect, rounding, theme::BG_CARD_HOVER);
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                fallback_emoji,
                egui::FontId::proportional(size * 0.5),
                theme::TEXT_MUTED,
            );
        }
    }
    response
}

/// Esqueleto de carga: fondo con una banda de luz que lo recorre.
pub fn skeleton(ui: &mut egui::Ui, size: f32, rounding: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, rounding, theme::BG_CARD_HOVER);
        paint_shimmer(ui, rect, rounding);
    }
    response
}

/// Banda de luz que cruza el cuadro mientras la carátula carga.
fn paint_shimmer(ui: &egui::Ui, rect: egui::Rect, rounding: f32) {
    let t = ui.input(|i| i.time) as f32;
    let phase = (t * 0.9).fract();
    let band = rect.width() * 0.6;
    let x = rect.left() - band + phase * (rect.width() + band);
    let strip = egui::Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(band, rect.height()));
    let stops = [
        (0.0, Color32::TRANSPARENT),
        (0.5, Color32::from_white_alpha(22)),
        (1.0, Color32::TRANSPARENT),
    ];
    let clipped = ui.painter().with_clip_rect(rect);
    clipped.add(horizontal_gradient_mesh(strip, &stops));
    let _ = rounding;
    ui.ctx().request_repaint();
}
