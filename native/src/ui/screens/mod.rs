use super::textures::TextureCache;
use crate::state::{ActiveTab, AppState};
use eframe::egui;

/// Contenido central según la pestaña y el detalle abierto.
pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    let show_artists_grid = state.active_tab == ActiveTab::Artists && state.selected_artist.is_none();
    if show_artists_grid {
        super::artists_grid::show(ui, state, textures);
    } else {
        super::song_table::show(ui, state, textures);
    }
}
