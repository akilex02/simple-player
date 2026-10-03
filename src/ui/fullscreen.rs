use super::backdrop::Backdrop;
use super::lyrics_view::LyricsView;
use super::textures::TextureCache;
use super::visualizers::particles as particle_draw;
use super::visualizers::{self, VisualizerMode};
use super::widgets::chip::chip;
use super::widgets::cover::paint_cover;
use super::widgets::glass::{paint_glass, GlassKind};
use super::widgets::icon_button::IconButton;
use super::{progress, transport, volume, Size};
use crate::state::AppState;
use crate::theme::{self, icons, text, with_alpha};
use crate::viz::bands::{BandAnalyzer, Bands};
use crate::viz::particles::{Anchor, ParticleField, Pull, Rect2, V2};
use crate::viz::VizFrame;
use eframe::egui::{self, RichText};

const CONTROLS_H: f32 = 124.0;
const BOTTOM_MARGIN: f32 = 28.0;
const STRIP_H: f32 = 84.0;
/// Margen sobre la fila de botones y alto del botón de reproducir (`Size::Large`).
const TRANSPORT_TOP: f32 = 16.0;
const TRANSPORT_H: f32 = 64.0;
/// Alto de la fila de la línea de tiempo y separación con los botones.
const PROGRESS_H: f32 = 22.0;
const CONTROLS_GAP: f32 = 2.0;
/// Margen interno del panel que agrupa los controles principales.
const PANEL_PAD_X: f32 = 14.0;
const PANEL_PAD_Y: f32 = 8.0;

struct Geometry {
    inner: egui::Rect,
    top: egui::Rect,
    controls: egui::Rect,
    body: egui::Rect,
    left: egui::Rect,
    cover_rect: egui::Rect,
    lyrics_anim: f32,
}

/// Pantalla completa "Ahora suena": un overlay que aparece y desaparece con
/// fundido sobre la app. Portada grande, fondo desenfocado, visualizador como
/// capa (o franja) y letras estilo Apple Music a la derecha.
pub struct FullscreenView {
    pub mode: VisualizerMode,
    lyrics: LyricsView,
    bands: BandAnalyzer,
    bands_now: Bands,
    /// Campo de partículas del modo actual (se recrea al cambiar de modo).
    field: Option<(VisualizerMode, ParticleField)>,
    /// `true` atrae al cursor; `G` alterna con repeler.
    attract: bool,
    mesh_vertices: usize,
}

impl Default for FullscreenView {
    fn default() -> Self {
        Self {
            mode: VisualizerMode::Bars,
            lyrics: LyricsView::default(),
            bands: BandAnalyzer::new(),
            bands_now: Bands::default(),
            field: None,
            attract: true,
            mesh_vertices: 0,
        }
    }
}

/// Semilla fija: la disposición inicial es la misma en cada arranque.
const FIELD_SEED: u64 = 0x5EED_F1E1D;

