use super::widgets::chip::chip;
use super::widgets::icon_button::IconButton;
use super::widgets::slider::PillSlider;
use super::Size;
use crate::state::AppState;
use crate::theme::icons;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, size: Size) {
    let slider_width = match size {
        Size::Compact => 96.0,
        Size::Large => 140.0,
    };

    ui.horizontal_centered(|ui| {
        if chip(ui, "NORM", state.is_normalize_volume).on_hover_text("Normalización de audio").clicked() {
            state.toggle_normalize_volume();
        }

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
}
