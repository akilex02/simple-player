use crate::stats::format::format_duration;
use crate::stats::model::TimelineBucket;
use crate::theme;
use eframe::egui;

/// Una barra mínima visible para valores pequeños pero mayores que cero.
const MIN_BAR_HEIGHT: f32 = 3.0;

/// Rectángulos de las barras dentro de `rect`; la barra más alta ocupa toda la altura.
pub fn bar_rects(rect: egui::Rect, values: &[u64], gap: f32) -> Vec<egui::Rect> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }
    let max = values.iter().copied().max().unwrap_or(0);
    let slot = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let mut height = if max == 0 { 0.0 } else { *v as f32 / max as f32 * rect.height() };
            if *v > 0 && height < MIN_BAR_HEIGHT {
                height = MIN_BAR_HEIGHT;
            }
            let x = rect.left() + i as f32 * (slot + gap);
            egui::Rect::from_min_size(egui::pos2(x, rect.bottom() - height), egui::vec2(slot, height))
        })
        .collect()
}

/// Barra bajo el puntero (por su columna, sin importar la altura), incluyendo medio hueco a cada lado.
pub fn bucket_at(rects: &[egui::Rect], x: f32) -> Option<usize> {
    let first = rects.first()?;
    let last = rects.last()?;
    let half_gap = if rects.len() > 1 { (rects[1].left() - rects[0].right()) / 2.0 } else { 0.0 };
    if x < first.left() - half_gap || x > last.right() + half_gap {
        return None;
    }
    rects
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (a.center().x - x).abs().total_cmp(&(b.center().x - x).abs()))
        .map(|(i, _)| i)
}

/// Cada cuántas barras se dibuja una etiqueta para que no se encimen.
pub fn label_step(count: usize) -> usize {
    match count {
        0..=12 => 1,
        13..=24 => 3,
        _ => 5,
    }
}

pub fn bar_chart(ui: &mut egui::Ui, buckets: &[TimelineBucket], peak: Option<usize>, height: f32) {
    const LABEL_H: f32 = 22.0;
    let accent = theme::accent(ui.ctx());
    let (outer, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height + LABEL_H), egui::Sense::hover());
    if !ui.is_rect_visible(outer) || buckets.is_empty() {
        return;
    }
    let plot = egui::Rect::from_min_size(outer.min, egui::vec2(outer.width(), height));
    let values: Vec<u64> = buckets.iter().map(|b| b.listened_ms).collect();
    let gap = if buckets.len() > 24 { 3.0 } else { 6.0 };
    let rects = bar_rects(plot, &values, gap);
    let hovered = response.hover_pos().and_then(|p| bucket_at(&rects, p.x));
    let painter = ui.painter();

    painter.hline(plot.x_range(), plot.bottom(), egui::Stroke::new(1.0_f32, theme::GLASS_BORDER));
    let step = label_step(buckets.len());
    for (i, (rect, bucket)) in rects.iter().zip(buckets).enumerate() {
        let is_peak = peak == Some(i);
        let lit = is_peak || hovered == Some(i);
        let bottom_color = if lit { theme::with_alpha(accent, 230) } else { theme::with_alpha(accent, 120) };
        let top_color = if lit { theme::lerp_color(accent, egui::Color32::WHITE, 0.25) } else { theme::with_alpha(accent, 170) };
        if rect.height() > 0.0 {
            let mut mesh = egui::Mesh::default();
            let v = |pos: egui::Pos2, color| egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color };
            mesh.vertices.extend([
                v(rect.left_top(), top_color),
                v(rect.right_top(), top_color),
                v(rect.right_bottom(), bottom_color),
                v(rect.left_bottom(), bottom_color),
            ]);
            mesh.indices.extend([0, 1, 2, 0, 2, 3]);
            painter.add(egui::Shape::mesh(mesh));
        }
        if i % step == 0 {
            let color = if lit { theme::TEXT_MAIN } else { theme::TEXT_MUTED };
            painter.text(
                egui::pos2(rect.center().x, plot.bottom() + LABEL_H / 2.0 + 2.0),
                egui::Align2::CENTER_CENTER,
                &bucket.label,
                egui::FontId::proportional(theme::text::XS),
                color,
            );
        }
    }

    if let Some(i) = hovered {
        let text = format!("{} · {}", buckets[i].label, format_duration(buckets[i].listened_ms));
        response.on_hover_text_at_pointer(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 100.0))
    }

    #[test]
    fn la_barra_mas_alta_ocupa_toda_la_altura_y_las_demas_son_proporcionales() {
        let rects = bar_rects(area(), &[50, 100, 25], 4.0);
        assert_eq!(rects.len(), 3);
        assert!((rects[1].height() - 100.0).abs() < 1e-3);
        assert!((rects[0].height() - 50.0).abs() < 1e-3);
        assert!((rects[2].height() - 25.0).abs() < 1e-3);
        assert!(rects.iter().all(|r| (r.bottom() - 120.0).abs() < 1e-3), "todas apoyadas en la base");
    }

    #[test]
    fn un_valor_cero_no_tiene_altura_y_uno_diminuto_se_ve() {
        let rects = bar_rects(area(), &[0, 1, 1_000_000], 4.0);
        assert_eq!(rects[0].height(), 0.0);
        assert!(rects[1].height() >= 3.0, "{}", rects[1].height());
    }

    #[test]
    fn todo_en_cero_no_produce_nan_ni_alturas() {
        let rects = bar_rects(area(), &[0, 0, 0], 4.0);
        assert!(rects.iter().all(|r| r.height() == 0.0 && r.width().is_finite()));
    }

    #[test]
    fn las_barras_con_sus_huecos_caben_exactamente_en_el_ancho() {
        let rects = bar_rects(area(), &[1, 2, 3, 4], 6.0);
        assert!((rects.first().unwrap().left() - 10.0).abs() < 1e-3);
        assert!((rects.last().unwrap().right() - 310.0).abs() < 1e-3);
        assert!(rects.windows(2).all(|w| (w[1].left() - w[0].right() - 6.0).abs() < 1e-3));
    }

    #[test]
    fn sin_valores_no_hay_barras() {
        assert!(bar_rects(area(), &[], 4.0).is_empty());
    }

    #[test]
    fn el_puntero_encuentra_la_barra_de_su_columna() {
        let rects = bar_rects(area(), &[1, 2, 3], 6.0);
        assert_eq!(bucket_at(&rects, rects[0].center().x), Some(0));
        assert_eq!(bucket_at(&rects, rects[2].center().x), Some(2));
        // en el hueco entre dos barras se elige la más cercana
        let gap_x = (rects[0].right() + rects[1].left()) / 2.0;
        assert!(bucket_at(&rects, gap_x).is_some());
        assert_eq!(bucket_at(&rects, 0.0), None);
        assert_eq!(bucket_at(&rects, 999.0), None);
        assert_eq!(bucket_at(&[], 50.0), None);
    }

    #[test]
    fn las_etiquetas_se_espacian_segun_la_cantidad_de_barras() {
        assert_eq!(label_step(7), 1);
        assert_eq!(label_step(12), 1);
        assert_eq!(label_step(24), 3);
        assert_eq!(label_step(31), 5);
    }
}
