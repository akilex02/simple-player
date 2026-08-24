pub mod bars;
pub mod glow;
pub mod radial;

use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VisualizerMode {
    Bars,
    Radial,
    Glow,
}

impl VisualizerMode {
    pub fn next(self) -> Self {
        match self {
            VisualizerMode::Bars => VisualizerMode::Radial,
            VisualizerMode::Radial => VisualizerMode::Glow,
            VisualizerMode::Glow => VisualizerMode::Bars,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VisualizerMode::Bars => "Barras",
            VisualizerMode::Radial => "Radial",
            VisualizerMode::Glow => "Resplandor",
        }
    }
}

/// Rango de magnitudes del elemento `spectrum` de GStreamer (dB).
pub const MIN_DB: f32 = -60.0;
pub const MAX_DB: f32 = 0.0;

pub fn normalize_db(db: f32) -> f32 {
    ((db - MIN_DB) / (MAX_DB - MIN_DB)).clamp(0.0, 1.0)
}

pub fn draw(ui: &egui::Ui, rect: egui::Rect, spectrum: &[f32], mode: VisualizerMode) {
    let painter = ui.painter_at(rect);
    match mode {
        VisualizerMode::Bars => bars::draw(&painter, rect, spectrum),
        VisualizerMode::Radial => radial::draw(&painter, rect, spectrum),
        VisualizerMode::Glow => glow::draw(&painter, rect, spectrum),
    }
}
