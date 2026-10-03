use super::artists::screen_header;
use super::layout::{format_total_duration, grid_layout, songs_label};
use crate::state::AppState;
use crate::theme::{self, icons, radius, space, text};
use crate::ui::textures::TextureCache;
use crate::ui::widgets::card::card;
use crate::ui::widgets::cover::cover;
use crate::ui::widgets::empty_state::empty_state;
use eframe::egui::{self, RichText};

const MIN_CARD_W: f32 = 184.0;
const GAP: f32 = 16.0;

/// Grilla de álbumes con carátula cuadrada.
pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    let albums = state.album_groups();
    screen_header(ui, state, "ÁLBUMES", albums.len());

    if albums.is_empty() {
        if empty_state(ui, icons::MAGNIFYING_GLASS, "Sin álbumes", "No hay resultados para tu búsqueda.", Some("Limpiar búsqueda")) {
            state.search_query.clear();
        }
        return;
    }

    let grid = grid_layout(ui.available_width() - 14.0, MIN_CARD_W, GAP, albums.len());
    let cover_size = grid.card_w - 2.0 * space::MD;
    let card_h = cover_size + 2.0 * space::MD + 96.0;
    let mut selected: Option<(String, String)> = None;

    ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, card_h, grid.rows, |ui, rows| {
        for row in rows {
            ui.horizontal(|ui| {
                for album in albums.iter().skip(row * grid.cols).take(grid.cols) {
                    let response = card(ui, egui::vec2(grid.card_w, card_h), |ui| {
                        cover(ui, textures, &album.cover, cover_size, radius::MD, icons::DISC, true);
                        ui.add_space(space::SM);
                        let name = if album.album.trim().is_empty() { "Álbum desconocido" } else { &album.album };
                        let artist = if album.artist.trim().is_empty() { "Artista desconocido" } else { &album.artist };
                        ui.add(egui::Label::new(RichText::new(name).font(theme::bold(text::BASE))).truncate());
                        ui.add(egui::Label::new(RichText::new(artist).size(text::SM).color(theme::TEXT_MUTED)).truncate());
                        let summary = format!("{} · {}", songs_label(album.count), format_total_duration(album.total_secs));
                        ui.add(egui::Label::new(RichText::new(summary).size(text::XS).color(theme::TEXT_MUTED)).truncate());
                    });
                    if response.clicked() {
                        selected = Some((album.album.clone(), album.artist.clone()));
                    }
                }
            });
        }
    });

    if let Some(album) = selected {
        state.selected_album = Some(album);
    }
}
