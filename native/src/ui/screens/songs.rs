use super::hero::{self, HeroAction, HeroInfo};
use super::layout::{column_widths, eq_heights, hero_text, HeroContext, TableColumns};
use crate::library::Song;
use crate::state::{AppState, SortDirection, SortField};
use crate::theme::{self, icons, radius, space, text, with_alpha};
use crate::ui::format_time;
use crate::ui::textures::TextureCache;
use crate::ui::widgets::cover::paint_cover;
use crate::ui::widgets::empty_state::empty_state;
use eframe::egui::{self, Color32, FontId};

const ROW_H: f32 = 52.0;
const COVER: f32 = 40.0;

/// Canciones de la biblioteca, de un artista o de un álbum, con encabezado y tabla virtualizada.
pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    if state.songs.is_empty() {
        let (title, subtitle, cta) = match &state.current_folder_path {
            Some(path) => (format!("No hay canciones en «{path}»"), "Prueba con otra carpeta.", "Elegir otra carpeta"),
            None => ("Elige tu carpeta de música".to_string(), "Simple Player buscará tus canciones ahí.", "Abrir carpeta"),
        };
        if empty_state(ui, icons::FOLDER_OPEN, &title, subtitle, Some(cta)) {
            state.select_folder_and_scan();
        }
        return;
    }

    let indices = state.visible_song_indices();
    let info = hero_info(state, &indices);
    let action = hero::show(ui, textures, &info);
    ui.add_space(space::LG);

    if indices.is_empty() {
        let query = state.search_query.trim().to_string();
        if empty_state(ui, icons::MAGNIFYING_GLASS, &format!("Sin resultados para «{query}»"), "Revisa la ortografía o prueba otra búsqueda.", Some("Limpiar búsqueda")) {
            state.search_query.clear();
        }
        return;
    }

    if action != HeroAction::None {
        let queue: Vec<Song> = indices.iter().map(|&i| state.songs[i].clone()).collect();
        match action {
            HeroAction::Play => {
                let first = queue[0].clone();
                state.handle_play_song_from_list(&first, Some(queue));
            }
            _ => state.start_shuffle_play(queue),
        }
        return;
    }

    let cols = column_widths(ui.available_width() - 14.0);
    header(ui, state, &cols);

    let current_path = state.current_song().map(|s| s.path.clone());
    let playing = state.is_playing;
    let accent = theme::accent(ui.ctx());
    let time = ui.input(|i| i.time) as f32;
    let mut clicked: Option<usize> = None;

    ui.spacing_mut().item_spacing.y = 4.0;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, ROW_H, indices.len(), |ui, range| {
        for i in range {
            let song = &state.songs[indices[i]];
            let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), egui::Sense::click());
            let is_current = current_path.as_deref() == Some(song.path.as_str());
            paint_row(ui, rect, &response, song, i + 1, is_current && playing, is_current, &cols, accent, time, textures);
            if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                clicked = Some(i);
            }
        }
    });

    if let Some(i) = clicked {
        let song = state.songs[indices[i]].clone();
        let queue: Vec<Song> = indices.iter().map(|&i| state.songs[i].clone()).collect();
        state.handle_play_song_from_list(&song, Some(queue));
    }
}

fn hero_info(state: &AppState, indices: &[usize]) -> HeroInfo {
    let total: u64 = indices.iter().map(|&i| state.songs[i].duration_secs).sum();
    let cover = indices.iter().find_map(|&i| state.songs[i].cover_art.clone());
    let query = state.search_query.trim().to_string();
    let context = match (&state.selected_album, &state.selected_artist) {
        (Some((album, artist)), _) => HeroContext::Album { album, artist },
        (None, Some(artist)) => HeroContext::Artist(artist),
        _ if !query.is_empty() => HeroContext::Search(&query),
        _ => HeroContext::Library,
    };
    let circle = matches!(context, HeroContext::Artist(_));
    let t = hero_text(context, indices.len(), total);
    HeroInfo { eyebrow: t.eyebrow, title: t.title, subtitle: t.subtitle, cover, circle }
}

fn header(ui: &mut egui::Ui, state: &mut AppState, cols: &TableColumns) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 28.0), egui::Sense::hover());
    ui.painter().text(
        egui::pos2(rect.left() + cols.index / 2.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        "#",
        theme::bold(text::XS),
        theme::TEXT_MUTED,
    );
    let title_x = rect.left() + cols.index + COVER + 12.0;
    sort_cell(ui, state, cell(title_x, rect, 70.0), "TÍTULO", SortField::Title, egui::Align2::LEFT_CENTER);
    sort_cell(ui, state, cell(title_x + 76.0, rect, 70.0), "ARTISTA", SortField::Artist, egui::Align2::LEFT_CENTER);
    if cols.album > 0.0 {
        let album_x = rect.left() + cols.index + cols.title + 8.0;
        sort_cell(ui, state, cell(album_x, rect, 120.0), "ÁLBUM", SortField::Album, egui::Align2::LEFT_CENTER);
    }
    let dur_right = rect.right() - 12.0;
    sort_cell(ui, state, cell(dur_right - 90.0, rect, 90.0), "DURACIÓN", SortField::Duration, egui::Align2::RIGHT_CENTER);
    ui.painter().hline(rect.x_range(), rect.bottom(), egui::Stroke::new(1.0_f32, theme::GLASS_BORDER));
}

