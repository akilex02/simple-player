mod audio;
mod events;
mod hotkeys;
mod library;
mod library_view;
mod lyrics;
mod mpris;
mod paths;
mod perf;
mod persistence;
mod repaint;
mod state;
mod theme;
mod ui;
mod viz;

use eframe::egui;
use events::AppEvent;
use gstreamer::prelude::*;
use state::{ActiveTab, AppState};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use ui::textures::TextureCache;
use viz::{engine::VizEngine, VizFrame};

struct App {
    state: AppState,
    _hotkeys: Option<hotkeys::Hotkeys>,
    rx: mpsc::Receiver<AppEvent>,
    spectrum_ring: std::sync::Arc<audio::spectrum::SpectrumRing>,
    viz_engine: VizEngine,
    viz: VizFrame,
    clock: audio::clock::PlaybackClock,
    last_clock_sync: Instant,
    last_draw: Instant,
    viz_latency_secs: f64,
    fullscreen: ui::fullscreen::FullscreenView,
    textures: TextureCache,
    perf: perf::PerfHud,
    gallery: Option<ui::gallery::Gallery>,
    backdrop: ui::backdrop::Backdrop,
    shot: Option<(String, u32)>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);

        let audio = audio::player::init();
        let (tx, rx) = mpsc::channel::<AppEvent>();
        let sender = events::EventSender::new(tx, cc.egui_ctx.clone());

        let spectrum_ring = std::sync::Arc::new(audio::spectrum::SpectrumRing::default());
        if let Ok(player) = audio.inner.lock() {
            if let Some(bus) = player.pipeline().bus() {
                audio::spectrum::install_spectrum_watch(&bus, std::sync::Arc::clone(&spectrum_ring));
            }
        }

        let mpris_tx = mpris::spawn_mpris_thread(std::sync::Arc::clone(&audio.inner), sender.clone());
        let hotkeys = match hotkeys::Hotkeys::register(sender) {
            Ok(h) => Some(h),
            Err(e) => {
                eprintln!("[aviso] Atajos globales desactivados: {e}");
                None
            }
        };

        let mut state = AppState::new(audio, mpris_tx);
        state.init();
        if let Some(tab) = ui::gallery::arg_value("--tab") {
            match tab.as_str() {
                "albums" => state.select_tab(ActiveTab::Albums),
                "artists" => state.select_tab(ActiveTab::Artists),
                _ => {}
            }
        }
        state.show_queue = std::env::args().any(|a| a == "--queue");
        if let Some(needle) = ui::gallery::arg_value("--song") {
            if let Some(song) = state.songs.iter().find(|s| s.path.contains(&needle)).cloned() {
                state.active_queue = vec![song];
                state.current_song_index = Some(0);
            }
        }
        if let Some(secs) = ui::gallery::arg_value("--time").and_then(|v| v.parse().ok()) {
            state.current_time = secs;
        }
        state.is_fullscreen = std::env::args().any(|a| a == "--fullscreen");
        state.show_lyrics = std::env::args().any(|a| a == "--lyrics");
        let gallery = std::env::args().any(|a| a == "--gallery").then(|| ui::gallery::Gallery::new(&state.songs));

        Self {
            state,
            _hotkeys: hotkeys,
            rx,
            spectrum_ring,
            viz_engine: VizEngine::new(),
            viz: VizFrame::default(),
            clock: audio::clock::PlaybackClock::new(Instant::now()),
            last_clock_sync: Instant::now(),
            last_draw: Instant::now(),
            viz_latency_secs: 0.0,
            fullscreen: ui::fullscreen::FullscreenView::default(),
            textures: TextureCache::default(),
            perf: perf::PerfHud::new(),
            gallery,
            backdrop: ui::backdrop::Backdrop::new(),
            shot: ui::gallery::arg_value("--shot").map(|path| (path, 0)),
        }
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::MediaPrev => self.state.handle_prev_song(),
                AppEvent::MediaNext => self.state.handle_next_song(),
                AppEvent::MediaPlayPause => self.state.toggle_play_pause(),
                AppEvent::MediaPlaying(playing) => self.state.is_playing = playing,
            }
        }
    }

    /// Mete los frames nuevos de espectro al motor y calcula las barras del
    /// instante de reproducción actual (reloj estimado menos la latencia ajustada).
    fn update_viz(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_draw).as_secs_f32().min(0.1);
        self.last_draw = now;

        let frames = self.spectrum_ring.try_drain();
        for frame in &frames {
            self.perf.on_spectrum(frame.arrived);
        }
        self.viz_engine.ingest(frames);

        let playing = self.state.is_playing;
        self.clock.set_playing(playing, now);
        if now.duration_since(self.last_clock_sync) >= Duration::from_millis(100) {
            if let Some(pos) = self.state.audio.try_position_secs() {
                self.clock.resync(pos, now);
            }
            self.last_clock_sync = now;
        }

        let clock_now = self.clock.now(now);
        self.viz = self.viz_engine.update(clock_now - self.viz_latency_secs, dt, playing).clone();
        if playing {
            if let Some(newest) = self.viz_engine.newest_time() {
                self.perf.record_spectrum_lead((newest - clock_now) * 1000.0);
            }
            self.perf.record_underrun(self.viz_engine.underrun());
        }
    }

    /// Flag de desarrollo `--shot <ruta>`: la app guarda su propia captura a
    /// los ~2 s y se cierra, para revisar el aspecto sin depender del escritorio.
    fn handle_dev_screenshot(&mut self, ctx: &egui::Context) {
        let Some((path, frames)) = &mut self.shot else { return };
        ctx.request_repaint();
        *frames += 1;
        if *frames == 120 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [w, h] = image.size;
            let rgba: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            if let Some(img) = image::RgbaImage::from_raw(w as u32, h as u32, rgba) {
                let _ = img.save(&*path);
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Coincide con los atajos globales de teclado de App.tsx: se ignoran
        // si el foco está en un campo de texto (egui ya no manda `Space` como
        // texto a un widget enfocado en ese caso, pero sí evitamos el bloqueo de flechas).
        let editing_text = ctx.wants_keyboard_input();
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            let window_fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!window_fullscreen));
        }

        ctx.input(|i| {
            if i.key_pressed(egui::Key::Space) && !editing_text {
                self.state.toggle_play_pause();
            }
            if i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight) {
                self.state.handle_next_song();
            }
            if i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft) {
                self.state.handle_prev_song();
            }
            if (i.key_pressed(egui::Key::Slash) && !editing_text) || (i.modifiers.command && i.key_pressed(egui::Key::K)) {
                self.state.focus_search = true;
            }
            if i.key_pressed(egui::Key::Escape) && self.state.is_fullscreen {
                self.state.close_fullscreen();
            }
            if i.key_pressed(egui::Key::F3) {
                self.perf.toggle();
            }
            if i.key_pressed(egui::Key::F4) {
                self.viz_latency_secs -= 0.010;
            }
            if i.key_pressed(egui::Key::F5) {
                self.viz_latency_secs += 0.010;
            }
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_dev_screenshot(ctx);
        self.perf.begin_frame(Instant::now());
        self.draw(ctx);
        let (tex_count, tex_bytes) = self.textures.stats();
        self.perf.show(
            ctx,
            &[
                format!("Compensación latencia {:+.0} ms (F4 -10 / F5 +10)", self.viz_latency_secs * 1000.0),
                format!("Texturas {tex_count} ({:.1} MB)", tex_bytes as f64 / 1_048_576.0),
            ],
        );
        self.perf.end_frame(Instant::now());

        let viz_peak = self.viz.bars.iter().cloned().fold(0.0, f32::max);
        match repaint::decide(self.state.is_playing, viz_peak) {
            repaint::Repaint::Now => ctx.request_repaint(),
            repaint::Repaint::After(d) => ctx.request_repaint_after(d),
        }
    }
}

