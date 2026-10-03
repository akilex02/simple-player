use crate::theme;
use crate::viz::VizFrame;
use eframe::egui;

/// Barras distribuidas en círculo, centradas en la pantalla — mismo mapeo
/// que `RadialSpectrum.tsx`.
pub fn draw(painter: &egui::Painter, rect: egui::Rect, frame: &VizFrame) {
    let spectrum = &frame.bars;
    if spectrum.is_empty() {
        return;
    }
    let n = spectrum.len();
    let short_side = rect.width().min(rect.height());
    let inner_radius = short_side * 0.16;
    let max_bar_len = short_side * 0.22;
    let line_width = ((std::f32::consts::TAU * inner_radius) / n as f32 * 0.6).max(2.0);

    for (i, norm) in spectrum.iter().enumerate() {
        let bar_len = norm * max_bar_len;
        let angle = (i as f32 / n as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let (sin, cos) = angle.sin_cos();

        let p1 = egui::pos2(rect.center().x + cos * inner_radius, rect.center().y + sin * inner_radius);
        let p2 = egui::pos2(
            rect.center().x + cos * (inner_radius + bar_len),
            rect.center().y + sin * (inner_radius + bar_len),
        );
        painter.line_segment([p1, p2], egui::Stroke::new(line_width, theme::ACCENT_PINK));
    }
}
