mod albums;
mod artists;
mod hero;
pub mod layout;
pub mod queue_panel;
mod songs;

use super::textures::TextureCache;
use crate::state::{ActiveTab, AppState};
use eframe::egui;

/// Contenido central según la pestaña y el detalle abierto.
pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    match state.active_tab {
        ActiveTab::Artists if state.selected_artist.is_none() => artists::show(ui, state, textures),
        ActiveTab::Albums if state.selected_album.is_none() => albums::show(ui, state, textures),
        _ => songs::show(ui, state, textures),
    }
}
