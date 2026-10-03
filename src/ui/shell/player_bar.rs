use crate::state::AppState;
use crate::theme::{self, icons, space, text};
use crate::ui::textures::TextureCache;
use crate::ui::widgets::cover::cover;
use crate::ui::widgets::icon_button::IconButton;
use crate::ui::{progress, transport, visualizers, volume, Size};
use crate::viz::VizFrame;
use eframe::egui::{self, RichText};

pub const HEIGHT: f32 = 96.0;
const VOLUME_GROUP_W: f32 = 140.0;
const CENTER_MIN_W: f32 = 300.0;
/// Ancho que debe conservar la pista actual para que se sume el mini visualizador.
const LEFT_COMFY_W: f32 = 240.0;
const VIZ_W: f32 = 120.0;
/// Tres botones de 34 px, sus separaciones y el grupo de volumen.
const RIGHT_W: f32 = 3.0 * 34.0 + 3.0 * 8.0 + space::SM + VOLUME_GROUP_W;

/// Barra inferior de tres columnas: pista actual · transporte y progreso ·
/// volumen, cola, letras y pantalla completa (con mini visualizador).
pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache, viz: &VizFrame) {
    let full = ui.max_rect().shrink2(egui::vec2(space::XL, 0.0));
    // El centro y la derecha tienen ancho propio (lo que necesitan); la pista actual toma el resto
    // y trunca su texto, así nada se encima aunque la ventana sea angosta.
    let center_w = (full.width() * 0.4).clamp(CENTER_MIN_W, 460.0);
    let spare = full.width() - center_w - RIGHT_W - 2.0 * space::LG - LEFT_COMFY_W;
    let show_viz = spare >= VIZ_W + space::MD;
    let right_w = if show_viz { RIGHT_W + space::MD + VIZ_W } else { RIGHT_W };
    let side = ((full.width() - center_w - 2.0 * space::LG) / 2.0).max(right_w);
    let right = egui::Rect::from_min_max(egui::pos2(full.right() - side, full.top()), full.max);
    let left = egui::Rect::from_min_max(full.min, egui::pos2(full.right() - side - center_w - 2.0 * space::LG, full.bottom()));
    let center = egui::Rect::from_min_max(egui::pos2(left.right() + space::LG, full.top()), egui::pos2(right.left() - space::LG, full.bottom()));

    let mut col = |rect: egui::Rect, layout: egui::Layout| ui.new_child(egui::UiBuilder::new().max_rect(rect).layout(layout));
    left_column(&mut col(left, egui::Layout::left_to_right(egui::Align::Center)), state, textures);
    center_column(&mut col(center, egui::Layout::top_down(egui::Align::Center)), state);
    right_column(&mut col(right, egui::Layout::right_to_left(egui::Align::Center)), state, viz, show_viz);
}

fn left_column(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    let cover_path = state.current_song().and_then(|s| s.cover_art.clone());
    let response = cover(ui, textures, &cover_path, 60.0, theme::radius::SM, icons::MUSIC_NOTES, true);
    ui.add_space(space::MD);

    let info = state.current_song().map(|s| (s.title.clone(), s.artist.clone()));
    let width = ui.available_width();
    ui.allocate_ui_with_layout(egui::vec2(width, 60.0), egui::Layout::top_down(egui::Align::Min), |ui| match info {
        Some((title, artist)) => {
            ui.add_space(10.0);
            ui.add(egui::Label::new(RichText::new(title).font(theme::bold(text::BASE))).truncate());
            let artist = if artist.trim().is_empty() { "Artista desconocido".to_string() } else { artist };
            ui.add(egui::Label::new(RichText::new(artist).size(text::SM).color(theme::accent(ui.ctx()))).truncate());
        }
        None => {
            ui.add_space(20.0);
            ui.label(RichText::new("Nada reproduciéndose").color(theme::TEXT_MUTED));
        }
    });
    if state.current_song().is_some()
        && response.interact(egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
    {
        state.open_fullscreen();
    }
}

fn center_column(ui: &mut egui::Ui, state: &mut AppState) {
    ui.add_space(10.0);
    transport::show(ui, state, Size::Compact);
    ui.add_space(2.0);
    progress::show(ui, state, Size::Compact);
}

fn right_column(ui: &mut egui::Ui, state: &mut AppState, viz: &VizFrame, show_viz: bool) {
    if IconButton::new(icons::ARROWS_OUT_SIMPLE, 34.0).tooltip("Pantalla completa").show(ui).clicked() {
        state.open_fullscreen();
    }
    let lyrics_active = state.show_lyrics && state.is_fullscreen;
    if IconButton::new(icons::TEXT_ALIGN_LEFT, 34.0).active(lyrics_active).tooltip("Letra").show(ui).clicked() {
        state.open_lyrics();
    }
    if IconButton::new(icons::QUEUE, 34.0).active(state.show_queue).tooltip("Cola").show(ui).clicked() {
        state.show_queue = !state.show_queue;
    }
    ui.add_space(space::SM);

    // right_to_left: lo que se agrega después queda más a la izquierda. El
    // volumen va en un bloque de ancho fijo con su propio orden de izquierda a derecha.
    ui.allocate_ui_with_layout(egui::vec2(VOLUME_GROUP_W, 34.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        volume::show(ui, state, Size::Compact);
    });

    if show_viz {
        ui.add_space(space::MD);
        let (_, rect) = ui.allocate_space(egui::vec2(120.0, 36.0));
        visualizers::bars::draw(&ui.painter_at(rect), rect, viz);
    }
}
