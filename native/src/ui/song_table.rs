use super::format_time;
use super::textures::TextureCache;
use crate::library::Song;
use crate::state::{AppState, SortField};
use crate::theme;
use eframe::egui;

const ROW_HEIGHT: f32 = 40.0;
const COVER_SIZE: f32 = 30.0;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    if let Some(artist) = state.selected_artist.clone() {
        ui.horizontal(|ui| {
            if ui.button("← Volver a Artistas").clicked() {
                state.selected_artist = None;
            }
            ui.add_space(8.0);
            ui.label(egui::RichText::new(&artist).size(20.0).color(theme::TEXT_MAIN).strong());
        });
        ui.add_space(8.0);
    }

    let songs: Vec<Song> = state.sorted_songs().into_iter().cloned().collect();

    if songs.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);
            ui.label(egui::RichText::new("🎵").size(48.0));
            let msg = match &state.current_folder_path {
                Some(p) => format!("No hay canciones en \"{p}\""),
                None => "No se encontraron canciones".to_string(),
            };
            ui.label(egui::RichText::new(msg).color(theme::TEXT_MUTED));
            if ui.button("Seleccionar carpeta de música").clicked() {
                state.select_folder_and_scan();
            }
        });
        return;
    }

    if ui
        .add(
            egui::Button::new(egui::RichText::new("🔀 Reproducción Aleatoria").color(egui::Color32::from_rgb(0x13, 0x13, 0x22)))
                .fill(theme::ACCENT_LIME)
                .rounding(egui::Rounding::same(9999.0)),
        )
        .clicked()
    {
        state.start_shuffle_play(songs.clone());
    }
    ui.add_space(10.0);

    let title_w = 320.0;
    let artist_w = 200.0;
    let album_w = 200.0;

    ui.horizontal(|ui| {
        ui.add_space(COVER_SIZE + 10.0);
        sort_header(ui, "Título", title_w, state, SortField::Title);
        sort_header(ui, "Artista", artist_w, state, SortField::Artist);
        sort_header(ui, "Álbum", album_w, state, SortField::Album);
        sort_header(ui, "Duración", 80.0, state, SortField::Duration);
    });
    ui.separator();

    let current_path = state.current_song().map(|s| s.path.clone());
    let mut clicked_song: Option<Song> = None;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, songs.len(), |ui, row_range| {
            for i in row_range {
                let song = &songs[i];
                let is_current = current_path.as_deref() == Some(song.path.as_str());

                let row_rect = ui
                    .horizontal(|ui| {
                        ui.set_min_height(ROW_HEIGHT);
                        super::cover_thumb(ui, textures, &song.cover_art, COVER_SIZE, 6.0, "🎵");
                        ui.add_space(10.0);

                        let title_color = if is_current { theme::ACCENT_PINK } else { theme::TEXT_MAIN };
                        ui.add_sized(
                            [title_w, ROW_HEIGHT],
                            egui::Label::new(egui::RichText::new(&song.title).color(title_color).strong()),
                        );
                        ui.add_sized(
                            [artist_w, ROW_HEIGHT],
                            egui::Label::new(egui::RichText::new(&song.artist).color(theme::TEXT_MUTED)),
                        );
                        ui.add_sized(
                            [album_w, ROW_HEIGHT],
                            egui::Label::new(egui::RichText::new(&song.album).color(theme::TEXT_MUTED)),
                        );
                        ui.label(egui::RichText::new(format_time(song.duration_secs)).color(theme::TEXT_MUTED));
                    })
                    .response
                    .rect;

                let response = ui.interact(row_rect, ui.id().with(("song_row", i)), egui::Sense::click());
                if is_current {
                    ui.painter().rect_filled(
                        row_rect.expand2(egui::vec2(0.0, 2.0)),
                        6.0,
                        theme::ACCENT_PINK.gamma_multiply(0.12),
                    );
                } else if response.hovered() {
                    ui.painter()
                        .rect_filled(row_rect.expand2(egui::vec2(0.0, 2.0)), 6.0, theme::BG_CARD);
                }
                if response.clicked() {
                    clicked_song = Some(song.clone());
                }
            }
        });

    if let Some(song) = clicked_song {
        state.handle_play_song_from_list(&song, Some(songs));
    }
}

fn sort_header(ui: &mut egui::Ui, label: &str, width: f32, state: &mut AppState, field: SortField) {
    let indicator = if state.sort_field == field {
        if state.sort_direction == crate::state::SortDirection::Asc { " ▲" } else { " ▼" }
    } else {
        ""
    };
    let text = egui::RichText::new(format!("{label}{indicator}")).color(theme::TEXT_MUTED).size(12.0).strong();
    if ui.add_sized([width, 20.0], egui::Button::new(text).frame(false)).clicked() {
        state.set_sort(field);
    }
}
