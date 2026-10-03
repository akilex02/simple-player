pub mod bars;
pub mod glow;
pub mod radial;

use crate::viz::VizFrame;
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VisualizerMode {
    Bars,
    Radial,
    Glow,
    /// Barras en una franja baja, sin tapar el contenido.
    Strip,
    Off,
}

impl VisualizerMode {
    pub fn next(self) -> Self {
        match self {
            VisualizerMode::Bars => VisualizerMode::Radial,
            VisualizerMode::Radial => VisualizerMode::Glow,
            VisualizerMode::Glow => VisualizerMode::Strip,
            VisualizerMode::Strip => VisualizerMode::Off,
            VisualizerMode::Off => VisualizerMode::Bars,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VisualizerMode::Bars => "Barras",
            VisualizerMode::Radial => "Radial",
            VisualizerMode::Glow => "Resplandor",
            VisualizerMode::Strip => "Franja",
            VisualizerMode::Off => "Apagado",
        }
    }
}

/// `cover` es el rectángulo de la portada; el modo radial se dibuja a su alrededor.
pub fn draw(ui: &egui::Ui, rect: egui::Rect, frame: &VizFrame, mode: VisualizerMode, cover: egui::Rect) {
    let painter = ui.painter_at(rect);
    match mode {
        VisualizerMode::Bars => bars::draw(&painter, rect, frame),
        VisualizerMode::Radial => radial::draw(&painter, frame, cover),
        VisualizerMode::Glow => glow::draw(&painter, rect, frame),
        VisualizerMode::Strip => bars::draw(&painter, rect, frame),
        VisualizerMode::Off => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_ciclo_recorre_los_cinco_modos_y_vuelve_al_inicio() {
        let mut mode = VisualizerMode::Bars;
        let mut labels = Vec::new();
        for _ in 0..5 {
            mode = mode.next();
            labels.push(mode.label());
        }
        assert_eq!(labels, ["Radial", "Resplandor", "Franja", "Apagado", "Barras"]);
    }
}
