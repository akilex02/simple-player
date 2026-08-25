pub mod artists_grid;
pub mod fullscreen;
pub mod header;
pub mod lyrics_panel;
pub mod player_bar;
pub mod progress;
pub mod sidebar;
mod size;
pub mod song_table;
pub mod textures;
pub mod transport;
pub mod visualizers;
pub mod volume;

use crate::theme;
use eframe::egui;
use textures::TextureCache;

pub use size::Size;

pub fn format_time(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}

/// Dibuja una carátula cuadrada (o el placeholder 🎵/🎧 si no hay portada o
/// no se pudo cargar), cacheando la textura decodificada por ruta.
pub fn cover_thumb(
    ui: &mut egui::Ui,
    textures: &mut TextureCache,
    cover_path: &Option<String>,
    size: f32,
    rounding: f32,
    fallback_emoji: &str,
) {
    let tex = cover_path.as_deref().and_then(|p| textures.get_or_load(p));

    match tex {
        Some(tex) => {
            let image = egui::Image::new((tex.id(), tex.size_vec2()))
                .fit_to_exact_size(egui::vec2(size, size))
                .rounding(rounding);
            ui.add(image);
        }
        None => {
            egui::Frame::none()
                .fill(theme::BG_CARD_HOVER)
                .rounding(rounding)
                .show(ui, |ui| {
                    ui.set_min_size(egui::vec2(size, size));
                    ui.set_max_size(egui::vec2(size, size));
                    ui.centered_and_justified(|ui| {
                        ui.label(egui::RichText::new(fallback_emoji).size(size * 0.5));
                    });
                });
        }
    }
}
