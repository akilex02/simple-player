use super::{lyrics_panel, progress, transport, volume, Size};
use crate::state::AppState;
use crate::theme;
use eframe::egui;

/// Overlay de pantalla completa: portada/info a la izquierda, letras a la
/// derecha (si están activas) con transición animada de opacidad, y
/// controles ampliados abajo.
pub fn show(ctx: &egui::Context, state: &mut AppState, last_active_lyric: &mut Option<usize>) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(theme::BG_DARK).inner_margin(40.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("REPRODUCIENDO AHORA")
                        .color(theme::ACCENT_PINK)
                        .size(12.0)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕ Cerrar").clicked() {
                        state.close_fullscreen();
                    }
                    let label = if state.show_lyrics { "Ocultar letra" } else { "Ver letra" };
                    if ui.button(label).clicked() {
                        state.toggle_lyrics_visibility();
                    }
                });
            });

            ui.add_space(24.0);

            // Coincide con LYRICS_TRANSITION_MS (300ms) de la versión React.
            let anim = ctx.animate_bool_with_time(egui::Id::new("lyrics_column_anim"), state.show_lyrics, 0.3);

            let body_height = ui.available_height() - 140.0;
            ui.horizontal(|ui| {
                ui.set_height(body_height);

                let left_width = if anim > 0.0 {
                    ui.available_width() * 0.42
                } else {
                    ui.available_width()
                };

                ui.allocate_ui(egui::vec2(left_width, body_height), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(body_height * 0.15);
                        egui::Frame::none()
                            .fill(theme::BG_CARD_HOVER)
                            .rounding(egui::Rounding::same(16.0))
                            .inner_margin(50.0)
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new("🎧").size(72.0));
                            });
                        ui.add_space(20.0);
                        match state.current_song() {
                            Some(song) => {
                                ui.label(egui::RichText::new(&song.title).size(30.0).color(theme::TEXT_MAIN).strong());
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(&song.artist).size(17.0).color(theme::ACCENT_PINK));
                                ui.label(egui::RichText::new(&song.album).size(13.0).color(theme::TEXT_MUTED));
                            }
                            None => {
                                ui.label(egui::RichText::new("Sin canción").color(theme::TEXT_MUTED));
                            }
                        }
                    });
                });

                if anim > 0.01 {
                    ui.separator();
                    ui.scope(|ui| {
                        ui.set_opacity(anim);
                        ui.allocate_ui(egui::vec2(ui.available_width(), body_height), |ui| {
                            let active_index = state.active_lyric_line_index();
                            lyrics_panel::show(ui, &state.lyrics, active_index, last_active_lyric);
                        });
                    });
                }
            });

            ui.add_space(16.0);
            ui.vertical_centered(|ui| {
                progress::show(ui, state, Size::Large);
                ui.add_space(14.0);
                transport::show(ui, state, Size::Large);
                ui.add_space(14.0);
                volume::show(ui, state, Size::Large);
            });
        });
}
