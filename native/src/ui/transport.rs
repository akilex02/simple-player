use super::widgets::icon_button::IconButton;
use super::Size;
use crate::state::{AppState, RepeatMode};
use crate::theme::icons;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, size: Size) {
    let (icon_d, play_d, gap) = match size {
        Size::Compact => (34.0, 46.0, 12.0),
        Size::Large => (44.0, 64.0, 18.0),
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;

        if IconButton::new(icons::SHUFFLE, icon_d).active(state.is_shuffle).tooltip("Aleatorio").show(ui).clicked() {
            state.toggle_shuffle();
        }
        if IconButton::new(icons::SKIP_BACK, icon_d).tooltip("Anterior").show(ui).clicked() {
            state.handle_prev_song();
        }
        let (play_icon, play_tip) = if state.is_playing { (icons::PAUSE, "Pausar") } else { (icons::PLAY, "Reproducir") };
        if IconButton::new(play_icon, play_d).primary(true).tooltip(play_tip).show(ui).clicked() {
            state.toggle_play_pause();
        }
        if IconButton::new(icons::SKIP_FORWARD, icon_d).tooltip("Siguiente").show(ui).clicked() {
            state.handle_next_song();
        }
        let repeat_icon = if state.repeat_mode == RepeatMode::One { icons::REPEAT_ONCE } else { icons::REPEAT };
        if IconButton::new(repeat_icon, icon_d).active(state.repeat_mode != RepeatMode::Off).tooltip("Repetir").show(ui).clicked() {
            state.toggle_repeat_mode();
        }
    });
}
