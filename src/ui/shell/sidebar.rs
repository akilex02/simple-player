use crate::state::{ActiveTab, AppState};
use crate::theme::{self, icons, space, text};
use crate::ui::widgets::nav_item::nav_item;
use crate::ui::widgets::pill_button::{PillButton, PillKind};
use eframe::egui::{self, RichText};

/// Lado del logo de la app (SVG) junto al nombre.
const LOGO_SIZE: f32 = 40.0;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {

    ui.horizontal(|ui| {
        ui.add(
            egui::Image::new(egui::include_image!("../../../assets/icons/SimplePlayer.svg"))
                .fit_to_exact_size(egui::vec2(LOGO_SIZE, LOGO_SIZE)),
        );
        ui.label(RichText::new("SIMPLE PLAYER").font(theme::deco(text::XL)).color(theme::TEXT_MAIN));
    });

    ui.add_space(space::XXL);
    section_label(ui, "BIBLIOTECA");
    for (icon, label, tab) in [
        (icons::MUSIC_NOTES, "Canciones", ActiveTab::All),
        (icons::DISC, "Álbumes", ActiveTab::Albums),
        (icons::USER, "Artistas", ActiveTab::Artists),
        (icons::CHART_BAR, "Estadísticas", ActiveTab::Stats),
    ] {
        if nav_item(ui, icon, label, state.active_tab == tab).clicked() {
            state.select_tab(tab);
        }
        ui.add_space(space::XS);
    }

    ui.add_space(space::XL);
    section_label(ui, "CARPETA");
    let summary = match state.settings.music_folders.len() {
        0 => "Sin carpetas".to_string(),
        1 => "1 carpeta".to_string(),
        n => format!("{n} carpetas"),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(icons::FOLDER_OPEN).size(text::LG).color(theme::TEXT_MUTED));
        ui.label(RichText::new(summary).color(theme::TEXT_MAIN));
    });
    ui.add_space(space::SM);
    ui.add_enabled_ui(!state.loading, |ui| {
        if PillButton::new("Agregar carpeta", PillKind::Secondary).icon(icons::FOLDER_OPEN).show(ui).clicked() {
            state.add_folder_via_dialog();
        }
    });
}

fn section_label(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label).font(theme::bold(text::XS)).color(theme::TEXT_MUTED));
    ui.add_space(space::SM);
}
