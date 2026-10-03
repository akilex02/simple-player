#![allow(dead_code)] // se quita al conectar (Tarea 7)
//! Dibujo de los modos de partículas: un solo `Mesh` por frame (una draw call).
use super::VisualizerMode;
use crate::viz::particles::{links, ParticleField, V2};
use eframe::egui;

pub const COUNT_PARTICLES: usize = 1500;
pub const COUNT_CONSTELLATION: usize = 600;
pub const COUNT_RING: usize = 360;
pub const COUNT_WAVE: usize = 400;
pub const MAX_LINKS: usize = 2500;
/// Tope de vértices por frame: un modo que lo rebase no pasa las pruebas.
pub const MAX_MESH_VERTICES: usize = 60_000;
const LINK_BASE_DIST: f32 = 45.0;
const LINK_BASS_DIST: f32 = 6.0;
const LINK_WIDTH: f32 = 1.0;
const TRACE_WIDTH: f32 = 2.5;
const DOT_ALPHA: u8 = 204;
const TRACE_ALPHA: u8 = 140;
/// Hexágono unitario: se ve redondo a tamaños de 1.5–3 px con pocos vértices.
const HEX: [(f32, f32); 6] = [(1.0, 0.0), (0.5, 0.866), (-0.5, 0.866), (-1.0, 0.0), (-0.5, -0.866), (0.5, -0.866)];

pub fn count_for(mode: VisualizerMode) -> usize {
    match mode {
        VisualizerMode::Particles => COUNT_PARTICLES,
        VisualizerMode::Constellation => COUNT_CONSTELLATION,
        VisualizerMode::Ring => COUNT_RING,
        VisualizerMode::Wave => COUNT_WAVE,
        VisualizerMode::Bars | VisualizerMode::Strip | VisualizerMode::Off => 0,
    }
}

fn vertex(x: f32, y: f32, color: egui::Color32) -> egui::epaint::Vertex {
    egui::epaint::Vertex { pos: egui::pos2(x, y), uv: egui::epaint::WHITE_UV, color }
}

/// El acento, más claro según `tint` (hasta 35 % hacia blanco), con la opacidad dada.
fn tinted(accent: egui::Color32, tint: f32, alpha: u8) -> egui::Color32 {
    let mix = |c: u8| (c as f32 + (255.0 - c as f32) * tint.clamp(0.0, 1.0) * 0.35).round() as u8;
    egui::Color32::from_rgba_unmultiplied(mix(accent.r()), mix(accent.g()), mix(accent.b()), alpha)
}

/// Un cuadrilátero delgado entre `a` y `b`.
fn push_segment(mesh: &mut egui::Mesh, a: V2, b: V2, width: f32, color: egui::Color32) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = dx.hypot(dy);
    if len < 0.01 {
        return;
    }
    let (nx, ny) = (-dy / len * width / 2.0, dx / len * width / 2.0);
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend([
        vertex(a.x + nx, a.y + ny, color),
        vertex(b.x + nx, b.y + ny, color),
        vertex(b.x - nx, b.y - ny, color),
        vertex(a.x - nx, a.y - ny, color),
    ]);
    mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

pub fn dots_mesh(field: &ParticleField, accent: egui::Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    mesh.reserve_vertices(field.particles().len() * 6);
    mesh.reserve_triangles(field.particles().len() * 4);
    for p in field.particles() {
        let color = tinted(accent, p.tint, DOT_ALPHA);
        let base = mesh.vertices.len() as u32;
        for (cx, cy) in HEX {
            mesh.vertices.push(vertex(p.pos.x + cx * p.size, p.pos.y + cy * p.size, color));
        }
        for i in 1..5u32 {
            mesh.indices.extend([base, base + i, base + i + 1]);
        }
    }
    mesh
}

