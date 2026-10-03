use super::backdrop::Backdrop;
use super::lyrics_view::LyricsView;
use super::textures::TextureCache;
use super::visualizers::{self, VisualizerMode};
use super::widgets::chip::chip;
use super::widgets::cover::paint_cover;
use super::widgets::icon_button::IconButton;
use super::{progress, transport, volume, Size};
use crate::state::AppState;
use crate::theme::{self, icons, text};
use crate::viz::VizFrame;
use eframe::egui::{self, RichText};

const CONTROLS_H: f32 = 168.0;
const BOTTOM_MARGIN: f32 = 32.0;
const STRIP_H: f32 = 84.0;

/// Pantalla completa "Ahora suena": un overlay que aparece y desaparece con
/// fundido sobre la app. Portada grande, fondo desenfocado, visualizador como
/// capa (o franja) y letras estilo Apple Music a la derecha.
pub struct FullscreenView {
    pub mode: VisualizerMode,
    lyrics: LyricsView,
}

impl Default for FullscreenView {
    fn default() -> Self {
        Self { mode: VisualizerMode::Bars, lyrics: LyricsView::default() }
    }
}

impl FullscreenView {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        state: &mut AppState,
        textures: &mut TextureCache,
        backdrop: &Backdrop,
        viz: &VizFrame,
    ) {
        let anim = ctx.animate_bool_with_time(egui::Id::new("fullscreen_anim"), state.is_fullscreen, theme::motion::SCREEN);
        if anim <= 0.0 {
            return;
        }
        let rect = ctx.screen_rect();
        egui::Area::new(egui::Id::new("fullscreen_overlay"))
            .order(egui::Order::Foreground)
            .fixed_pos(rect.min)
            .constrain(false)
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_opacity(anim);
                ui.set_clip_rect(rect);
                // Reclama toda la ventana para que nada de abajo reciba el mouse.
                ui.allocate_exact_size(rect.size(), egui::Sense::click_and_drag());

                let painter = ui.painter().clone();
                painter.rect_filled(rect, 0.0, theme::BG_BASE);
                backdrop.paint(textures, &painter, rect, ctx.input(|i| i.time) as f32);
                painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(70));

                self.paint_visualizer(ui, rect, viz, anim);
                self.content(ui, rect, state, textures, anim);
            });
    }

    fn paint_visualizer(&self, ui: &mut egui::Ui, rect: egui::Rect, viz: &VizFrame, anim: f32) {
        let (area, opacity) = match self.mode {
            VisualizerMode::Off => return,
            VisualizerMode::Strip => {
                let bottom = rect.bottom() - BOTTOM_MARGIN - CONTROLS_H - 6.0;
                let band = egui::Rect::from_min_max(
                    egui::pos2(rect.left() + 56.0, bottom - STRIP_H),
                    egui::pos2(rect.right() - 56.0, bottom),
                );
                (band, 0.75)
            }
            _ => (rect, 0.4),
        };
        let mut layer = ui.new_child(egui::UiBuilder::new().max_rect(area));
        layer.set_opacity(anim * opacity);
        visualizers::draw(&layer, area, viz, self.mode);
    }

    fn content(&mut self, ui: &mut egui::Ui, rect: egui::Rect, state: &mut AppState, textures: &mut TextureCache, anim: f32) {
        let ctx = ui.ctx().clone();
        let accent = theme::accent(&ctx);
        let inner = rect.shrink2(egui::vec2(56.0, BOTTOM_MARGIN));
        let top = egui::Rect::from_min_size(inner.min, egui::vec2(inner.width(), 44.0));
        let controls = egui::Rect::from_min_max(egui::pos2(inner.left(), inner.bottom() - CONTROLS_H), inner.max);
        // La franja del visualizador ocupa su propio espacio sobre los controles.
        let strip_space = if self.mode == VisualizerMode::Strip { STRIP_H + 12.0 } else { 0.0 };
        let body = egui::Rect::from_min_max(
            egui::pos2(inner.left(), top.bottom() + 8.0),
            egui::pos2(inner.right(), controls.top() - 8.0 - strip_space),
        );

        let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(top).layout(egui::Layout::left_to_right(egui::Align::Center)));
        bar.label(RichText::new("REPRODUCIENDO AHORA").font(theme::bold(text::XS)).color(accent));
        bar.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if IconButton::new(icons::X, 40.0).tooltip("Cerrar (Esc)").show(ui).clicked() {
                state.close_fullscreen();
            }
            ui.add_space(8.0);
            if IconButton::new(icons::TEXT_ALIGN_LEFT, 40.0).active(state.show_lyrics).tooltip("Letra").show(ui).clicked() {
                state.toggle_lyrics_visibility();
            }
            ui.add_space(8.0);
            if chip(ui, self.mode.label(), self.mode != VisualizerMode::Off).on_hover_text("Cambiar visualizador").clicked() {
                self.mode = self.mode.next();
            }
            ui.label(RichText::new(icons::WAVEFORM).size(text::LG).color(theme::TEXT_MUTED));
        });

        let song = state.current_song().cloned();
        let lyrics_anim = ctx.animate_bool_with_time(egui::Id::new("fullscreen_lyrics"), state.show_lyrics, 0.3);
        let left_w = body.width() + (body.width() * 0.42 - body.width()) * lyrics_anim;
        let left = egui::Rect::from_min_size(body.min, egui::vec2(left_w, body.height()));

        let cover_size = (left_w * 0.78).min(body.height() * 0.62).clamp(180.0, 460.0);
        let block_h = cover_size + 24.0 + 96.0;
        let top_y = (left.center().y - block_h / 2.0).max(left.top());
        let cover_rect = egui::Rect::from_min_size(egui::pos2(left.center().x - cover_size / 2.0, top_y), egui::vec2(cover_size, cover_size));
        let cover_path = song.as_ref().and_then(|s| s.cover_art.clone());
        paint_cover(ui, cover_rect, textures, &cover_path, 20.0, icons::MUSIC_NOTES, true);

        let info_rect = egui::Rect::from_min_max(egui::pos2(left.left(), cover_rect.bottom() + 24.0), egui::pos2(left.right(), cover_rect.bottom() + 120.0));
        let mut info = ui.new_child(egui::UiBuilder::new().max_rect(info_rect).layout(egui::Layout::top_down(egui::Align::Center)));
        match &song {
            Some(song) => {
                info.add(egui::Label::new(RichText::new(&song.title).font(theme::bold(text::XXL)).color(theme::TEXT_MAIN)).truncate());
                let artist = if song.artist.trim().is_empty() { "Artista desconocido" } else { &song.artist };
                info.add(egui::Label::new(RichText::new(artist).font(theme::bold(text::MD)).color(accent)).truncate());
                info.add(egui::Label::new(RichText::new(&song.album).color(theme::TEXT_MUTED)).truncate());
            }
            None => {
                info.label(RichText::new("Nada reproduciéndose").color(theme::TEXT_MUTED));
            }
        }

        if lyrics_anim > 0.01 {
            let lyrics_rect = egui::Rect::from_min_max(egui::pos2(left.right() + 56.0, body.top()), body.max);
            let mut layer = ui.new_child(egui::UiBuilder::new().max_rect(lyrics_rect));
            layer.set_opacity(anim * lyrics_anim);
            let active = state.active_lyric_line_index();
            let track = song.as_ref().map(|s| s.path.clone()).unwrap_or_default();
            if let Some(secs) = self.lyrics.show(&mut layer, lyrics_rect, &state.lyrics, active, &track) {
                state.seek_commit(secs);
            }
        }

        let center_w = (inner.width() - 460.0).clamp(320.0, 760.0);
        let center = egui::Rect::from_center_size(egui::pos2(inner.center().x, controls.center().y), egui::vec2(center_w, controls.height()));
        // Velo: sube desde transparente hasta oscuro para que los controles se lean sobre las barras.
        let veil = egui::Rect::from_min_max(egui::pos2(rect.left(), controls.top() - 60.0), rect.max);
        let (clear, dark) = (egui::Color32::TRANSPARENT, egui::Color32::from_black_alpha(150));
        let mut mesh = egui::Mesh::default();
        let v = |pos: egui::Pos2, color| egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color };
        mesh.vertices.extend([
            v(veil.left_top(), clear),
            v(veil.right_top(), clear),
            v(veil.right_bottom(), dark),
            v(veil.left_bottom(), dark),
        ]);
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        ui.painter().add(egui::Shape::mesh(mesh));

        let mut main_controls = ui.new_child(egui::UiBuilder::new().max_rect(center).layout(egui::Layout::top_down(egui::Align::Center)));
        main_controls.add_space(20.0);
        progress::show(&mut main_controls, state, Size::Large);
        main_controls.add_space(10.0);
        transport::show(&mut main_controls, state, Size::Large);

        let vol = egui::Rect::from_min_size(egui::pos2(inner.right() - 250.0, controls.center().y - 20.0), egui::vec2(250.0, 40.0));
        let mut volume_ui = ui.new_child(egui::UiBuilder::new().max_rect(vol).layout(egui::Layout::right_to_left(egui::Align::Center)));
        volume_ui.allocate_ui_with_layout(egui::vec2(216.0, 34.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            volume::show(ui, state, Size::Compact);
        });
    }
}
