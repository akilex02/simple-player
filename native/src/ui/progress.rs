use super::widgets::slider::PillSlider;
use super::{format_time, Size};
use crate::state::AppState;
use crate::theme;
use eframe::egui;

const TIME_W: f32 = 44.0;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, size: Size) {
    let duration = state.current_song().map(|s| s.duration_secs).unwrap_or(0);
    let max_width = match size {
        Size::Compact => 640.0,
        Size::Large => 760.0,
    };
    let slider_w = (ui.available_width() - 2.0 * (TIME_W + ui.spacing().item_spacing.x)).clamp(80.0, max_width);
    let mut fraction = if duration == 0 { 0.0 } else { (state.current_time / duration as f64) as f32 };

    ui.horizontal(|ui| {
        let large = size == Size::Large;
        let time_label = |text: String| {
            let rich = egui::RichText::new(text);
            if large {
                rich.font(theme::bold(theme::text::SM)).color(theme::TEXT_MAIN)
            } else {
                rich.size(theme::text::XS).color(theme::TEXT_MUTED)
            }
        };
        ui.add_sized([TIME_W, 18.0], egui::Label::new(time_label(format_time(state.current_time as u64))));

        let total = duration as f32;
        let response = PillSlider::new(&mut fraction, slider_w)
            .tooltip(move |v| format_time((v * total) as u64))
            .show(ui);

        if response.drag_started() {
            state.is_dragging_seek = true;
        }
        if response.changed() {
            state.current_time = (fraction * total) as f64;
        }
        if response.drag_stopped() || (response.clicked() && !response.dragged()) {
            state.seek_commit((fraction * total) as f64);
        }

        ui.add_sized([TIME_W, 18.0], egui::Label::new(time_label(format_time(duration))));
    });
}
