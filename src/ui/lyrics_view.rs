/// Posición superior de cada línea, con `gap` entre líneas.
pub fn line_tops(heights: &[f32], gap: f32) -> Vec<f32> {
    let mut y = 0.0;
    heights
        .iter()
        .map(|h| {
            let top = y;
            y += h + gap;
            top
        })
        .collect()
}

pub fn total_height(heights: &[f32], gap: f32) -> f32 {
    if heights.is_empty() {
        return 0.0;
    }
    heights.iter().sum::<f32>() + gap * (heights.len() - 1) as f32
}

/// Desplazamiento que deja la línea activa un poco por encima del centro de
/// la vista, sin salirse del contenido.
pub fn target_offset(tops: &[f32], heights: &[f32], active: usize, view_h: f32, total: f32) -> f32 {
    let (Some(top), Some(height)) = (tops.get(active), heights.get(active)) else { return 0.0 };
    let desired = top + height / 2.0 - view_h * 0.38;
    desired.clamp(0.0, (total - view_h).max(0.0))
}

/// Opacidad de una línea según su distancia a la activa.
pub fn distance_alpha(distance: usize) -> f32 {
    match distance {
        0 => 1.0,
        1 => 0.5,
        2 => 0.36,
        _ => 0.26,
    }
}

/// Movimiento exponencial hacia `target`, independiente del dt.
pub fn approach(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    current + (target - current) * (1.0 - (-dt / tau).exp())
}


// ── Vista de letras (estilo Apple Music) ──────────────────────────────────

use crate::lyrics::Lyrics;
use crate::theme;
use eframe::egui::{self, Color32};
use std::sync::Arc;

const GAP: f32 = 24.0;
const PAD_X: f32 = 12.0;
const MANUAL_SCROLL_SECS: f64 = 3.0;

/// Letras con la línea activa nítida y el resto atenuado según su distancia,
/// scroll suave animado, clic en una línea para hacer seek (si está
/// sincronizada) y scroll manual que pausa el seguimiento automático.
#[derive(Default)]
pub struct LyricsView {
    offset: f32,
    manual_until: f64,
    track: String,
    cache: Option<(f32, String, Vec<Arc<egui::Galley>>)>,
}

