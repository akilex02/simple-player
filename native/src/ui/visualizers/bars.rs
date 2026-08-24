use super::normalize_db;
use crate::theme;
use eframe::egui;

/// Barras verticales con gradiente rosa→lima — mismo mapeo que
/// `SpectrumBars.tsx`, solo que dibujado con `egui::Painter` en vez de canvas 2D.
pub fn draw(painter: &egui::Painter, rect: egui::Rect, spectrum: &[f32]) {
    if spectrum.is_empty() {
        return;
    }
    let gap = 4.0;
    let n = spectrum.len();
    let bar_width = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;

    for (i, db) in spectrum.iter().enumerate() {
        let norm = normalize_db(*db);
        let h = norm * rect.height();
        let x = rect.left() + i as f32 * (bar_width + gap);
        let bar = egui::Rect::from_min_size(egui::pos2(x, rect.bottom() - h), egui::vec2(bar_width.max(1.0), h));
        painter.rect_filled(bar, 2.0, lerp_color(theme::ACCENT_PINK, theme::ACCENT_LIME, norm));
    }
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}
