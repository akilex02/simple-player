use super::textures::TextureCache;
use crate::state::AppState;
use crate::theme;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    let groups = state.artist_groups();

    ui.horizontal(|ui| {
        ui.label(format!("Artistas ({})", groups.len()));
        if ui.button("Cambiar orden alfabético").clicked() {
            state.toggle_artist_sort_order();
        }
    });
    ui.add_space(8.0);

    let mut selected: Option<String> = None;

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            for group in &groups {
                ui.allocate_ui(egui::vec2(200.0, 160.0), |ui| {
                    egui::Frame::none()
                        .fill(theme::BG_CARD)
                        .rounding(egui::Rounding::same(16.0))
                        .inner_margin(16.0)
                        .show(ui, |ui| {
                            ui.set_width(168.0);
                            ui.vertical_centered(|ui| {
                                super::cover_thumb(ui, textures, &group.representative_cover, 80.0, 12.0, "🎤");
                                ui.add_space(6.0);
                                ui.label(egui::RichText::new(&group.artist).color(theme::TEXT_MAIN).strong());
                                let label = if group.count == 1 { "canción" } else { "canciones" };
                                ui.label(
                                    egui::RichText::new(format!("{} {}", group.count, label))
                                        .color(theme::TEXT_MUTED)
                                        .size(11.0),
                                );
                                ui.add_space(6.0);
                                if ui.button("Ver").clicked() {
                                    selected = Some(group.artist.clone());
                                }
                            });
                        });
                });
            }
        });
    });

    if let Some(artist) = selected {
        state.selected_artist = Some(artist);
    }
}
