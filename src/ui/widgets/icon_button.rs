use super::gradient::radial_glow_mesh;
use crate::theme::{self, with_alpha};
use eframe::egui::{self, Color32};

/// Botón circular con ícono: fondo de vidrio que se ilumina al pasar el mouse,
/// se encoge al pulsarse y brilla con el color del acento.
pub struct IconButton<'a> {
    icon: &'a str,
    size: f32,
    active: bool,
    primary: bool,
    pulse: bool,
    accent: Option<Color32>,
    tooltip: Option<&'a str>,
}

impl<'a> IconButton<'a> {
    pub fn new(icon: &'a str, size: f32) -> Self {
        Self { icon, size, active: false, primary: false, pulse: false, accent: None, tooltip: None }
    }

    /// Encendido (shuffle activo, letras visibles…): ícono en color de acento.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Botón principal (reproducir): relleno de acento y texto oscuro.
    pub fn primary(mut self, primary: bool) -> Self {
        self.primary = primary;
        self
    }

    /// El halo late suavemente (reproduciendo).
    pub fn pulse(mut self, pulse: bool) -> Self {
        self.pulse = pulse;
        self
    }

    pub fn accent(mut self, accent: Color32) -> Self {
        self.accent = Some(accent);
        self
    }

    pub fn tooltip(mut self, tooltip: &'a str) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let accent = self.accent.unwrap_or_else(|| theme::accent(ui.ctx()));
        let (rect, mut response) = ui.allocate_exact_size(egui::vec2(self.size, self.size), egui::Sense::click());
        response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if let Some(text) = self.tooltip {
            response = response.on_hover_text(text);
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), theme::motion::HOVER);
        let pressed = response.is_pointer_button_down_on();
        let scale = if pressed { 0.93 } else { 1.0 };
        let radius = self.size / 2.0 * scale;
        let painter = ui.painter();

        if self.primary || hover > 0.0 {
            let beat = if self.pulse { theme::motion::pulse(ui.input(|i| i.time) as f32, 2.4) } else { 0.0 };
            let (glow, reach) = if self.primary {
                (0.36 + 0.15 * hover + 0.14 * beat, 1.38 + 0.1 * hover + 0.1 * beat)
            } else {
                (0.35 * hover, 1.5)
            };
            painter.add(radial_glow_mesh(rect.center(), radius * reach, with_alpha(accent, (glow * 140.0) as u8), 28));
        }

        let (fill, icon_color) = if self.primary {
            let fill = theme::lerp_color(accent, Color32::WHITE, 0.25 * hover);
            (fill, theme::BG_DARK)
        } else {
            let fill = if self.active {
                with_alpha(accent, (46.0 + 30.0 * hover) as u8)
            } else {
                Color32::from_white_alpha((14.0 + 22.0 * hover) as u8)
            };
            (fill, if self.active { accent } else { theme::lerp_color(theme::TEXT_MUTED, theme::TEXT_MAIN, hover) })
        };
        painter.circle_filled(rect.center(), radius, fill);
        if !self.primary {
            painter.circle_stroke(rect.center(), radius, egui::Stroke::new(1.0_f32, theme::lerp_color(theme::GLASS_BORDER, accent, if self.active { 0.8 } else { hover * 0.6 })));
        }
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            self.icon,
            egui::FontId::proportional(self.size * 0.48 * scale),
            icon_color,
        );

        if response.has_focus() {
            painter.circle_stroke(rect.center(), radius + 3.0, egui::Stroke::new(2.0_f32, accent));
        }
        response
    }
}
