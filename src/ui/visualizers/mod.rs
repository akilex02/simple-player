pub mod bars;
pub mod radial;

use crate::viz::VizFrame;
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum VisualizerMode {
    Bars,
    Radial,
    /// Barras en una franja baja, sin tapar el contenido.
    Strip,
    Off,
}

impl VisualizerMode {
    pub fn next(self) -> Self {
        match self {
            VisualizerMode::Bars => VisualizerMode::Radial,
            VisualizerMode::Radial => VisualizerMode::Strip,
            VisualizerMode::Strip => VisualizerMode::Off,
            VisualizerMode::Off => VisualizerMode::Bars,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VisualizerMode::Bars => "Barras",
            VisualizerMode::Radial => "Radial",
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
        VisualizerMode::Strip => bars::draw(&painter, rect, frame),
        VisualizerMode::Off => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_ciclo_recorre_los_cuatro_modos_y_vuelve_al_inicio() {
        let mut mode = VisualizerMode::Bars;
        let mut labels = Vec::new();
        for _ in 0..4 {
            mode = mode.next();
            labels.push(mode.label());
        }
        assert_eq!(labels, ["Radial", "Franja", "Apagado", "Barras"]);
    }
}
