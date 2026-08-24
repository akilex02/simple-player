use crate::state::{ActiveTab, AppState};
use crate::theme;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(28.0);
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("SIMPLE PLAYER")
                .font(egui::FontId::new(18.0, egui::FontFamily::Name(theme::FONT_CONDENSED.into())))
                .color(theme::TEXT_MAIN)
                .strong(),
        );
    });

    ui.add_space(32.0);
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new("NAVEGACIÓN")
                .color(theme::ACCENT_PINK)
                .size(11.0)
                .strong(),
        );
    });
    ui.add_space(10.0);

    nav_item(ui, state, "Toda la Música", ActiveTab::All);
    nav_item(ui, state, "Artistas", ActiveTab::Artists);
}

fn nav_item(ui: &mut egui::Ui, state: &mut AppState, label: &str, tab: ActiveTab) {
    let is_active = state.active_tab == tab;
    let (bg, text_color) = if is_active {
        (theme::ACCENT_LIME, egui::Color32::from_rgb(0x13, 0x13, 0x22))
    } else {
        (egui::Color32::TRANSPARENT, theme::TEXT_MUTED)
    };

    let button = egui::Button::new(egui::RichText::new(label).color(text_color))
        .fill(bg)
        .rounding(egui::Rounding::same(9999.0))
        .min_size(egui::vec2(ui.available_width(), 38.0));

    if ui.add(button).clicked() {
        state.select_tab(tab);
    }
    ui.add_space(8.0);
}
