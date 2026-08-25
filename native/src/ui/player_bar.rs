use super::textures::TextureCache;
use super::{progress, transport, volume, Size};
use crate::state::AppState;
use crate::theme;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache, spectrum: &[f32]) {
    ui.horizontal(|ui| {
        let cover_path = state.current_song().and_then(|s| s.cover_art.clone());
        super::cover_thumb(ui, textures, &cover_path, 64.0, 10.0, "🎧");

        match state.current_song() {
            Some(song) => {
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new(&song.title).color(theme::TEXT_MAIN).strong());
                    ui.label(egui::RichText::new(&song.artist).color(theme::ACCENT_PINK).size(12.0));
                });
            }
            None => {
                ui.label(egui::RichText::new("Ninguna canción").color(theme::TEXT_MUTED));
            }
        }

        ui.add_space(24.0);

        // ── Controles de transporte + progreso, centrados ────────────────
        ui.vertical(|ui| {
            transport::show(ui, state, Size::Compact);
            progress::show(ui, state, Size::Compact);
        });

        ui.add_space(16.0);

        // ── Espectro en vivo (miniatura) ──────────────────────────────────
        let (_, rect) = ui.allocate_space(egui::vec2(180.0, 50.0));
        let painter = ui.painter_at(rect);
        let gap = 2.0;
        let bar_width = (rect.width() - gap * (spectrum.len() as f32 - 1.0)) / spectrum.len() as f32;
        for (i, db) in spectrum.iter().enumerate() {
            let norm = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
            let h = norm * rect.height();
            let x = rect.left() + i as f32 * (bar_width + gap);
            let bar = egui::Rect::from_min_size(egui::pos2(x, rect.bottom() - h), egui::vec2(bar_width, h));
            painter.rect_filled(bar, 1.0, theme::ACCENT_PINK);
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("⛶").on_hover_text("Pantalla completa").clicked() {
                state.open_fullscreen();
            }
            let lyrics_active = state.show_lyrics && state.is_fullscreen;
            if ui
                .add(egui::Button::new("📃").fill(if lyrics_active { theme::ACCENT_PINK } else { theme::BG_CARD }))
                .on_hover_text("Ver letra")
                .clicked()
            {
                state.open_lyrics();
            }
            volume::show(ui, state, Size::Compact);
        });
    });
}
