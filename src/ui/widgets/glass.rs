use crate::theme::{self, GLASS_BORDER, GLASS_FILL, GLASS_FILL_STRONG, GLASS_HIGHLIGHT};
use eframe::egui::{self, Color32, Shadow};

#[derive(Clone, Copy, PartialEq)]
pub enum GlassKind {
    /// Tarjetas y paneles flotantes.
    Standard,
    /// Sidebar y barra inferior: más opaco para que el texto se lea.
    Strong,
}

impl GlassKind {
    pub fn fill(self) -> Color32 {
        match self {
            GlassKind::Standard => GLASS_FILL,
            GlassKind::Strong => GLASS_FILL_STRONG,
        }
    }
}

pub fn shadow(strength: f32) -> Shadow {
    Shadow {
        offset: egui::vec2(0.0, 8.0 + 4.0 * strength),
        blur: 24.0 + 12.0 * strength,
        spread: 0.0,
        color: Color32::from_black_alpha((90.0 + 40.0 * strength) as u8),
    }
}

/// Pinta sombra, relleno, borde y reflejo superior de un panel de vidrio.
/// `lift` (0..1) hace la sombra más grande y el borde más claro (hover).
pub fn paint_glass(painter: &egui::Painter, rect: egui::Rect, kind: GlassKind, rounding: f32, lift: f32) {
    painter.add(shadow(lift).as_shape(rect, rounding));
    painter.rect_filled(rect, rounding, kind.fill());
    let border = theme::lerp_color(GLASS_BORDER, Color32::from_white_alpha(70), lift);
    painter.rect_stroke(rect, rounding, egui::Stroke::new(1.0_f32, border));
    paint_highlight(painter, rect, rounding);
}

/// Línea clara en el borde superior que simula el reflejo del cristal.
pub fn paint_highlight(painter: &egui::Painter, rect: egui::Rect, rounding: f32) {
    let inset = rounding.min(rect.height() / 2.0).min(rect.width() / 2.0);
    let y = rect.top() + 0.5;
    painter.line_segment(
        [egui::pos2(rect.left() + inset, y), egui::pos2(rect.right() - inset, y)],
        egui::Stroke::new(1.0_f32, GLASS_HIGHLIGHT),
    );
}

pub fn glass_panel<R>(
    ui: &mut egui::Ui,
    kind: GlassKind,
    rounding: f32,
    margin: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let frame = egui::Frame::none()
        .fill(kind.fill())
        .stroke(egui::Stroke::new(1.0_f32, GLASS_BORDER))
        .rounding(rounding)
        .inner_margin(margin)
        .shadow(shadow(0.0));
    let out = frame.show(ui, add_contents);
    paint_highlight(ui.painter(), out.response.rect, rounding);
    out
}
