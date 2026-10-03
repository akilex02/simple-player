use super::gradient::horizontal_gradient_mesh;
use crate::theme;
use eframe::egui;

/// Valor 0..1 que corresponde a la posición `x` del puntero sobre una pista
/// que empieza en `left` y mide `width`.
pub fn value_from_pointer(x: f32, left: f32, width: f32) -> f32 {
    if width <= 0.0 {
        return 0.0;
    }
    ((x - left) / width).clamp(0.0, 1.0)
}

/// Slider de píldora: pista con degradado "atardecer", thumb que aparece en
/// hover/arrastre y burbuja con el valor bajo el puntero.
pub struct PillSlider<'a> {
    value: &'a mut f32,
    width: f32,
    tooltip: Option<Box<dyn Fn(f32) -> String + 'a>>,
}

impl<'a> PillSlider<'a> {
    pub fn new(value: &'a mut f32, width: f32) -> Self {
        Self { value, width, tooltip: None }
    }

    pub fn tooltip(mut self, text: impl Fn(f32) -> String + 'a) -> Self {
        self.tooltip = Some(Box::new(text));
        self
    }

    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        const THUMB_R: f32 = 7.0;
        let (rect, mut response) = ui.allocate_exact_size(egui::vec2(self.width, 22.0), egui::Sense::click_and_drag());
        let response_id = response.id;
        let track_left = rect.left() + THUMB_R + 1.0;
        let track_w = rect.width() - 2.0 * (THUMB_R + 1.0);

        if response.is_pointer_button_down_on() || response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                *self.value = value_from_pointer(pos.x, track_left, track_w);
                response.mark_changed();
            }
        }
        if response.has_focus() {
            // Que las flechas izquierda/derecha muevan el slider en vez del foco.
            ui.memory_mut(|m| {
                m.set_focus_lock_filter(response_id, egui::EventFilter { horizontal_arrows: true, ..Default::default() })
            });
            let (left, right, shift) = ui.input(|i| {
                (i.key_pressed(egui::Key::ArrowLeft), i.key_pressed(egui::Key::ArrowRight), i.modifiers.shift)
            });
            let adjusted = crate::shortcuts::adjust_with_keys(*self.value, left, right, shift);
            if adjusted != *self.value {
                *self.value = adjusted;
                response.mark_changed();
            }
        }
        if !ui.is_rect_visible(rect) {
            return response;
        }

        let active = response.hovered() || response.dragged() || response.has_focus();
        let hot = ui.ctx().animate_bool_with_time(response_id, active, theme::motion::HOVER);
        let painter = ui.painter();
        let height = 4.0 + 2.0 * hot;
        let track = egui::Rect::from_center_size(
            egui::pos2(track_left + track_w / 2.0, rect.center().y),
            egui::vec2(track_w, height),
        );
        painter.rect_filled(track, height / 2.0, egui::Color32::from_white_alpha(38));

        let fill_w = track_w * self.value.clamp(0.0, 1.0);
        if fill_w > 0.5 {
            let fill = egui::Rect::from_min_size(track.min, egui::vec2(fill_w, height));
            painter
                .with_clip_rect(fill)
                .add(horizontal_gradient_mesh(track, &theme::GRADIENT_SUNSET));
            painter.circle_filled(egui::pos2(track.left() + height / 2.0, track.center().y), height / 2.0, theme::ACCENT_PINK);
        }

        if response.has_focus() {
            super::paint_focus_ring(painter, rect.shrink2(egui::vec2(0.0, 3.0)), 8.0, theme::accent(ui.ctx()));
        }
        let thumb_x = track_left + fill_w;
        let r = THUMB_R * hot;
        if r > 0.5 {
            painter.circle_filled(egui::pos2(thumb_x, rect.center().y), r + 3.0, egui::Color32::from_white_alpha(40));
            painter.circle_filled(egui::pos2(thumb_x, rect.center().y), r, egui::Color32::WHITE);
        }

        if let (Some(text), true) = (&self.tooltip, active) {
            let shown = if response.dragged() {
                *self.value
            } else {
                response.hover_pos().map_or(*self.value, |p| value_from_pointer(p.x, track_left, track_w))
            };
            let x = track_left + track_w * shown;
            paint_bubble(ui, response_id, egui::pos2(x, rect.top() - 4.0), &text(shown));
        }
        response
    }
}

fn paint_bubble(ui: &egui::Ui, id: egui::Id, anchor: egui::Pos2, text: &str) {
    let layer = egui::LayerId::new(egui::Order::Tooltip, id.with("bubble"));
    let painter = ui.ctx().layer_painter(layer);
    let galley = painter.layout_no_wrap(text.to_string(), theme::bold(theme::text::XS), theme::TEXT_MAIN);
    let size = galley.size() + egui::vec2(14.0, 8.0);
    let screen = ui.ctx().screen_rect();
    let x = (anchor.x - size.x / 2.0).clamp(screen.left() + 4.0, screen.right() - size.x - 4.0);
    let rect = egui::Rect::from_min_size(egui::pos2(x, anchor.y - size.y), size);
    painter.rect_filled(rect, 8.0, egui::Color32::from_rgba_unmultiplied(0x22, 0x22, 0x38, 240));
    painter.rect_stroke(rect, 8.0, egui::Stroke::new(1.0_f32, theme::GLASS_BORDER));
    painter.galley(rect.min + egui::vec2(7.0, 4.0), galley, theme::TEXT_MAIN);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapea_los_extremos_y_el_centro() {
        assert_eq!(value_from_pointer(100.0, 100.0, 200.0), 0.0);
        assert_eq!(value_from_pointer(300.0, 100.0, 200.0), 1.0);
        assert_eq!(value_from_pointer(200.0, 100.0, 200.0), 0.5);
    }

    #[test]
    fn fuera_de_la_pista_se_satura() {
        assert_eq!(value_from_pointer(-50.0, 100.0, 200.0), 0.0);
        assert_eq!(value_from_pointer(900.0, 100.0, 200.0), 1.0);
    }

    #[test]
    fn una_pista_sin_ancho_da_cero() {
        assert_eq!(value_from_pointer(5.0, 5.0, 0.0), 0.0);
    }
}
