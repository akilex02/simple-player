pub mod artists_grid;
pub mod fullscreen;
pub mod gallery;
pub mod lyrics_panel;
pub mod progress;
pub mod screens;
pub mod shell;
mod size;
pub mod backdrop;
pub mod crossfade;
pub mod song_table;
pub mod textures;
pub mod transport;
pub mod visualizers;
pub mod widgets;
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

/// Dibuja una carátula cuadrada (o el placeholder si no hay portada o aún no
/// cargó). Solo pide la textura si el cuadro está a la vista, y la pide al
/// tamaño que de verdad se va a dibujar.
pub fn cover_thumb(
    ui: &mut egui::Ui,
    textures: &mut TextureCache,
    cover_path: &Option<String>,
    size: f32,
    rounding: f32,
    fallback_emoji: &str,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    if !ui.is_rect_visible(rect) {
        return;
    }

    let cover_size = textures::CoverSize::for_physical_px(size * ui.ctx().pixels_per_point());
    let tex = cover_path.as_deref().and_then(|p| textures.get_or_load(p, cover_size));

    match tex {
        Some(tex) => {
            egui::Image::new((tex.id(), tex.size_vec2())).rounding(rounding).paint_at(ui, rect);
        }
        None => {
            ui.painter().rect_filled(rect, rounding, theme::BG_CARD_HOVER);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                fallback_emoji,
                egui::FontId::proportional(size * 0.5),
                theme::TEXT_MAIN,
            );
        }
    }
}
