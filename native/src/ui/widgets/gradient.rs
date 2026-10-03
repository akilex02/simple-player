use eframe::egui::{self, Color32};

/// Degradado horizontal con varias paradas `(posición 0..1, color)` en un solo `Mesh`.
pub fn horizontal_gradient_mesh(rect: egui::Rect, stops: &[(f32, Color32)]) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    if stops.len() < 2 {
        return mesh;
    }
    let vertex = |x: f32, y: f32, color: Color32| egui::epaint::Vertex { pos: egui::pos2(x, y), uv: egui::epaint::WHITE_UV, color };
    for (frac, color) in stops {
        let x = rect.left() + frac.clamp(0.0, 1.0) * rect.width();
        mesh.vertices.push(vertex(x, rect.top(), *color));
        mesh.vertices.push(vertex(x, rect.bottom(), *color));
    }
    for i in 0..(stops.len() as u32 - 1) {
        let (t0, b0, t1, b1) = (2 * i, 2 * i + 1, 2 * i + 2, 2 * i + 3);
        mesh.indices.extend([t0, t1, b1, t0, b1, b0]);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(200.0, 8.0))
    }

    const STOPS: [(f32, Color32); 3] = [
        (0.0, Color32::from_rgb(255, 0, 0)),
        (0.5, Color32::from_rgb(0, 255, 0)),
        (1.0, Color32::from_rgb(0, 0, 255)),
    ];

    #[test]
    fn hay_dos_vertices_por_parada_y_un_quad_por_tramo() {
        let mesh = horizontal_gradient_mesh(rect(), &STOPS);
        assert_eq!(mesh.vertices.len(), 6);
        assert_eq!(mesh.indices.len(), 12);
        assert!(mesh.indices.iter().all(|i| (*i as usize) < mesh.vertices.len()));
    }

    #[test]
    fn las_paradas_quedan_en_su_fraccion_con_su_color() {
        let mesh = horizontal_gradient_mesh(rect(), &STOPS);
        let at = |x: f32| mesh.vertices.iter().find(|v| (v.pos.x - x).abs() < 1e-3).unwrap().color;
        assert_eq!(at(10.0), STOPS[0].1);
        assert_eq!(at(110.0), STOPS[1].1);
        assert_eq!(at(210.0), STOPS[2].1);
    }

    #[test]
    fn cubre_toda_la_altura_del_rect() {
        let mesh = horizontal_gradient_mesh(rect(), &STOPS);
        let ys: Vec<f32> = mesh.vertices.iter().map(|v| v.pos.y).collect();
        assert!(ys.iter().all(|y| (*y - 20.0).abs() < 1e-3 || (*y - 28.0).abs() < 1e-3));
    }

    #[test]
    fn menos_de_dos_paradas_no_dibuja_nada() {
        assert!(horizontal_gradient_mesh(rect(), &STOPS[..1]).vertices.is_empty());
    }
}
