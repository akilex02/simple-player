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
use ui::visualizers::VisualizerMode;
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
    last_active_lyric_line: Option<usize>,
    visualizer_mode: VisualizerMode,
    textures: TextureCache,
    perf: perf::PerfHud,
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
            last_active_lyric_line: None,
            visualizer_mode: VisualizerMode::Bars,
            textures: TextureCache::default(),
            perf: perf::PerfHud::new(),
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

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // Coincide con los atajos globales de teclado de App.tsx: se ignoran
        // si el foco está en un campo de texto (egui ya no manda `Space` como
        // texto a un widget enfocado en ese caso, pero sí evitamos el bloqueo de flechas).
        let editing_text = ctx.wants_keyboard_input();

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

        self.handle_events();
        self.handle_keyboard_shortcuts(ctx);
        let tick_start = Instant::now();
        self.state.tick();
        self.perf.record_tick(tick_start.elapsed());
        self.update_viz();
        self.state.ensure_lyrics_for_current_song();

        if self.state.is_fullscreen {
            ui::fullscreen::show(
                ctx,
                &mut self.state,
                &mut self.textures,
                &mut self.last_active_lyric_line,
                &self.viz,
                &mut self.visualizer_mode,
            );
            return;
        }

        egui::SidePanel::left("sidebar")
            .exact_width(250.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(theme::BG_SIDEBAR).inner_margin(20.0))
            .show(ctx, |ui| {
                ui::sidebar::show(ui, &mut self.state);
            });

        egui::TopBottomPanel::bottom("player_bar")
            .exact_height(96.0)
            .frame(egui::Frame::none().fill(theme::BG_CARD).inner_margin(16.0))
            .show(ctx, |ui| {
                ui::player_bar::show(ui, &mut self.state, &mut self.textures, &self.viz);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG_DARK).inner_margin(28.0))
            .show(ctx, |ui| {
                ui::header::show(ui, &mut self.state);
                ui.add_space(16.0);

                let show_artists_grid =
                    self.state.active_tab == ActiveTab::Artists && self.state.selected_artist.is_none();

                if show_artists_grid {
                    ui::artists_grid::show(ui, &mut self.state, &mut self.textures);
                } else {
                    ui::song_table::show(ui, &mut self.state, &mut self.textures);
                }
            });
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