impl App {
    fn draw(&mut self, ctx: &egui::Context) {
        self.textures.begin_frame(ctx, 3);

        if let Some(gallery) = &mut self.gallery {
            gallery.show(ctx, &mut self.textures);
            return;
        }

        self.handle_events();
        self.handle_keyboard_shortcuts(ctx);
        let tick_start = Instant::now();
        self.state.tick();
        self.perf.record_tick(tick_start.elapsed());
        self.update_viz();
        self.state.ensure_lyrics_for_current_song();

        let cover = self.state.current_song().and_then(|s| s.cover_art.clone());
        self.backdrop.show(ctx, &mut self.textures, cover.as_deref(), ctx.screen_rect());
        theme::set_accent(ctx, self.backdrop.accent());

        let sidebar = egui::SidePanel::left("sidebar")
            .exact_width(236.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(theme::GLASS_FILL_STRONG).inner_margin(egui::Margin::symmetric(16.0, 24.0)))
            .show(ctx, |ui| {
                ui::shell::sidebar::show(ui, &mut self.state);
            });

        let bar = egui::TopBottomPanel::bottom("player_bar")
            .exact_height(ui::shell::player_bar::HEIGHT)
            .frame(egui::Frame::none().fill(theme::GLASS_FILL_STRONG))
            .show(ctx, |ui| {
                ui::shell::player_bar::show(ui, &mut self.state, &mut self.textures, &self.viz);
            });

        let queue = self.state.show_queue.then(|| {
            egui::SidePanel::right("queue")
                .exact_width(332.0)
                .resizable(false)
                .frame(egui::Frame::none().fill(theme::GLASS_FILL_STRONG).inner_margin(egui::Margin::symmetric(16.0, 20.0)))
                .show(ctx, |ui| {
                    ui::screens::queue_panel::show(ui, &mut self.state, &mut self.textures);
                })
        });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin::symmetric(28.0, 20.0)))
            .show(ctx, |ui| {
                ui::shell::topbar::show(ui, &mut self.state);
                ui.add_space(16.0);
                ui::screens::show(ui, &mut self.state, &mut self.textures);
            });

        self.fullscreen.show(ctx, &mut self.state, &mut self.textures, &self.backdrop, &self.viz);

        let lines = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, egui::Id::new("shell_borders")));
        let stroke = egui::Stroke::new(1.0_f32, theme::GLASS_BORDER);
        let (s, b) = (sidebar.response.rect, bar.response.rect);
        lines.vline(s.right() - 0.5, s.y_range(), stroke);
        lines.hline(b.x_range(), b.top() + 0.5, stroke);
        if let Some(queue) = queue {
            let q = queue.response.rect;
            lines.vline(q.left() + 0.5, q.y_range(), stroke);
        }
    }
}

const ICON_PNG: &[u8] = include_bytes!("../assets/icons/icon.png");

fn load_icon(png: &[u8]) -> Option<egui::IconData> {
    let img = image::load_from_memory(png).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData { rgba: img.into_raw(), width, height })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_icono_incrustado_se_decodifica() {
        let icon = load_icon(ICON_PNG).expect("el PNG incrustado debe decodificar");
        assert!(icon.width > 0 && icon.height > 0);
        assert_eq!(icon.rgba.len(), (icon.width * icon.height * 4) as usize);
    }

    #[test]
    fn bytes_invalidos_no_producen_icono() {
        assert!(load_icon(b"no es un png").is_none());
    }
}

fn main() -> eframe::Result<()> {
    let icon = load_icon(ICON_PNG);

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1600.0, 1000.0])
        .with_title("Simple Player");
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Simple Player",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}
