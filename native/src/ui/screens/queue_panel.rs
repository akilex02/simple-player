use super::layout::songs_label;
use crate::state::AppState;
use crate::theme::{self, icons, radius, space, text, with_alpha};
use crate::ui::format_time;
use crate::ui::textures::TextureCache;
use crate::ui::widgets::cover::{cover, paint_cover};
use crate::ui::widgets::empty_state::empty_state;
use crate::ui::widgets::icon_button::IconButton;
use eframe::egui::{self, FontId, RichText};

const ROW_H: f32 = 52.0;

/// Panel derecho: pista actual y lo que viene a continuación en la cola.
pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("COLA").font(theme::deco(text::XL)).color(theme::accent(ui.ctx())));
        ui.label(RichText::new(songs_label(state.active_queue.len())).size(text::SM).color(theme::TEXT_MUTED));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if IconButton::new(icons::X, 30.0).tooltip("Cerrar").show(ui).clicked() {
                state.show_queue = false;
            }
        });
    });
    ui.add_space(space::LG);

    let Some(current) = state.current_song_index.filter(|&i| i < state.active_queue.len()) else {
        empty_state(ui, icons::QUEUE, "La cola está vacía", "Reproduce una canción para empezar.", None);
        return;
    };

    section_label(ui, "REPRODUCIENDO AHORA");
    let now = state.active_queue[current].clone();
    ui.horizontal(|ui| {
        cover(ui, textures, &now.cover_art, 56.0, radius::SM, icons::MUSIC_NOTES, true);
        ui.vertical(|ui| {
            ui.add_space(8.0);
            ui.add(egui::Label::new(RichText::new(&now.title).font(theme::bold(text::BASE)).color(theme::accent(ui.ctx()))).truncate());
            ui.add(egui::Label::new(RichText::new(artist_name(&now.artist)).size(text::SM).color(theme::TEXT_MUTED)).truncate());
        });
    });
    ui.add_space(space::LG);

    let next_count = state.active_queue.len() - current - 1;
    section_label(ui, "A CONTINUACIÓN");
    if next_count == 0 {
        ui.label(RichText::new("No hay más canciones en la cola.").color(theme::TEXT_MUTED));
        return;
    }

    let mut clicked: Option<usize> = None;
    ui.spacing_mut().item_spacing.y = 4.0;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show_rows(ui, ROW_H, next_count, |ui, range| {
        for i in range {
            let queue_index = current + 1 + i;
            let song = &state.active_queue[queue_index];
            let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), egui::Sense::click());
            if response.hovered() {
                ui.painter().rect_filled(rect, radius::SM, egui::Color32::from_white_alpha(16));
            }
            let cover_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 24.0, rect.center().y), egui::vec2(40.0, 40.0));
            paint_cover(ui, cover_rect, textures, &song.cover_art, 6.0, icons::MUSIC_NOTES, false);
            let text_area = egui::Rect::from_min_max(egui::pos2(cover_rect.right() + 10.0, rect.top()), egui::pos2(rect.right() - 52.0, rect.bottom()));
            let painter = ui.painter().with_clip_rect(text_area);
            painter.text(egui::pos2(text_area.left(), rect.center().y - 9.0), egui::Align2::LEFT_CENTER, &song.title, theme::bold(text::BASE), theme::TEXT_MAIN);
            painter.text(egui::pos2(text_area.left(), rect.center().y + 10.0), egui::Align2::LEFT_CENTER, artist_name(&song.artist), FontId::proportional(text::SM), theme::TEXT_MUTED);
            ui.painter().text(egui::pos2(rect.right() - 8.0, rect.center().y), egui::Align2::RIGHT_CENTER, format_time(song.duration_secs), FontId::proportional(text::XS), with_alpha(theme::TEXT_MUTED, 220));
            if response.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                clicked = Some(queue_index);
            }
        }
    });

    if let Some(index) = clicked {
        state.play_index(index as i64, None);
    }
}

fn artist_name(artist: &str) -> &str {
    if artist.trim().is_empty() { "Artista desconocido" } else { artist }
}

fn section_label(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label).font(theme::bold(text::XS)).color(theme::TEXT_MUTED));
    ui.add_space(space::SM);
}
