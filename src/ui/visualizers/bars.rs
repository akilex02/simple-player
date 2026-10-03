use crate::theme;
use crate::viz::VizFrame;
use eframe::egui;

/// Barras con degradado vertical (rosa abajo → lima arriba según la altura),
/// todas en un solo `Mesh`: una draw call sin importar cuántas barras haya.
pub fn bars_mesh(rect: egui::Rect, bars: &[f32]) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let slot = rect.width() / bars.len().max(1) as f32;
    let bar_width = slot * (1.0 - GAP_RATIO);
    for (i, value) in bars.iter().enumerate() {
        let norm = value.clamp(0.0, 1.0);
        let height = norm * rect.height();
        if height < 0.5 {
            continue;
        }
        let x0 = rect.left() + i as f32 * slot + (slot - bar_width) / 2.0;
        let x1 = x0 + bar_width;
        let (top, bottom) = (rect.bottom() - height, rect.bottom());
        let top_color = lerp_color(theme::ACCENT_PINK, theme::ACCENT_LIME, norm);
        let base = mesh.vertices.len() as u32;
        let vertex = |x, y, color| egui::epaint::Vertex { pos: egui::pos2(x, y), uv: egui::epaint::WHITE_UV, color };
        mesh.vertices.extend([
            vertex(x0, top, top_color),
            vertex(x1, top, top_color),
            vertex(x1, bottom, theme::ACCENT_PINK),
            vertex(x0, bottom, theme::ACCENT_PINK),
        ]);
        mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh
}

fn lerp_color(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let lerp = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)).round() as u8;
    egui::Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

pub fn draw(painter: &egui::Painter, rect: egui::Rect, frame: &VizFrame) {
    if frame.bars.is_empty() {
        return;
    }
    painter.add(egui::Shape::mesh(bars_mesh(rect, &frame.bars)));

    let n = frame.bars.len() as f32;
    let slot = rect.width() / n;
    let bar_width = slot * (1.0 - GAP_RATIO);
    for (i, peak) in frame.peaks.iter().enumerate() {
        if *peak < 0.02 {
            continue;
        }
        let x = rect.left() + i as f32 * slot + (slot - bar_width) / 2.0;
        let y = rect.bottom() - peak * rect.height();
        let cap = egui::Rect::from_min_size(egui::pos2(x, y - 2.0), egui::vec2(bar_width, 2.0));
        painter.rect_filled(cap, 0.0, theme::ACCENT_LIME);
    }
}

const GAP_RATIO: f32 = 0.25;

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 100.0))
    }

    #[test]
    fn cada_barra_visible_aporta_un_quad() {
        let mesh = bars_mesh(rect(), &[0.0, 0.5, 1.0]);
        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.indices.len(), 12);
    }

    #[test]
    fn la_altura_de_la_barra_es_proporcional_al_valor() {
        let mesh = bars_mesh(rect(), &[0.5]);
        let top = mesh.vertices.iter().map(|v| v.pos.y).fold(f32::MAX, f32::min);
        let bottom = mesh.vertices.iter().map(|v| v.pos.y).fold(f32::MIN, f32::max);
        assert!((top - 70.0).abs() < 1e-3, "top {top}");
        assert!((bottom - 120.0).abs() < 1e-3, "bottom {bottom}");
    }

    #[test]
    fn el_degradado_va_del_rosa_abajo_a_un_color_mas_claro_arriba() {
        let mesh = bars_mesh(rect(), &[1.0]);
        let at = |y: f32| mesh.vertices.iter().find(|v| (v.pos.y - y).abs() < 1e-3).unwrap().color;
        assert_eq!(at(120.0), theme::ACCENT_PINK);
        assert_eq!(at(20.0), theme::ACCENT_LIME);
    }

    #[test]
    fn todos_los_indices_apuntan_a_vertices_existentes() {
        let mesh = bars_mesh(rect(), &[0.3, 0.9, 0.6]);
        assert!(mesh.indices.iter().all(|i| (*i as usize) < mesh.vertices.len()));
    }
}
