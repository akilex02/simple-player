mod albums;
mod artists;
mod hero;
pub mod layout;
pub mod queue_panel;
mod settings;
mod songs;
mod stats;

use super::textures::TextureCache;
use crate::state::{ActiveTab, AppState};
use crate::theme;
use eframe::egui;
use std::hash::{Hash, Hasher};

/// La pantalla entra con un fundido corto cada vez que cambia la pestaña o el detalle abierto.
fn fade_in_on_change(ui: &mut egui::Ui, state: &AppState) {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    (state.active_tab as u8, &state.selected_artist, &state.selected_album).hash(&mut hasher);
    let key = hasher.finish();

    let ctx = ui.ctx().clone();
    let id = egui::Id::new("screen_fade");
    let now = ctx.input(|i| i.time);
    let (last_key, started): (u64, f64) = ctx.data(|d| d.get_temp(id)).unwrap_or((key, f64::NEG_INFINITY));
    let started = if last_key == key {
        started
    } else {
        ctx.data_mut(|d| d.insert_temp(id, (key, now)));
        now
    };
    let progress = theme::motion::fade_in((now - started) as f32, theme::motion::SCREEN);
    if progress < 1.0 {
        ctx.request_repaint();
        ui.set_opacity(0.15 + 0.85 * progress);
    }
}

/// Contenido central según la pestaña y el detalle abierto.
pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    fade_in_on_change(ui, state);
    match state.active_tab {
        ActiveTab::Artists if state.selected_artist.is_none() => artists::show(ui, state, textures),
        ActiveTab::Stats => stats::show(ui, state, textures),
        ActiveTab::Settings => settings::show(ui, state),
        ActiveTab::Albums if state.selected_album.is_none() => albums::show(ui, state, textures),
        _ => songs::show(ui, state, textures),
    }
}
