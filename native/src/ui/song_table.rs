use super::format_time;
use crate::library::Song;
use crate::state::{AppState, SortField};
use crate::theme;
use eframe::egui;

const ROW_HEIGHT: f32 = 32.0;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    if let Some(artist) = state.selected_artist.clone() {
        ui.horizontal(|ui| {
            if ui.button("← Volver a Artistas").clicked() {
                state.selected_artist = None;
            }
            ui.heading(artist);
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
            ui.label(msg);
            if ui.button("Seleccionar carpeta de música").clicked() {
                state.select_folder_and_scan();
            }
        });
        return;
    }

    if ui
        .add(egui::Button::new("🔀 Reproducción Aleatoria").fill(theme::ACCENT_LIME))
        .clicked()
    {
        state.start_shuffle_play(songs.clone());
    }
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.add_space(48.0);
        if ui.button("Título").clicked() {
            state.set_sort(SortField::Title);
        }
        if ui.button("Artista").clicked() {
            state.set_sort(SortField::Artist);
        }
        if ui.button("Álbum").clicked() {
            state.set_sort(SortField::Album);
        }
        if ui.button("Duración").clicked() {
            state.set_sort(SortField::Duration);
        }
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

                ui.horizontal(|ui| {
                    ui.set_min_height(ROW_HEIGHT);
                    let text = egui::RichText::new(format!(
                        "{}   —   {}   —   {}   —   {}",
                        song.title,
                        song.artist,
                        song.album,
                        format_time(song.duration_secs)
                    ));
                    let text = if is_current { text.color(theme::ACCENT_PINK).strong() } else { text };

                    if ui.selectable_label(is_current, text).clicked() {
                        clicked_song = Some(song.clone());
                    }
                });
            }
        });

    if let Some(song) = clicked_song {
        state.handle_play_song_from_list(&song, Some(songs));
    }
}
