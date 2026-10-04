use super::gradient::radial_glow_mesh;
use crate::theme::{self, with_alpha};
use eframe::egui::{self, Color32};

/// Radio y opacidad del resplandor del botón principal según el hover (0..1): discreto, para que no
/// se desborde sobre el texto cercano.
fn primary_glow(width: f32, hover: f32) -> (f32, u8) {
    let hover = hover.clamp(0.0, 1.0);
    (width * (0.2 + 0.22 * hover), (hover * 45.0) as u8)
}

#[derive(Clone, Copy, PartialEq)]
pub enum PillKind {
    /// Acción principal (Reproducir): relleno lima.
    Primary,
    /// Acción secundaria: vidrio con borde.
    Secondary,
    /// Sin relleno hasta el hover.
    Ghost,
}

pub struct PillButton<'a> {
    label: &'a str,
    icon: Option<&'a str>,
    kind: PillKind,
    height: f32,
}

impl<'a> PillButton<'a> {
    pub fn new(label: &'a str, kind: PillKind) -> Self {
        Self { label, icon: None, kind, height: 38.0 }
    }

    pub fn icon(mut self, icon: &'a str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let text_color = if self.kind == PillKind::Primary { theme::BG_DARK } else { theme::TEXT_MAIN };
        let painter = ui.painter().clone();
        let label = painter.layout_no_wrap(self.label.to_string(), theme::bold(theme::text::BASE), text_color);
        let icon = self
            .icon
            .map(|i| painter.layout_no_wrap(i.to_string(), egui::FontId::proportional(theme::text::MD + 2.0), text_color));

        let pad = 18.0;
        let gap = if icon.is_some() { 8.0 } else { 0.0 };
        let width = pad * 2.0 + label.size().x + icon.as_ref().map_or(0.0, |i| i.size().x) + gap;
        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, self.height), egui::Sense::click());
        let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let hover = ui.ctx().animate_bool_with_time(response.id, response.hovered(), theme::motion::HOVER);
        let pressed = response.is_pointer_button_down_on();
        let rect = if pressed { rect.shrink2(egui::vec2(1.5, 1.0)) } else { rect };
        let rounding = rect.height() / 2.0;

        match self.kind {
            PillKind::Primary => {
                if hover > 0.0 {
                    let (radius, alpha) = primary_glow(rect.width(), hover);
                    painter.add(radial_glow_mesh(rect.center(), radius, with_alpha(theme::ACCENT_LIME, alpha), 32));
                }
                painter.rect_filled(rect, rounding, theme::lerp_color(theme::ACCENT_LIME, Color32::WHITE, 0.35 * hover));
            }
            PillKind::Secondary => {
                painter.rect_filled(rect, rounding, Color32::from_white_alpha((18.0 + 20.0 * hover) as u8));
                painter.rect_stroke(rect, rounding, egui::Stroke::new(1.0_f32, theme::lerp_color(theme::GLASS_BORDER, theme::ACCENT_PINK, hover)));
                super::glass::paint_highlight(&painter, rect, rounding);
            }
            PillKind::Ghost => {
                painter.rect_filled(rect, rounding, Color32::from_white_alpha((28.0 * hover) as u8));
            }
        }

        let total = label.size().x + icon.as_ref().map_or(0.0, |i| i.size().x) + gap;
        let mut x = rect.center().x - total / 2.0;
        if let Some(icon) = icon {
            let y = rect.center().y - icon.size().y / 2.0;
            let w = icon.size().x;
            painter.galley(egui::pos2(x, y), icon, text_color);
            x += w + gap;
        }
        let y = rect.center().y - label.size().y / 2.0;
        painter.galley(egui::pos2(x, y), label, text_color);

        if response.has_focus() {
            painter.rect_stroke(rect.expand(3.0), rounding + 3.0, egui::Stroke::new(2.0_f32, theme::ACCENT_PINK));
        }
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_resplandor_es_discreto_y_crece_con_el_hover() {
        let (r0, a0) = primary_glow(160.0, 0.0);
        let (r1, a1) = primary_glow(160.0, 1.0);
        assert_eq!(a0, 0, "sin hover no hay brillo");
        assert!(r0 <= r1 && a0 <= a1);
        assert!(r1 <= 160.0 * 0.45, "el radio no pasa del 45 % del ancho: {r1}");
        assert!(a1 <= 50, "la intensidad máxima se mantiene baja: {a1}");
        assert!(a1 > 0 && r1 > 160.0 * 0.5 * 0.5, "pero se nota");
    }
}
