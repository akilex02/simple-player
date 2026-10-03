use super::widgets::icon_button::IconButton;
use super::Size;
use crate::state::{AppState, RepeatMode};
use crate::theme::icons;
use eframe::egui;

/// Espacio inicial para centrar una fila de ancho `content` en `available`.
pub fn centered_leading_space(available: f32, content: f32) -> f32 {
    ((available - content) / 2.0).max(0.0)
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState, size: Size) {
    let (icon_d, play_d, gap) = match size {
        Size::Compact => (34.0, 46.0, 12.0),
        Size::Large => (44.0, 64.0, 18.0),
    };

    // Sobre el visualizador (pantalla completa) los botones llevan un respaldo oscuro para que se distingan.
    let solid = size == Size::Large;
    let row_width = 4.0 * icon_d + play_d + 4.0 * gap;
    let leading = centered_leading_space(ui.available_width(), row_width);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.add_space(leading);

        if IconButton::new(icons::SHUFFLE, icon_d).solid(solid).active(state.is_shuffle).tooltip("Aleatorio").show(ui).clicked() {
            state.toggle_shuffle();
        }
        if IconButton::new(icons::SKIP_BACK, icon_d).solid(solid).tooltip("Anterior").show(ui).clicked() {
            state.handle_prev_song();
        }
        let (play_icon, play_tip) = if state.is_playing { (icons::PAUSE, "Pausar") } else { (icons::PLAY, "Reproducir") };
        if IconButton::new(play_icon, play_d).primary(true).pulse(state.is_playing).tooltip(play_tip).show(ui).clicked() {
            state.toggle_play_pause();
        }
        if IconButton::new(icons::SKIP_FORWARD, icon_d).solid(solid).tooltip("Siguiente").show(ui).clicked() {
            state.handle_next_song();
        }
        let repeat_icon = if state.repeat_mode == RepeatMode::One { icons::REPEAT_ONCE } else { icons::REPEAT };
        if IconButton::new(repeat_icon, icon_d).solid(solid).active(state.repeat_mode != RepeatMode::Off).tooltip("Repetir").show(ui).clicked() {
            state.toggle_repeat_mode();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_fila_queda_centrada() {
        // sobran 200 px: 100 a cada lado
        assert_eq!(centered_leading_space(400.0, 200.0), 100.0);
    }

    #[test]
    fn si_no_cabe_o_sobra_poco_no_hay_espacio_negativo() {
        assert_eq!(centered_leading_space(200.0, 300.0), 0.0);
        assert_eq!(centered_leading_space(201.0, 200.0), 0.5);
    }
}
