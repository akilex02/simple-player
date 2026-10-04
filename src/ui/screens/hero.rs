use crate::theme::{self, icons, radius, space, text, with_alpha};
use crate::ui::textures::TextureCache;
use crate::ui::widgets::cover::paint_cover;
use crate::ui::widgets::glass::{paint_glass, GlassKind};
use crate::ui::widgets::gradient::radial_glow_mesh;
use crate::ui::widgets::pill_button::{PillButton, PillKind};
use eframe::egui::{self, RichText};

pub struct HeroInfo {
    pub eyebrow: String,
    pub title: String,
    pub subtitle: String,
    pub cover: Option<String>,
    /// Carátula circular (artistas) en vez de cuadrada.
    pub circle: bool,
}

#[derive(PartialEq)]
pub enum HeroAction {
    None,
    Play,
    Shuffle,
}

/// Por debajo de este alto disponible el encabezado se compacta.
const COMPACT_BELOW: f32 = 560.0;

/// Radio máximo de un resplandor centrado en `center` que no llega a la zona que las esquinas
/// redondeadas (`corner`) recortan de `rect`. El resplandor se recorta con un rectángulo recto, así que
/// si alcanzara esas esquinas pintaría color fuera de la tarjeta.
fn max_glow_radius(center: egui::Pos2, rect: egui::Rect, corner: f32) -> f32 {
    let mut nearest = f32::MAX;
    for corner_pos in [rect.left_top(), rect.right_top(), rect.left_bottom(), rect.right_bottom()] {
        let inward = egui::vec2(
            if corner_pos.x == rect.left() { 1.0 } else { -1.0 },
            if corner_pos.y == rect.top() { 1.0 } else { -1.0 },
        );
        let arc_center = corner_pos + inward * corner;
        // El arco va de un borde de la esquina al otro; se mide a lo largo de él (incluidos los extremos).
        for step in 0..=16 {
            let angle = std::f32::consts::FRAC_PI_2 * step as f32 / 16.0;
            let on_arc = arc_center - inward * egui::vec2(angle.sin(), angle.cos()) * corner;
            nearest = nearest.min(on_arc.distance(center));
        }
    }
    nearest
}

/// ¿Se usa el encabezado compacto con este alto disponible? (se decide fuera del scroll de la lista)
pub fn is_compact(available_height: f32) -> bool {
    available_height < COMPACT_BELOW
}

/// Alto del encabezado en cada modo.
pub fn height(compact: bool) -> f32 {
    if compact { 164.0 } else { 196.0 }
}

/// Encabezado de la pantalla: carátula, título, resumen y acciones.
pub fn show(ui: &mut egui::Ui, textures: &mut TextureCache, info: &HeroInfo, compact: bool) -> HeroAction {
    let accent = theme::accent(ui.ctx());
    let (height, cover_side) = (height(compact), if compact { 112.0 } else { 144.0 });
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
    let painter = ui.painter();
    paint_glass(painter, rect, GlassKind::Standard, radius::XL, 0.0);
    painter.rect_filled(rect, radius::XL, with_alpha(accent, 34));

    let cover_rect = egui::Rect::from_min_size(egui::pos2(rect.left() + 26.0, rect.center().y - cover_side / 2.0), egui::vec2(cover_side, cover_side));
    painter
        .with_clip_rect(rect)
                .add(radial_glow_mesh(cover_rect.center(), max_glow_radius(cover_rect.center(), rect, radius::XL).min(150.0), with_alpha(accent, 70), 40));
    let rounding = if info.circle { cover_side / 2.0 } else { radius::MD };
    paint_cover(ui, cover_rect, textures, &info.cover, rounding, icons::MUSIC_NOTES, true);

    let text_rect = egui::Rect::from_min_max(
        egui::pos2(cover_rect.right() + 28.0, rect.top() + 24.0),
        egui::pos2(rect.right() - 26.0, rect.bottom() - 24.0),
    );
    let mut action = HeroAction::None;
    let mut inner = ui.new_child(egui::UiBuilder::new().max_rect(text_rect).layout(egui::Layout::top_down(egui::Align::Min)));
    if !compact {
        inner.label(RichText::new(&info.eyebrow).font(theme::bold(text::XS)).color(accent));
    }
    let title_size = if compact { text::XXL } else { text::XXL + 8.0 };
    inner.add(egui::Label::new(RichText::new(&info.title).font(theme::deco(title_size)).color(theme::TEXT_MAIN)).truncate());
    inner.label(RichText::new(&info.subtitle).color(theme::TEXT_MUTED));
    inner.add_space(space::MD);
    inner.horizontal(|ui| {
        if PillButton::new("Reproducir", PillKind::Primary).icon(icons::PLAY).show(ui).clicked() {
            action = HeroAction::Play;
        }
        if PillButton::new("Aleatorio", PillKind::Secondary).icon(icons::SHUFFLE).show(ui).clicked() {
            action = HeroAction::Shuffle;
        }
    });
    action
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 196.0))
    }

    #[test]
    fn el_resplandor_no_llega_a_la_zona_recortada_por_las_esquinas_redondeadas() {
        let (center, corner) = (egui::pos2(98.0, 98.0), 20.0);
        let max = max_glow_radius(center, rect(), corner);
        // Todo punto del cuadrado de la esquina que queda fuera del redondeo debe estar a `max` o más.
        let r = rect();
        let mut checked = 0;
        for corner_pos in [r.left_top(), r.right_top(), r.left_bottom(), r.right_bottom()] {
            let inward = egui::vec2(if corner_pos.x == r.left() { 1.0 } else { -1.0 }, if corner_pos.y == r.top() { 1.0 } else { -1.0 });
            let arc_center = corner_pos + inward * corner;
            for i in 0..=20 {
                for j in 0..=20 {
                    let p = corner_pos + egui::vec2(inward.x * corner * i as f32 / 20.0, inward.y * corner * j as f32 / 20.0);
                    if p.distance(arc_center) > corner + 0.01 {
                        checked += 1;
                        assert!(p.distance(center) >= max - 0.5, "{p:?} queda dentro del resplandor ({max})");
                    }
                }
            }
        }
        assert!(checked > 100);
    }

    #[test]
    fn lejos_de_las_esquinas_el_resplandor_puede_ser_grande() {
        let big = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(2000.0, 2000.0));
        assert!(max_glow_radius(egui::pos2(1000.0, 1000.0), big, 20.0) > 500.0);
    }

    #[test]
    fn el_radio_aprovecha_el_espacio_sin_ser_cero() {
        // En el hero real el halo sigue siendo claramente más grande que la carátula (72 px de radio).
        assert!(max_glow_radius(egui::pos2(98.0, 98.0), rect(), 20.0) > 100.0);
    }
}
