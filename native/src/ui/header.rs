use crate::state::AppState;
use eframe::egui;

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut state.search_query)
                .hint_text("Buscar canciones, artistas...")
                .desired_width(360.0),
        );

        if !state.search_query.is_empty() && ui.button("✕").clicked() {
            state.search_query.clear();
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let label = if state.loading {
                "Escaneando..."
            } else if state.current_folder_path.is_some() {
                "Cambiar carpeta..."
            } else {
                "Abrir carpeta de música"
            };
            if ui.add_enabled(!state.loading, egui::Button::new(label)).clicked() {
                state.select_folder_and_scan();
            }
        });
    });
}
