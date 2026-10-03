use crate::theme;
use crate::viz::VizFrame;
use eframe::egui;

/// Resplandor radial que pulsa con el volumen promedio — aproxima el
/// `radial-gradient` + `transform: scale`/`opacity` de `GlowPulse.tsx` con un
/// círculo semitransparente, ya que `egui::Painter` no tiene degradados
/// nativos en sus primitivas de relleno.
pub fn draw(painter: &egui::Painter, rect: egui::Rect, frame: &VizFrame) {
    let spectrum = &frame.bars;
    if spectrum.is_empty() {
        return;
    }
    let norm = (spectrum.iter().sum::<f32>() / spectrum.len() as f32 * 1.4).clamp(0.0, 1.0);

    let short_side = rect.width().min(rect.height());
    let radius = short_side * (0.28 + norm * 0.14);
    let alpha = (90.0 + norm * 120.0).round() as u8;

    let color = egui::Color32::from_rgba_unmultiplied(
        theme::ACCENT_PINK.r(),
        theme::ACCENT_PINK.g(),
        theme::ACCENT_PINK.b(),
        alpha,
    );
    painter.circle_filled(rect.center(), radius, color);
}