/// ¿El puntero está sobre algo con lo que se interactúa o que no debe "atraer"?
fn over_control(pos: egui::Pos2, blockers: &[egui::Rect]) -> bool {
    blockers.iter().any(|r| r.contains(pos))
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
                let (_, background) = ui.allocate_exact_size(rect.size(), egui::Sense::click_and_drag());

                let painter = ui.painter().clone();
                painter.rect_filled(rect, 0.0, theme::BG_BASE);
                backdrop.paint(textures, &painter, rect, ctx.input(|i| i.time) as f32);
                painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(70));

                let geo = self.geometry(ctx, rect, state.show_lyrics);
                let pull = self.pull_from_input(ctx, &background, &geo, state.show_lyrics);
                self.update_field(ctx, rect, &geo, viz, state.is_playing, pull);
                self.paint_visualizer(ui, rect, viz, anim, pull);
                self.content(ui, rect, state, textures, anim);
            });
    }

    /// Zonas de la pantalla; las comparten el contenido y el visualizador (el radial se centra en la portada).
    fn geometry(&self, ctx: &egui::Context, rect: egui::Rect, show_lyrics: bool) -> Geometry {
        let inner = rect.shrink2(egui::vec2(56.0, BOTTOM_MARGIN));
        let top = egui::Rect::from_min_size(inner.min, egui::vec2(inner.width(), 44.0));
        let controls = egui::Rect::from_min_max(egui::pos2(inner.left(), inner.bottom() - CONTROLS_H), inner.max);
        // La franja del visualizador ocupa su propio espacio sobre los controles.
        let strip_space = if self.mode == VisualizerMode::Strip { STRIP_H + 12.0 } else { 0.0 };
        let body = egui::Rect::from_min_max(
            egui::pos2(inner.left(), top.bottom() + 8.0),
            egui::pos2(inner.right(), controls.top() - 8.0 - strip_space),
        );

        let lyrics_anim = ctx.animate_bool_with_time(egui::Id::new("fullscreen_lyrics"), show_lyrics, 0.3);
        let left_w = body.width() + (body.width() * 0.42 - body.width()) * lyrics_anim;
        let left = egui::Rect::from_min_size(body.min, egui::vec2(left_w, body.height()));

        let cover_size = (left_w * 0.78).min(body.height() * 0.62).clamp(180.0, 460.0);
        let block_h = cover_size + 24.0 + 96.0;
        let top_y = (left.center().y - block_h / 2.0).max(left.top());
        let cover_rect = egui::Rect::from_min_size(egui::pos2(left.center().x - cover_size / 2.0, top_y), egui::vec2(cover_size, cover_size));
        Geometry { inner, top, controls, body, left, cover_rect, lyrics_anim }
    }

    pub fn mesh_vertices(&self) -> usize {
        self.mesh_vertices
    }

    /// Clic izquierdo mantenido sobre el fondo; no cuenta sobre widgets (ya capturan el clic)
    /// ni sobre portada, información, controles, encabezado o letras.
    fn pull_from_input(&self, ctx: &egui::Context, background: &egui::Response, geo: &Geometry, show_lyrics: bool) -> Option<Pull> {
        self.mode.field_kind()?;
        let (down, pos) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
        let pos = pos?;
        if !down || !background.is_pointer_button_down_on() {
            return None;
        }
        let info = egui::Rect::from_min_max(
            egui::pos2(geo.left.left(), geo.cover_rect.bottom() + 24.0),
            egui::pos2(geo.left.right(), geo.cover_rect.bottom() + 120.0),
        );
        let lyrics = egui::Rect::from_min_max(egui::pos2(geo.left.right() + 56.0, geo.body.top()), geo.body.max);
        let mut blockers = vec![geo.cover_rect, info, geo.controls, geo.top];
        if show_lyrics || geo.lyrics_anim > 0.01 {
            blockers.push(lyrics);
        }
        if over_control(pos, &blockers) {
            return None;
        }
        Some(Pull { pos: V2::new(pos.x, pos.y), attract: self.attract })
    }

    /// Bandas, creación/redimensión del campo y un paso de simulación (solo si suena).
    fn update_field(&mut self, ctx: &egui::Context, rect: egui::Rect, geo: &Geometry, viz: &VizFrame, playing: bool, pull: Option<Pull>) {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        self.bands_now = self.bands.update(&viz.bars, dt);
        if ctx.input(|i| i.key_pressed(egui::Key::G)) {
            self.attract = !self.attract;
        }
        let Some(kind) = self.mode.field_kind() else {
            self.field = None;
            self.mesh_vertices = 0;
            return;
        };
        let bounds = Rect2::new(rect.left(), rect.top(), rect.right(), rect.bottom());
        match &mut self.field {
            Some((mode, field)) if *mode == self.mode => {
                if field.bounds() != bounds {
                    field.resize(bounds);
                }
            }
            _ => {
                self.field = Some((self.mode, ParticleField::new(kind, particle_draw::count_for(self.mode), bounds, FIELD_SEED)));
            }
        }
        if playing {
            let anchor = Anchor {
                center: V2::new(geo.cover_rect.center().x, geo.cover_rect.center().y),
                radius: geo.cover_rect.width() * 0.5,
            };
            let time = ctx.input(|i| i.time) as f32;
            if let Some((_, field)) = &mut self.field {
                field.step(dt, self.bands_now, anchor, pull, time);
            }
        }
    }

    fn paint_visualizer(&mut self, ui: &mut egui::Ui, rect: egui::Rect, viz: &VizFrame, anim: f32, pull: Option<Pull>) {
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
        let Some((_, field)) = self.field.as_ref().filter(|_| self.mode.field_kind().is_some()) else {
            visualizers::draw(&layer, area, viz, self.mode);
            return;
        };
        let accent = theme::accent(ui.ctx());
        let mesh = particle_draw::mesh_for(self.mode, field, accent, self.bands_now.bass);
        self.mesh_vertices = mesh.vertices.len();
        let painter = layer.painter_at(area);
        painter.add(egui::Shape::mesh(mesh));
        if let Some(pull) = pull {
            // Indicador del cursor: verde atrae, rojo repele; late con los graves.
            let color = if pull.attract { egui::Color32::from_rgb(0, 255, 128) } else { egui::Color32::from_rgb(255, 40, 40) };
            let radius = 36.0 + self.bands_now.bass * 30.0;
            painter.circle_filled(egui::pos2(pull.pos.x, pull.pos.y), radius, color.gamma_multiply(0.18));
            painter.circle_filled(egui::pos2(pull.pos.x, pull.pos.y), 3.0, color);
        }
    }

    fn content(&mut self, ui: &mut egui::Ui, rect: egui::Rect, state: &mut AppState, textures: &mut TextureCache, anim: f32) {
        let ctx = ui.ctx().clone();
        let accent = theme::accent(&ctx);
        let Geometry { inner, top, controls, body, left, cover_rect, lyrics_anim } = self.geometry(&ctx, rect, state.show_lyrics);

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
            let hint = if self.mode.field_kind().is_some() { "Cambiar visualizador · clic: atraer · G: atraer/repeler" } else { "Cambiar visualizador" };
            if chip(ui, self.mode.label(), self.mode != VisualizerMode::Off).on_hover_text(hint).clicked() {
                self.mode = self.mode.next();
            }
            ui.label(RichText::new(icons::WAVEFORM).size(text::LG).color(theme::TEXT_MUTED));
        });

        let song = state.current_song().cloned();
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

        let center_w = (inner.width() - 460.0).clamp(320.0, 360.0);
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

        // Panel de vidrio (como el encabezado de la biblioteca) que agrupa botones y línea de tiempo.
        let panel = egui::Rect::from_min_max(
            egui::pos2(center.left() - PANEL_PAD_X, center.top() + TRANSPORT_TOP - PANEL_PAD_Y),
            egui::pos2(
                center.right() + PANEL_PAD_X,
                center.top() + TRANSPORT_TOP + TRANSPORT_H + CONTROLS_GAP + PROGRESS_H + PANEL_PAD_Y,
            ),
        );
        paint_glass(ui.painter(), panel, GlassKind::Strong, theme::radius::XL, 0.0);
        ui.painter().rect_filled(panel, theme::radius::XL, with_alpha(accent, 30));

        let mut main_controls = ui.new_child(egui::UiBuilder::new().max_rect(center).layout(egui::Layout::top_down(egui::Align::Center)));
        // Sin espacio automático entre filas: el alto del panel se calcula con las constantes de arriba.
        main_controls.spacing_mut().item_spacing.y = 0.0;
        // Mismo orden que la barra inferior: botones arriba, línea de tiempo debajo.
        main_controls.add_space(TRANSPORT_TOP);
        transport::show(&mut main_controls, state, Size::Large);
        main_controls.add_space(CONTROLS_GAP);
        progress::show(&mut main_controls, state, Size::Large);

        // El volumen se alinea con la fila de botones (su centro vertical).
        let transport_center_y = center.top() + TRANSPORT_TOP + TRANSPORT_H / 2.0;
        let vol = egui::Rect::from_center_size(egui::pos2(inner.right() - 125.0, transport_center_y), egui::vec2(250.0, 40.0));
        let mut volume_ui = ui.new_child(egui::UiBuilder::new().max_rect(vol).layout(egui::Layout::right_to_left(egui::Align::Center)));
        volume_ui.allocate_ui_with_layout(egui::vec2(140.0, 34.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            volume::show(ui, state, Size::Compact);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_puntero_sobre_una_zona_bloqueada_no_atrae() {
        let cover = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(200.0, 200.0));
        let controls = egui::Rect::from_min_size(egui::pos2(0.0, 600.0), egui::vec2(800.0, 100.0));
        let blockers = [cover, controls];
        assert!(over_control(egui::pos2(150.0, 150.0), &blockers));
        assert!(over_control(egui::pos2(400.0, 650.0), &blockers));
        assert!(!over_control(egui::pos2(500.0, 300.0), &blockers));
        assert!(!over_control(egui::pos2(500.0, 300.0), &[]));
    }
}
