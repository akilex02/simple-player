use super::crossfade::Crossfade;
use super::textures::TextureCache;
use super::widgets::gradient::radial_glow_mesh;
use crate::theme::{self, color, with_alpha};
use eframe::egui::{self, Color32};

/// Zona de la textura (en UV) que cubre un rect sin deformar la imagen
/// (equivale a `object-fit: cover`).
pub fn cover_uv(tex_size: egui::Vec2, target: egui::Vec2) -> egui::Rect {
    let full = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
    if tex_size.min_elem() <= 0.0 || target.min_elem() <= 0.0 {
        return full;
    }
    let (image, rect) = (tex_size.x / tex_size.y, target.x / target.y);
    if image > rect {
        let margin = (1.0 - rect / image) / 2.0;
        egui::Rect::from_min_max(egui::pos2(margin, 0.0), egui::pos2(1.0 - margin, 1.0))
    } else if image < rect {
        let margin = (1.0 - image / rect) / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.0, margin), egui::pos2(1.0, 1.0 - margin))
    } else {
        full
    }
}

/// Fondo vivo: la carátula actual desenfocada a pantalla completa con
/// crossfade, o un degradado de atardecer si no hay carátula. También lleva
/// el color de acento dinámico que sale de la carátula.
pub struct Backdrop {
    fade: Crossfade<String>,
    accent: Color32,
}

impl Backdrop {
    pub fn new() -> Self {
        Self { fade: Crossfade::new(theme::motion::BACKDROP), accent: theme::ACCENT_PINK }
    }

    /// Acento actual: rosa mezclado con el color dominante de la carátula.
    pub fn accent(&self) -> Color32 {
        self.accent
    }

    pub fn is_animating(&self) -> bool {
        self.fade.is_animating()
    }

    pub fn show(&mut self, ctx: &egui::Context, textures: &mut TextureCache, cover: Option<&str>, rect: egui::Rect) {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        self.fade.set_target(cover.map(String::from));
        self.fade.tick(dt);

        let painter = ctx.layer_painter(egui::LayerId::background());
        paint_sunset(&painter, rect, ctx.input(|i| i.time) as f32);

        let layers = [
            (self.fade.previous().cloned(), self.fade.previous_alpha()),
            (self.fade.current().cloned(), self.fade.current_alpha()),
        ];
        let mut target_accent = theme::ACCENT_PINK;
        for (index, (path, alpha)) in layers.into_iter().enumerate() {
            let Some(path) = path else { continue };
            let Some((tex, dominant)) = textures.get_backdrop(&path) else { continue };
            let uv = cover_uv(tex.size_vec2(), rect.size());
            painter.image(tex.id(), rect, uv, Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0) as u8));
            if index == 1 {
                if let Some(d) = dominant {
                    target_accent = theme::lerp_color(theme::ACCENT_PINK, color::ensure_vibrant(d), 0.65);
                }
            }
        }
        painter.rect_filled(rect, 0.0, with_alpha(theme::BG_BASE, 150));

        self.accent = color::approach_color(self.accent, target_accent, dt, 0.35);
        if self.fade.is_animating() || self.accent != target_accent {
            ctx.request_repaint();
        }
    }
}

/// Fondo sin carátula: base índigo con manchas de luz de atardecer.
pub fn paint_sunset(painter: &egui::Painter, rect: egui::Rect, time: f32) {
    painter.rect_filled(rect, 0.0, theme::BG_BASE);
    let drift = |speed: f32, phase: f32| (time * speed + phase).sin() * 0.04;
    let blobs = [
        (0.18 + drift(0.07, 0.0), 0.12, 0.75, theme::ACCENT_PINK, 80),
        (0.85 + drift(0.05, 2.0), 0.30 + drift(0.06, 1.0), 0.60, theme::ACCENT_PURPLE, 70),
        (0.50 + drift(0.04, 4.0), 1.05, 0.70, theme::ACCENT_SUNSET, 60),
    ];
    let clipped = painter.with_clip_rect(rect);
    for (fx, fy, size, color, alpha) in blobs {
        let center = egui::pos2(rect.left() + fx * rect.width(), rect.top() + fy * rect.height());
        clipped.add(radial_glow_mesh(center, size * rect.width().max(rect.height()) * 0.6, with_alpha(color, alpha), 40));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_imagen_cuadrada_en_un_rect_ancho_recorta_arriba_y_abajo() {
        let uv = cover_uv(egui::vec2(100.0, 100.0), egui::vec2(200.0, 100.0));
        assert_eq!((uv.min.x, uv.max.x), (0.0, 1.0));
        assert!((uv.min.y - 0.25).abs() < 1e-5 && (uv.max.y - 0.75).abs() < 1e-5, "{uv:?}");
    }

    #[test]
    fn una_imagen_ancha_en_un_rect_cuadrado_recorta_los_lados() {
        let uv = cover_uv(egui::vec2(200.0, 100.0), egui::vec2(100.0, 100.0));
        assert_eq!((uv.min.y, uv.max.y), (0.0, 1.0));
        assert!((uv.min.x - 0.25).abs() < 1e-5 && (uv.max.x - 0.75).abs() < 1e-5, "{uv:?}");
    }

    #[test]
    fn con_la_misma_proporcion_se_usa_toda_la_imagen() {
        let uv = cover_uv(egui::vec2(300.0, 200.0), egui::vec2(150.0, 100.0));
        assert_eq!(uv, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)));
    }

    #[test]
    fn dimensiones_invalidas_no_producen_nan() {
        let uv = cover_uv(egui::vec2(0.0, 0.0), egui::vec2(100.0, 100.0));
        assert!(uv.min.x.is_finite() && uv.max.y.is_finite());
    }
}