pub fn links_mesh(field: &ParticleField, accent: egui::Color32, max_dist: f32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let ps = field.particles();
    for link in links(ps, max_dist, MAX_LINKS) {
        let alpha = (link.alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        let color = tinted(accent, 0.0, alpha);
        push_segment(&mut mesh, ps[link.a as usize].pos, ps[link.b as usize].pos, LINK_WIDTH, color);
    }
    mesh
}

/// Une cada partícula con la siguiente; `closed` cierra el aro con la última.
pub fn trace_mesh(field: &ParticleField, closed: bool, accent: egui::Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let ps = field.particles();
    let n = ps.len();
    if n < 2 {
        return mesh;
    }
    let segments = if closed { n } else { n - 1 };
    for i in 0..segments {
        let (a, b) = (&ps[i], &ps[(i + 1) % n]);
        push_segment(&mut mesh, a.pos, b.pos, TRACE_WIDTH, tinted(accent, a.tint, TRACE_ALPHA));
    }
    mesh
}

/// Todo lo que dibuja un modo de partículas, en un solo mesh.
pub fn mesh_for(mode: VisualizerMode, field: &ParticleField, accent: egui::Color32, bass: f32) -> egui::Mesh {
    let mut mesh = match mode {
        VisualizerMode::Constellation => links_mesh(field, accent, LINK_BASE_DIST + bass * LINK_BASS_DIST),
        VisualizerMode::Ring => trace_mesh(field, true, accent),
        VisualizerMode::Wave => trace_mesh(field, false, accent),
        _ => egui::Mesh::default(),
    };
    mesh.append(dots_mesh(field, accent));
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::viz::particles::{FieldKind, ParticleField, Rect2, V2};

    fn accent() -> egui::Color32 {
        egui::Color32::from_rgb(0xff, 0x9e, 0xbd)
    }

    fn field_at(kind: FieldKind, positions: &[V2]) -> ParticleField {
        let mut field = ParticleField::new(kind, positions.len(), Rect2::new(0.0, 0.0, 1000.0, 600.0), 1);
        field.set_positions_for_tests(positions);
        field
    }

    fn valid_indices(mesh: &egui::Mesh) -> bool {
        mesh.indices.iter().all(|&i| (i as usize) < mesh.vertices.len())
    }

    #[test]
    fn cada_punto_aporta_un_hexagono() {
        let field = field_at(FieldKind::Free, &[V2::new(10.0, 10.0), V2::new(50.0, 50.0)]);
        let mesh = dots_mesh(&field, accent());
        assert_eq!(mesh.vertices.len(), 12);
        assert_eq!(mesh.indices.len(), 24);
        assert!(valid_indices(&mesh));
    }

    #[test]
    fn un_enlace_cercano_aporta_un_quad_y_uno_lejano_nada() {
        let near = field_at(FieldKind::Free, &[V2::new(0.0, 0.0), V2::new(10.0, 0.0)]);
        let far = field_at(FieldKind::Free, &[V2::new(0.0, 0.0), V2::new(500.0, 0.0)]);
        assert_eq!(links_mesh(&near, accent(), 45.0).vertices.len(), 4);
        assert_eq!(links_mesh(&far, accent(), 45.0).vertices.len(), 0);
    }

    #[test]
    fn el_trazo_cerrado_une_todas_y_el_abierto_deja_un_hueco() {
        let ps: Vec<V2> = (0..5).map(|i| V2::new(i as f32 * 20.0, 10.0)).collect();
        let ring = field_at(FieldKind::Ring, &ps);
        assert_eq!(trace_mesh(&ring, true, accent()).vertices.len(), 5 * 4);
        assert_eq!(trace_mesh(&ring, false, accent()).vertices.len(), 4 * 4);
    }

    #[test]
    fn los_modos_sin_campo_no_tienen_particulas() {
        assert_eq!(count_for(VisualizerMode::Bars), 0);
        assert_eq!(count_for(VisualizerMode::Strip), 0);
        assert_eq!(count_for(VisualizerMode::Off), 0);
        assert_eq!(count_for(VisualizerMode::Particles), COUNT_PARTICLES);
        assert_eq!(count_for(VisualizerMode::Constellation), COUNT_CONSTELLATION);
    }

    #[test]
    fn ningun_modo_supera_el_tope_de_vertices_ni_en_el_peor_caso() {
        // Peor caso: todas las partículas apiladas en 100 × 100, así que se llega al tope de enlaces.
        for mode in [VisualizerMode::Ring, VisualizerMode::Particles, VisualizerMode::Constellation, VisualizerMode::Wave] {
            let kind = mode.field_kind().unwrap();
            let field = ParticleField::new(kind, count_for(mode), Rect2::new(0.0, 0.0, 100.0, 100.0), 1);
            let mesh = mesh_for(mode, &field, accent(), 1.0);
            assert!(valid_indices(&mesh), "{mode:?}: índices inválidos");
            assert!(
                mesh.vertices.len() <= MAX_MESH_VERTICES,
                "{mode:?}: {} vértices > {MAX_MESH_VERTICES}",
                mesh.vertices.len()
            );
        }
    }

    #[test]
    fn la_constelacion_no_pasa_del_tope_de_enlaces() {
        let field = ParticleField::new(FieldKind::Free, COUNT_CONSTELLATION, Rect2::new(0.0, 0.0, 100.0, 100.0), 1);
        let mesh = links_mesh(&field, accent(), 45.0 + 6.0);
        assert!(mesh.vertices.len() <= MAX_LINKS * 4);
    }
}
