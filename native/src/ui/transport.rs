use super::Size;
use crate::state::{AppState, RepeatMode};
use crate::theme;
use eframe::egui;

fn pill_button(ui: &mut egui::Ui, icon: &str, diameter: f32, active: bool) -> egui::Response {
    let (bg, fg) = if active {
        (theme::ACCENT_PINK, egui::Color32::from_rgb(0x13, 0x13, 0x22))
    } else {
        (theme::BG_CARD, theme::TEXT_MUTED)
    };
    let button = egui::Button::new(egui::RichText::new(icon).size(diameter * 0.55).color(fg))
        .fill(bg)
        .rounding(egui::Rounding::same(9999.0))
        .min_size(egui::vec2(diameter, diameter));
    ui.add(button)
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState, size: Size) {
    let (icon_d, play_d, gap) = match size {
        Size::Compact => (36.0, 52.0, 14.0),
        Size::Large => (44.0, 64.0, 18.0),
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;

        if pill_button(ui, "🔀", icon_d, state.is_shuffle).clicked() {
            state.toggle_shuffle();
        }
        if pill_button(ui, "⏮", icon_d, false).clicked() {
            state.handle_prev_song();
        }
        if pill_button(ui, if state.is_playing { "⏸" } else { "▶" }, play_d, true).clicked() {
            state.toggle_play_pause();
        }
        if pill_button(ui, "⏭", icon_d, false).clicked() {
            state.handle_next_song();
        }
        let repeat_icon = if state.repeat_mode == RepeatMode::One { "🔂" } else { "🔁" };
        if pill_button(ui, repeat_icon, icon_d, state.repeat_mode != RepeatMode::Off).clicked() {
            state.toggle_repeat_mode();
        }
    });
}
