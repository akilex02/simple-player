use super::{format_time, Size};
use crate::state::AppState;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, size: Size) {
    let duration = state.current_song().map(|s| s.duration_secs).unwrap_or(0);
    let mut value = state.current_time as f32;
    let slider_width = match size {
        Size::Compact => 420.0,
        Size::Large => 620.0,
    };

    ui.horizontal(|ui| {
        ui.label(format_time(state.current_time as u64));

        let slider = egui::Slider::new(&mut value, 0.0..=(duration.max(1) as f32)).show_value(false);
        let response = ui.add_sized([slider_width, 18.0], slider);

        if response.drag_started() {
            state.is_dragging_seek = true;
        }
        if response.changed() {
            state.current_time = value as f64;
        }
        if response.drag_stopped() {
            state.seek_commit(value as f64);
        }

        ui.label(format_time(duration));
    });
}
