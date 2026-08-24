use super::Size;
use crate::state::AppState;
use crate::theme;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, size: Size) {
    let slider_width = match size {
        Size::Compact => 100.0,
        Size::Large => 150.0,
    };

    ui.horizontal(|ui| {
        let norm_bg = if state.is_normalize_volume { theme::ACCENT_PINK } else { theme::BG_CARD };
        let norm_fg = if state.is_normalize_volume {
            egui::Color32::from_rgb(0x13, 0x13, 0x22)
        } else {
            theme::TEXT_MUTED
        };
        if ui
            .add(egui::Button::new(egui::RichText::new("NORM").color(norm_fg).size(11.0)).fill(norm_bg))
            .on_hover_text("Normalización de audio")
            .clicked()
        {
            state.toggle_normalize_volume();
        }

        let mute_icon = if state.is_muted || state.volume == 0.0 { "🔇" } else { "🔊" };
        if ui.button(mute_icon).clicked() {
            state.toggle_mute();
        }

        let mut vol = state.volume as f32;
        let slider = egui::Slider::new(&mut vol, 0.0..=1.0).show_value(false);
        if ui.add_sized([slider_width, 18.0], slider).changed() {
            state.set_volume(vol as f64);
        }

        ui.label(format!("{}%", (state.volume * 100.0).round() as i32));
    });
}
