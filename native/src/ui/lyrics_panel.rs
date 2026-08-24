use crate::lyrics::Lyrics;
use crate::theme;
use eframe::egui;

/// Dibuja las líneas de letra y hace auto-scroll a la línea activa. Solo
/// dispara el scroll el frame en que `active_index` cambia (comparando con
/// `last_active`), para no pelear con el scroll manual del usuario.
pub fn show(
    ui: &mut egui::Ui,
    lyrics: &Option<Lyrics>,
    active_index: Option<usize>,
    last_active: &mut Option<usize>,
) {
    let lines: Vec<(String, bool)> = match lyrics {
        Some(Lyrics::Synced(v)) => v
            .iter()
            .enumerate()
            .map(|(i, l)| (l.text.clone(), Some(i) == active_index))
            .collect(),
        Some(Lyrics::Plain(v)) => v.iter().map(|t| (t.clone(), false)).collect(),
        None => Vec::new(),
    };

    if lines.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.label(egui::RichText::new("No hay letra disponible para esta canción").color(theme::TEXT_MUTED));
        });
        *last_active = active_index;
        return;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .id_salt("lyrics_scroll")
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.35);
                for (text, is_active) in &lines {
                    let rich = egui::RichText::new(text)
                        .size(if *is_active { 19.0 } else { 16.0 })
                        .color(if *is_active { theme::TEXT_MAIN } else { theme::TEXT_MUTED });
                    let response = ui.label(if *is_active { rich.strong() } else { rich });

                    if *is_active && *last_active != active_index {
                        response.scroll_to_me(Some(egui::Align::Center));
                    }
                }
                ui.add_space(ui.available_height() * 0.35);
            });
        });

    *last_active = active_index;
}
