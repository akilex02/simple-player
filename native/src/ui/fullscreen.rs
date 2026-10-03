use super::textures::TextureCache;
use super::visualizers::{self, VisualizerMode};
use super::{lyrics_panel, progress, transport, volume, Size};
use crate::state::AppState;
use crate::theme;
use crate::viz::VizFrame;
use eframe::egui;

/// Overlay de pantalla completa: portada/info a la izquierda, letras a la
/// derecha (si están activas) con transición animada de opacidad, y
/// controles ampliados abajo. El fondo es el visualizador de espectro en
/// vivo (barras/radial/resplandor) en vez del blur pre-horneado de la
/// versión React.
pub fn show(
    ctx: &egui::Context,
    state: &mut AppState,
    textures: &mut TextureCache,
    last_active_lyric: &mut Option<usize>,
    viz: &VizFrame,
    visualizer_mode: &mut VisualizerMode,
) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(theme::BG_DARK).inner_margin(40.0))
        .show(ctx, |ui| {
            // Fondo: visualizador a pantalla completa + scrim oscuro para legibilidad.
            let full_rect = ui.max_rect();
            visualizers::draw(ui, full_rect, viz, *visualizer_mode);
            ui.painter_at(full_rect).rect_filled(
                full_rect,
                0.0,
                egui::Color32::from_black_alpha(150),
            );

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
                    if ui
                        .button(format!("🎨 {}", visualizer_mode.label()))
                        .on_hover_text("Cambiar visualizador")
                        .clicked()
                    {
                        *visualizer_mode = visualizer_mode.next();
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
                        let cover_path = state.current_song().and_then(|s| s.cover_art.clone());
                        super::cover_thumb(ui, textures, &cover_path, 220.0, 16.0, "🎧");
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
