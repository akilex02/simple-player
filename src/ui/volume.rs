use super::widgets::icon_button::IconButton;
use super::widgets::slider::PillSlider;
use super::Size;
use crate::shortcuts::volume_after_scroll;
use crate::state::AppState;
use crate::theme::icons;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, size: Size) {
    let slider_width = match size {
        Size::Compact => 96.0,
        Size::Large => 140.0,
    };

    let group = ui.horizontal_centered(|ui| {
        let muted = state.is_muted || state.volume == 0.0;
        let icon = if muted {
            icons::SPEAKER_X
        } else if state.volume < 0.4 {
            icons::SPEAKER_LOW
        } else {
            icons::SPEAKER_HIGH
        };
        if IconButton::new(icon, 32.0).tooltip(if muted { "Quitar silencio" } else { "Silenciar" }).show(ui).clicked() {
            state.toggle_mute();
        }

        let mut vol = state.volume as f32;
        if PillSlider::new(&mut vol, slider_width)
            .tooltip(|v| format!("{:.0} %", v * 100.0))
            .show(ui)
            .changed()
        {
            state.set_volume(vol as f64);
        }
    });

    // La rueda sobre el control sube o baja el volumen (y no desplaza lo que haya detrás).
    if ui.rect_contains_pointer(group.response.rect) {
        let scroll = ui.input(|i| i.raw_scroll_delta.y);
        if scroll != 0.0 {
            state.set_volume(volume_after_scroll(state.volume, scroll));
            ui.input_mut(|i| {
                i.raw_scroll_delta = egui::Vec2::ZERO;
                i.smooth_scroll_delta = egui::Vec2::ZERO;
            });
        }
    }
}
