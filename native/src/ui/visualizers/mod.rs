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

pub fn draw(ui: &egui::Ui, rect: egui::Rect, frame: &VizFrame, mode: VisualizerMode) {
    let painter = ui.painter_at(rect);
    match mode {
        VisualizerMode::Bars => bars::draw(&painter, rect, frame),
        VisualizerMode::Radial => radial::draw(&painter, rect, frame),
        VisualizerMode::Glow => glow::draw(&painter, rect, frame),
    }
}
