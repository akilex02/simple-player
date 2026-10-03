use crate::state::AppState;
use crate::theme::{self, icons, with_alpha};
use crate::ui::widgets::icon_button::IconButton;
use eframe::egui::{self, RichText};

pub const SEARCH_ID: &str = "search_box";

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    if state.active_tab == crate::state::ActiveTab::Stats {
        ui.add_space(8.0); // el buscador no aplica en Estadísticas
        return;
    }
    ui.horizontal(|ui| {
        let in_detail = state.selected_artist.is_some() || state.selected_album.is_some();
        if in_detail && IconButton::new(icons::CARET_LEFT, 38.0).tooltip("Volver").show(ui).clicked() {
            state.selected_artist = None;
            state.selected_album = None;
        }

        search_box(ui, state);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if state.loading {
                ui.label(RichText::new("Escaneando…").color(theme::TEXT_MUTED));
                ui.spinner();
            } else {
                ui.label(RichText::new(format!("{} canciones", state.songs.len())).size(theme::text::SM).color(theme::TEXT_MUTED));
            }
        });
    });
}

fn search_box(ui: &mut egui::Ui, state: &mut AppState) {
    let id = egui::Id::new(SEARCH_ID);
    let width = ui.available_width().min(460.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 40.0), egui::Sense::hover());

    if state.focus_search {
        ui.memory_mut(|m| m.request_focus(id));
        state.focus_search = false;
    }
    let focused = ui.memory(|m| m.has_focus(id));
    let accent = theme::accent(ui.ctx());
    let focus = ui.ctx().animate_bool_with_time(id.with("focus"), focused, theme::motion::HOVER);

    let painter = ui.painter();
    let rounding = rect.height() / 2.0;
    painter.rect_filled(rect, rounding, egui::Color32::from_white_alpha((16.0 + 10.0 * focus) as u8));
    painter.rect_stroke(rect, rounding, egui::Stroke::new(1.0_f32, theme::lerp_color(theme::GLASS_BORDER, with_alpha(accent, 200), focus)));
    painter.text(
        egui::pos2(rect.left() + 20.0, rect.center().y),
        egui::Align2::CENTER_CENTER,
        icons::MAGNIFYING_GLASS,
        egui::FontId::proportional(theme::text::LG),
        theme::TEXT_MUTED,
    );

    let clear_w = if state.search_query.is_empty() { 0.0 } else { 30.0 };
    let edit_rect = egui::Rect::from_min_max(
        egui::pos2(rect.left() + 40.0, rect.top() + 4.0),
        egui::pos2(rect.right() - 12.0 - clear_w, rect.bottom() - 4.0),
    );
    ui.put(
        edit_rect,
        egui::TextEdit::singleline(&mut state.search_query)
            .id(id)
            .frame(false)
            .hint_text("Buscar canciones, artistas…  ( / )")
            .font(egui::FontId::proportional(theme::text::MD))
            .desired_width(edit_rect.width())
            .vertical_align(egui::Align::Center),
    );

    if !state.search_query.is_empty() {
        let clear = egui::Rect::from_center_size(egui::pos2(rect.right() - 20.0, rect.center().y), egui::vec2(24.0, 24.0));
        let response = ui.interact(clear, id.with("clear"), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
        let color = if response.hovered() { theme::TEXT_MAIN } else { theme::TEXT_MUTED };
        ui.painter().text(clear.center(), egui::Align2::CENTER_CENTER, icons::X, egui::FontId::proportional(theme::text::MD), color);
        if response.clicked() {
            state.search_query.clear();
        }
    }
}