fn cell(x: f32, row: egui::Rect, width: f32) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(x, row.top()), egui::vec2(width, row.height()))
}

fn sort_cell(ui: &mut egui::Ui, state: &mut AppState, rect: egui::Rect, label: &str, field: SortField, align: egui::Align2) {
    let active = state.sort_field == field && state.search_query.trim().is_empty();
    let id = ui.id().with(("sort_cell", label));
    let response = ui.interact(rect, id, egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
    let color = if active { theme::accent(ui.ctx()) } else if response.hovered() { theme::TEXT_MAIN } else { theme::TEXT_MUTED };
    let caret = match (active, state.sort_direction) {
        (true, SortDirection::Asc) => format!(" {}", icons::CARET_UP),
        (true, SortDirection::Desc) => format!(" {}", icons::CARET_DOWN),
        _ => String::new(),
    };
    let anchor = if align == egui::Align2::RIGHT_CENTER { egui::pos2(rect.right(), rect.center().y) } else { egui::pos2(rect.left(), rect.center().y) };
    ui.painter().text(anchor, align, format!("{label}{caret}"), theme::bold(text::XS), color);
    if response.clicked() {
        state.set_sort(field);
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_row(
    ui: &egui::Ui,
    rect: egui::Rect,
    response: &egui::Response,
    song: &Song,
    number: usize,
    animating: bool,
    is_current: bool,
    cols: &TableColumns,
    accent: Color32,
    time: f32,
    textures: &mut TextureCache,
) {
    let painter = ui.painter();
    if is_current {
        painter.rect_filled(rect, radius::SM, with_alpha(accent, 44));
    } else if response.hovered() {
        painter.rect_filled(rect, radius::SM, Color32::from_white_alpha(16));
    }

    let cy = rect.center().y;
    let index_center = egui::pos2(rect.left() + cols.index / 2.0, cy);
    if is_current {
        paint_equalizer(painter, index_center, eq_heights(time, animating), accent);
    } else if response.hovered() {
        painter.text(index_center, egui::Align2::CENTER_CENTER, icons::PLAY, FontId::proportional(text::LG), theme::TEXT_MAIN);
    } else {
        painter.text(index_center, egui::Align2::CENTER_CENTER, number.to_string(), FontId::proportional(text::SM), theme::TEXT_MUTED);
    }

    let title_left = rect.left() + cols.index;
    let cover_rect = egui::Rect::from_center_size(egui::pos2(title_left + COVER / 2.0, cy), egui::vec2(COVER, COVER));
    paint_cover(ui, cover_rect, textures, &song.cover_art, 6.0, icons::MUSIC_NOTES, false);

    let text_left = title_left + COVER + 12.0;
    let text_area = egui::Rect::from_min_max(egui::pos2(text_left, rect.top()), egui::pos2(title_left + cols.title - 12.0, rect.bottom()));
    let clipped = painter.with_clip_rect(text_area);
    let title_color = theme::TEXT_MAIN;
    let meta_color = if is_current { Color32::from_white_alpha(215) } else { theme::TEXT_MUTED };
    clipped.text(egui::pos2(text_left, cy - 9.0), egui::Align2::LEFT_CENTER, &song.title, theme::bold(text::BASE), title_color);
    let artist = if song.artist.trim().is_empty() { "Artista desconocido" } else { &song.artist };
    clipped.text(egui::pos2(text_left, cy + 10.0), egui::Align2::LEFT_CENTER, artist, FontId::proportional(text::SM), meta_color);

    if cols.album > 0.0 {
        let album_left = title_left + cols.title + 8.0;
        let area = egui::Rect::from_min_max(egui::pos2(album_left, rect.top()), egui::pos2(album_left + cols.album - 16.0, rect.bottom()));
        let album = if song.album.trim().is_empty() { "—" } else { &song.album };
        painter.with_clip_rect(area).text(egui::pos2(album_left, cy), egui::Align2::LEFT_CENTER, album, FontId::proportional(text::BASE), meta_color);
    }

    painter.text(
        egui::pos2(rect.right() - 12.0, cy),
        egui::Align2::RIGHT_CENTER,
        format_time(song.duration_secs),
        FontId::proportional(text::SM),
        meta_color,
    );
}

fn paint_equalizer(painter: &egui::Painter, center: egui::Pos2, heights: [f32; 3], color: Color32) {
    const BAR_W: f32 = 3.0;
    const GAP: f32 = 2.0;
    const MAX_H: f32 = 14.0;
    let total = 3.0 * BAR_W + 2.0 * GAP;
    for (i, h) in heights.iter().enumerate() {
        let x = center.x - total / 2.0 + i as f32 * (BAR_W + GAP);
        let bar = egui::Rect::from_min_max(egui::pos2(x, center.y + MAX_H / 2.0 - h * MAX_H), egui::pos2(x + BAR_W, center.y + MAX_H / 2.0));
        painter.rect_filled(bar, 1.0, color);
    }
}
