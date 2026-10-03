pub mod bars;
pub mod particles;

use crate::viz::particles::FieldKind;
use crate::viz::VizFrame;
use eframe::egui;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VisualizerMode {
    Bars,
    Ring,
    Particles,
    Constellation,
    Wave,
    /// Barras en una franja baja, sin tapar el contenido.
    Strip,
    Off,
}

impl VisualizerMode {
    pub fn next(self) -> Self {
        match self {
            VisualizerMode::Bars => VisualizerMode::Ring,
            VisualizerMode::Ring => VisualizerMode::Particles,
            VisualizerMode::Particles => VisualizerMode::Constellation,
            VisualizerMode::Constellation => VisualizerMode::Wave,
            VisualizerMode::Wave => VisualizerMode::Strip,
            VisualizerMode::Strip => VisualizerMode::Off,
            VisualizerMode::Off => VisualizerMode::Bars,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VisualizerMode::Bars => "Barras",
            VisualizerMode::Ring => "Anillo",
            VisualizerMode::Particles => "Partículas",
            VisualizerMode::Constellation => "Constelación",
            VisualizerMode::Wave => "Osciloscopio",
            VisualizerMode::Strip => "Franja",
            VisualizerMode::Off => "Apagado",
        }
    }

    /// Tipo de campo de partículas que usa el modo, si usa uno.
    pub fn field_kind(self) -> Option<FieldKind> {
        match self {
            VisualizerMode::Ring => Some(FieldKind::Ring),
            VisualizerMode::Particles | VisualizerMode::Constellation => Some(FieldKind::Free),
            VisualizerMode::Wave => Some(FieldKind::Wave),
            _ => None,
        }
    }

    /// Para `--viz`: ignora mayúsculas y acentos.
    pub fn from_name(name: &str) -> Option<Self> {
        let wanted = fold(name);
        let mut mode = VisualizerMode::Bars;
        for _ in 0..7 {
            if fold(mode.label()) == wanted {
                return Some(mode);
            }
            mode = mode.next();
        }
        None
    }
}

fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' => 'u',
            other => other,
        })
        .collect()
}

/// Barras y franja; los modos de partículas se dibujan con `particles::mesh_for`.
pub fn draw(ui: &egui::Ui, rect: egui::Rect, frame: &VizFrame, mode: VisualizerMode) {
    let painter = ui.painter_at(rect);
    if matches!(mode, VisualizerMode::Bars | VisualizerMode::Strip) {
        bars::draw(&painter, rect, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_ciclo_recorre_los_siete_modos_y_vuelve_al_inicio() {
        let mut mode = VisualizerMode::Bars;
        let mut labels = Vec::new();
        for _ in 0..7 {
            mode = mode.next();
            labels.push(mode.label());
        }
        assert_eq!(labels, ["Anillo", "Partículas", "Constelación", "Osciloscopio", "Franja", "Apagado", "Barras"]);
    }

    #[test]
    fn el_nombre_se_reconoce_sin_mayusculas_ni_acentos() {
        assert_eq!(VisualizerMode::from_name("CONSTELACION"), Some(VisualizerMode::Constellation));
        assert_eq!(VisualizerMode::from_name("particulas"), Some(VisualizerMode::Particles));
        assert_eq!(VisualizerMode::from_name("Osciloscopio"), Some(VisualizerMode::Wave));
        assert_eq!(VisualizerMode::from_name("anillo"), Some(VisualizerMode::Ring));
        assert_eq!(VisualizerMode::from_name("radial"), None);
        assert_eq!(VisualizerMode::from_name(""), None);
    }

    #[test]
    fn solo_los_modos_de_particulas_tienen_campo() {
        use crate::viz::particles::FieldKind;
        assert_eq!(VisualizerMode::Ring.field_kind(), Some(FieldKind::Ring));
        assert_eq!(VisualizerMode::Particles.field_kind(), Some(FieldKind::Free));
        assert_eq!(VisualizerMode::Constellation.field_kind(), Some(FieldKind::Free));
        assert_eq!(VisualizerMode::Wave.field_kind(), Some(FieldKind::Wave));
        assert_eq!(VisualizerMode::Bars.field_kind(), None);
        assert_eq!(VisualizerMode::Strip.field_kind(), None);
        assert_eq!(VisualizerMode::Off.field_kind(), None);
    }
}
