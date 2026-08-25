mod audio;
mod events;
mod hotkeys;
mod library;
mod lyrics;
mod mpris;
mod paths;
mod persistence;
mod state;
mod theme;
mod ui;

use eframe::egui;
use events::AppEvent;
use gstreamer::prelude::*;
use state::{ActiveTab, AppState};
use std::sync::mpsc;
use std::time::Instant;
use ui::textures::TextureCache;
use ui::visualizers::VisualizerMode;

struct App {
    state: AppState,
    hotkeys: hotkeys::Hotkeys,
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    last_spectrum: Vec<f32>,
    last_active_lyric_line: Option<usize>,
    visualizer_mode: VisualizerMode,
    textures: TextureCache,
    // ── Medición de rendimiento (temporal, para la prueba de FPS) ──────────
    frame_count: u32,
    fps_window_start: Instant,
    fps: f64,
    recording: bool,
    recorded_samples: Vec<f64>,
    last_recording_summary: Option<String>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::install(&cc.egui_ctx);

        let audio = audio::player::init();
        let (tx, rx) = mpsc::channel::<AppEvent>();

        if let Ok(player) = audio.inner.lock() {
            if let Some(bus) = player.pipeline().bus() {
                audio::spectrum::install_spectrum_watch(&bus, tx.clone());
            }
        }

        let mpris_tx = mpris::spawn_mpris_thread(std::sync::Arc::clone(&audio.inner), tx.clone());
        let hotkeys = hotkeys::Hotkeys::register().expect("No se pudieron registrar los atajos globales");

        let mut state = AppState::new(audio, mpris_tx);
        state.init();

        Self {
            state,
            hotkeys,
            tx,
            rx,
            last_spectrum: vec![-60.0; 32],
            last_active_lyric_line: None,
            visualizer_mode: VisualizerMode::Bars,
            textures: TextureCache::default(),
            frame_count: 0,
            fps_window_start: Instant::now(),
            fps: 0.0,
            recording: false,
            recorded_samples: Vec::new(),
            last_recording_summary: None,
        }
    }

    /// Cuenta frames y loguea el FPS real cada 2s — lectura ambiente, no la
    /// prueba controlada (para eso está el botón de grabar).
    fn track_fps(&mut self) {
        self.frame_count += 1;
        let elapsed = self.fps_window_start.elapsed().as_secs_f64();
        if elapsed >= 2.0 {
            self.fps = self.frame_count as f64 / elapsed;
            println!("[perf] {:.1} fps ({} frames en {:.2}s)", self.fps, self.frame_count, elapsed);
            self.frame_count = 0;
            self.fps_window_start = Instant::now();
        }
    }

    /// Mientras `recording` esté activo, guarda el fps instantáneo de cada
    /// frame (1/dt) para poder calcular mínimo/promedio/p10 al detener —
    /// eso es lo que de verdad muestra si hubo caídas, no solo el promedio.
    fn sample_recording(&mut self, ctx: &egui::Context) {
        if !self.recording {
            return;
        }
        let dt = ctx.input(|i| i.unstable_dt) as f64;
        if dt > 0.0 {
            self.recorded_samples.push(1.0 / dt);
        }
    }

    fn toggle_recording(&mut self) {
        if self.recording {
            self.recording = false;
            if !self.recorded_samples.is_empty() {
                let mut sorted = self.recorded_samples.clone();
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let n = sorted.len();
                let pct = |p: f64| sorted[((n as f64 * p) as usize).min(n - 1)];
                let avg = sorted.iter().sum::<f64>() / n as f64;
                let under = |limit: f64| sorted.iter().filter(|v| **v < limit).count();
                let under30 = under(30.0);
                let under10 = under(10.0);

                let summary = format!(
                    "{n} fr — prom {avg:.0} | p50 {:.0} p25 {:.0} p10 {:.0} p5 {:.0} p1 {:.0} mín {:.0} | <30fps: {under30} ({:.0}%) <10fps: {under10} ({:.0}%)",
                    pct(0.5), pct(0.25), pct(0.10), pct(0.05), pct(0.01), sorted[0],
                    100.0 * under30 as f64 / n as f64,
                    100.0 * under10 as f64 / n as f64,
                );
                println!("[perf] Grabación terminada: {summary}");
                self.last_recording_summary = Some(summary);
            }
            self.recorded_samples.clear();
        } else {
            self.recording = true;
            self.recorded_samples.clear();
            self.last_recording_summary = None;
            println!("[perf] Grabación iniciada...");
        }
    }

    fn handle_events(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::Spectrum(values) => self.last_spectrum = values,
                AppEvent::MediaPrev => self.state.handle_prev_song(),
                AppEvent::MediaNext => self.state.handle_next_song(),
                AppEvent::MediaPlayPause => self.state.toggle_play_pause(),
                AppEvent::MediaPlaying(playing) => self.state.is_playing = playing,
            }
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
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();
        self.track_fps();
        self.sample_recording(ctx);
        self.textures.begin_frame(ctx, 3);

        egui::Area::new(egui::Id::new("fps_overlay"))
            .fixed_pos(egui::pos2(8.0, 4.0))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let color = egui::Color32::from_rgb(0x66, 0xff, 0x99);
                    ui.label(egui::RichText::new(format!("{:.0} fps", self.fps)).color(color).small());
                    let (label, fill) = if self.recording {
                        ("⏹ Detener grabación", egui::Color32::from_rgb(0xff, 0x5a, 0x5a))
                    } else {
                        ("⏺ Grabar FPS", egui::Color32::from_rgb(0x33, 0x33, 0x33))
                    };
                    if ui.add(egui::Button::new(egui::RichText::new(label).small()).fill(fill)).clicked() {
                        self.toggle_recording();
                    }
                    if let Some(summary) = &self.last_recording_summary {
                        ui.label(egui::RichText::new(summary).color(color).small());
                    }
                });
            });

        self.hotkeys.poll(&self.tx);
        self.handle_events();
        self.handle_keyboard_shortcuts(ctx);
        self.state.tick();
        self.state.ensure_lyrics_for_current_song();

        if self.state.is_fullscreen {
            ui::fullscreen::show(
                ctx,
                &mut self.state,
                &mut self.textures,
                &mut self.last_active_lyric_line,
                &self.last_spectrum,
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
                ui::player_bar::show(ui, &mut self.state, &mut self.textures, &self.last_spectrum);
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


fn main() -> eframe::Result<()> {
    let icon = image::open("assets/icons/icon.png")
        .ok()
        .map(|img| {
            let img = img.to_rgba8();
            let (width, height) = img.dimensions();
            egui::IconData {
                rgba: img.into_raw(),
                width,
                height,
            }
        });

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