impl LyricsView {
    /// Dibuja las letras en `rect`. Devuelve los segundos a los que hay que
    /// saltar si el usuario hizo clic en una línea.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        lyrics: &Option<Lyrics>,
        active: Option<usize>,
        track: &str,
    ) -> Option<f64> {
        let (texts, times): (Vec<&str>, Option<Vec<u64>>) = match lyrics {
            Some(Lyrics::Synced(lines)) => (lines.iter().map(|l| l.text.as_str()).collect(), Some(lines.iter().map(|l| l.time_ms).collect())),
            Some(Lyrics::Plain(lines)) => (lines.iter().map(String::as_str).collect(), None),
            None => (Vec::new(), None),
        };
        let painter = ui.painter_at(rect);
        if texts.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No hay letra disponible para esta canción",
                egui::FontId::proportional(theme::text::MD),
                theme::TEXT_MUTED,
            );
            return None;
        }

        let ctx = ui.ctx().clone();
        let galleys = self.galleys(&ctx, &texts, track, rect.width() - 2.0 * PAD_X);
        let heights: Vec<f32> = galleys.iter().map(|g| g.size().y).collect();
        let lead = rect.height() * 0.38;
        let tops: Vec<f32> = line_tops(&heights, GAP).into_iter().map(|t| t + lead).collect();
        let total = lead + total_height(&heights, GAP) + rect.height() * 0.5;
        let max_offset = (total - rect.height()).max(0.0);

        let (dt, now) = ctx.input(|i| (i.stable_dt.min(0.1), i.time));
        let track_changed = self.track != track;
        if track_changed {
            self.track = track.to_string();
            self.offset = 0.0;
            self.manual_until = 0.0;
        }

        if ui.rect_contains_pointer(rect) {
            let wheel = ctx.input(|i| i.smooth_scroll_delta.y);
            if wheel != 0.0 {
                self.offset = (self.offset - wheel).clamp(0.0, max_offset);
                self.manual_until = now + MANUAL_SCROLL_SECS;
            }
        }
        if times.is_some() && now >= self.manual_until {
            if let Some(active) = active {
                let target = target_offset(&tops, &heights, active, rect.height(), total);
                self.offset = if track_changed { target } else { approach(self.offset, target, dt, 0.2) };
                if (self.offset - target).abs() > 0.5 {
                    ctx.request_repaint();
                }
            }
        }

        let mut seek = None;
        for (i, galley) in galleys.iter().enumerate() {
            let y = rect.top() + tops[i] - self.offset;
            if y + heights[i] < rect.top() {
                continue;
            }
            if y > rect.bottom() {
                break;
            }
            let pos = egui::pos2(rect.left() + PAD_X, y);
            let target_alpha = match (active, &times) {
                (Some(a), Some(_)) => distance_alpha(a.abs_diff(i)),
                (None, Some(_)) => 0.4,
                _ => 0.9,
            };
            let alpha = ctx.animate_value_with_time(ui.id().with(("lyric_alpha", i)), target_alpha, 0.25);
            let hit = egui::Rect::from_min_size(pos, galley.size()).expand2(egui::vec2(8.0, GAP / 2.0 - 2.0));

            if let Some(times) = &times {
                let response = ui.interact(hit.intersect(rect), ui.id().with(("lyric_line", i)), egui::Sense::click());
                if response.hovered() {
                    painter.rect_filled(hit, theme::radius::SM, Color32::from_white_alpha(14));
                    ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if response.clicked() {
                    seek = Some(times[i] as f64 / 1000.0);
                }
            }
            let color = Color32::from_white_alpha((alpha.clamp(0.0, 1.0) * 255.0) as u8);
            painter.galley_with_override_text_color(pos, galley.clone(), color);
        }
        seek
    }

    fn galleys(&mut self, ctx: &egui::Context, texts: &[&str], track: &str, width: f32) -> Vec<Arc<egui::Galley>> {
        if let Some((w, t, galleys)) = &self.cache {
            if (*w - width).abs() < 0.5 && t == track && galleys.len() == texts.len() {
                return galleys.clone();
            }
        }
        let font = theme::bold(theme::text::XL + 4.0);
        let galleys: Vec<Arc<egui::Galley>> = ctx.fonts(|fonts| {
            texts
                .iter()
                .map(|text| fonts.layout(if text.is_empty() { "♪".to_string() } else { text.to_string() }, font.clone(), Color32::WHITE, width))
                .collect()
        });
        self.cache = Some((width, track.to_string(), galleys.clone()));
        galleys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn las_posiciones_acumulan_alturas_y_separacion() {
        assert_eq!(line_tops(&[10.0, 20.0, 30.0], 5.0), vec![0.0, 15.0, 40.0]);
        assert_eq!(total_height(&[10.0, 20.0, 30.0], 5.0), 70.0);
        assert_eq!(total_height(&[], 5.0), 0.0);
    }

    fn uniform(n: usize) -> (Vec<f32>, Vec<f32>, f32) {
        let heights = vec![30.0; n];
        (line_tops(&heights, 10.0), heights.clone(), total_height(&heights, 10.0))
    }

    #[test]
    fn la_linea_activa_se_centra_un_poco_por_encima_de_la_mitad() {
        let (tops, heights, total) = uniform(20);
        let offset = target_offset(&tops, &heights, 10, 400.0, total);
        assert!((offset - 263.0).abs() < 1e-3, "{offset}");
    }

    #[test]
    fn el_desplazamiento_no_sale_del_contenido() {
        let (tops, heights, total) = uniform(20);
        assert_eq!(target_offset(&tops, &heights, 0, 400.0, total), 0.0);
        assert_eq!(target_offset(&tops, &heights, 19, 400.0, total), total - 400.0);
    }

    #[test]
    fn si_todo_cabe_no_hay_desplazamiento() {
        let (tops, heights, total) = uniform(3);
        assert_eq!(target_offset(&tops, &heights, 2, 400.0, total), 0.0);
    }

    #[test]
    fn la_opacidad_baja_con_la_distancia_y_tiene_piso() {
        assert_eq!(distance_alpha(0), 1.0);
        let values: Vec<f32> = (0..8).map(distance_alpha).collect();
        assert!(values.windows(2).all(|w| w[0] >= w[1]), "{values:?}");
        assert!(values[1] < 1.0 && values[7] >= 0.2, "{values:?}");
    }

    #[test]
    fn approach_no_se_mueve_sin_tiempo_y_llega_con_mucho() {
        assert_eq!(approach(10.0, 100.0, 0.0, 0.2), 10.0);
        assert!((approach(10.0, 100.0, 5.0, 0.2) - 100.0).abs() < 0.01);
        let mid = approach(0.0, 100.0, 0.2, 0.2);
        assert!(mid > 50.0 && mid < 70.0, "{mid}");
    }
}
