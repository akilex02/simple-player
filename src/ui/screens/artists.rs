use super::layout::{grid_layout, songs_label};
use crate::state::{AppState, SortDirection};
use crate::theme::{self, icons, space, text};
use crate::ui::textures::TextureCache;
use crate::ui::widgets::card::card;
use crate::ui::widgets::chip::chip;
use crate::ui::widgets::cover::cover;
use crate::ui::widgets::empty_state::empty_state;
use eframe::egui::{self, RichText};

const MIN_CARD_W: f32 = 176.0;
const GAP: f32 = 16.0;

/// Grilla de artistas con carátula circular.
pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    let groups = state.artist_groups();
    screen_header(ui, state, "ARTISTAS", groups.len());

    if groups.is_empty() {
        if empty_state(ui, icons::MAGNIFYING_GLASS, "Sin artistas", "No hay resultados para tu búsqueda.", Some("Limpiar búsqueda")) {
            state.search_query.clear();
        }
        return;
    }

    let grid = grid_layout(ui.available_width() - 14.0, MIN_CARD_W, GAP, groups.len());
    let cover_size = grid.card_w - 2.0 * space::MD;
    let card_h = cover_size + 2.0 * space::MD + 78.0;
    let mut selected: Option<String> = None;

    ui.spacing_mut().item_spacing = egui::vec2(GAP, GAP);
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, card_h, grid.rows, |ui, rows| {
        for row in rows {
            ui.horizontal(|ui| {
                for group in groups.iter().skip(row * grid.cols).take(grid.cols) {
                    let response = card(ui, egui::vec2(grid.card_w, card_h), |ui| {
                        cover(ui, textures, &group.representative_cover, cover_size, cover_size / 2.0, icons::USER, true);
                        ui.add_space(space::SM);
                        ui.add(egui::Label::new(RichText::new(&group.artist).font(theme::bold(text::BASE))).truncate());
                        ui.label(RichText::new(songs_label(group.count)).size(text::SM).color(theme::TEXT_MUTED));
                    });
                    if response.clicked() {
                        selected = Some(group.artist.clone());
                    }
                }
            });
        }
    });

    if let Some(artist) = selected {
        state.selected_artist = Some(artist);
    }
}

/// Título de sección con contador y orden alfabético (compartido con álbumes).
pub fn screen_header(ui: &mut egui::Ui, state: &mut AppState, title: &str, count: usize) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(title).font(theme::deco(text::XL + 4.0)).color(theme::accent(ui.ctx())));
        ui.label(RichText::new(count.to_string()).size(text::SM).color(theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let desc = state.artist_sort_order == SortDirection::Desc;
            if chip(ui, if desc { "Z–A" } else { "A–Z" }, true).on_hover_text("Cambiar orden").clicked() {
                state.toggle_artist_sort_order();
            }
        });
    });
    ui.add_space(space::LG);
}
