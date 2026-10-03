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
mod shortcuts;
mod single_instance;
mod window_state;
mod state;
mod stats;
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
    /// Dónde guardar el tamaño de la ventana al cerrar (`None` en corridas de desarrollo).
    window_state_path: Option<std::path::PathBuf>,
    window_size: Option<window_state::WindowSize>,
    bench: Option<Bench>,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>, window_state_path: Option<std::path::PathBuf>) -> Self {
        theme::install(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);

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
        // Las capturas de desarrollo no deben pelear los atajos con otra instancia
        // abierta: X responde con BadAccess y Xlib mata el proceso.
        let skip_hotkeys = std::env::args().any(|a| a == "--shot" || a == "--no-hotkeys");
        let hotkeys = if skip_hotkeys {
            None
        } else {
            match hotkeys::Hotkeys::register(sender) {
            Ok(h) => Some(h),
            Err(e) => {
                eprintln!("[aviso] Atajos globales desactivados: {e}");
                None
            }
            }
        };

        let mut state = AppState::new(audio, mpris_tx);
        state.init();
        if let Some(tab) = ui::gallery::arg_value("--tab") {
            match tab.as_str() {
                "albums" => state.select_tab(ActiveTab::Albums),
                "artists" => state.select_tab(ActiveTab::Artists),
                "stats" => state.select_tab(ActiveTab::Stats),
                _ => {}
            }
        }
        if let Some(range) = ui::gallery::arg_value("--stats-range") {
            state.stats_range = match range.as_str() {
                "today" => stats::model::StatsRange::Today,
                "month" => stats::model::StatsRange::Month,
                "year" => stats::model::StatsRange::Year,
                "all" => stats::model::StatsRange::All,
                _ => stats::model::StatsRange::Week,
            };
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
        if std::env::args().any(|a| a == "--play") {
            // Solo desarrollo: reproduce en silencio sin pasar por `play_index`,
            // que persistiría la cola y pisaría los ajustes guardados del usuario.
            if let Some(path) = state.current_song().map(|s| s.path.clone()) {
                let _ = state.audio.set_volume(0.0);
                let _ = state.audio.play(&path);
                state.is_playing = true;
            }
        }
        state.is_fullscreen = std::env::args().any(|a| a == "--fullscreen");
        let mut fullscreen_view = ui::fullscreen::FullscreenView::default();
        if let Some(mode) = ui::gallery::arg_value("--viz").and_then(|v| ui::visualizers::VisualizerMode::from_name(&v)) {
            fullscreen_view.mode = mode;
        }
        state.show_lyrics = std::env::args().any(|a| a == "--lyrics");

        // Estadísticas: base real en uso normal; memoria en corridas de desarrollo.
        let args: Vec<String> = std::env::args().collect();
        let location = stats::location::location_from_args(
            &args,
            std::env::var("XDG_DATA_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        );
        let repaint_ctx = cc.egui_ctx.clone();
        let stats_handle = stats::service::StatsHandle::spawn(location, std::sync::Arc::new(move || repaint_ctx.request_repaint()));
        if args.iter().any(|a| a == "--stats-demo") {
            let snapshots: Vec<stats::model::SongSnapshot> = state.songs.iter().take(60).map(stats::model::SongSnapshot::from).collect();
            stats_handle.record_batch(stats::demo::demo_events(stats::recorder::epoch_ms_now(), &snapshots));
        }
        state.stats = stats::recorder::StatsRecorder::new(stats_handle);
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
            fullscreen: fullscreen_view,
            textures: TextureCache::default(),
            perf: perf::PerfHud::new(),
            gallery,
            backdrop: ui::backdrop::Backdrop::new(),
            shot: ui::gallery::arg_value("--shot").map(|path| (path, 0)),
            window_state_path,
            window_size: None,
            bench: ui::gallery::arg_value("--bench").and_then(|v| v.parse().ok()).map(Bench::new),
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

    /// Recuerda el último tamaño "normal" de la ventana (no maximizada ni a pantalla completa).
    fn track_window_size(&mut self, ctx: &egui::Context) {
        let size = ctx.input(|i| {
            let v = i.viewport();
            if v.maximized == Some(true) || v.fullscreen == Some(true) {
                return None;
            }
            v.inner_rect.map(|r| window_state::WindowSize { width: r.width(), height: r.height() })
        });
        if size.is_some() {
            self.window_size = size;
        }
    }

    /// Flag de desarrollo `--shot <ruta>`: la app guarda su propia captura a
    /// los ~2 s y se cierra, para revisar el aspecto sin depender del escritorio.
    fn handle_dev_bench(&mut self, ctx: &egui::Context) {
        let Some(bench) = &mut self.bench else { return };
        // Sondeo lento: no debe alterar lo que se mide.
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
        // (el conteo de frames solo incluye los que ya se dibujan por otras razones)
        if let Some(report) = bench.poll() {
            println!("{report}");
            self.bench = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

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

        let widget_focused = ctx.memory(|m| m.focused().is_some());
        let (duration, now_secs) = (self.state.current_song().map(|s| s.duration_secs as f64).unwrap_or(0.0), self.state.current_time);
        let mut seek_by: Option<f64> = None;
        let mut volume_by: Option<f64> = None;
        ctx.input(|i| {
            let plain = !i.modifiers.alt && !i.modifiers.command && !i.modifiers.shift;
            if plain && !editing_text && !widget_focused {
                if i.key_pressed(egui::Key::ArrowLeft) { seek_by = Some(-shortcuts::SEEK_STEP_SECS); }
                if i.key_pressed(egui::Key::ArrowRight) { seek_by = Some(shortcuts::SEEK_STEP_SECS); }
                if i.key_pressed(egui::Key::ArrowUp) { volume_by = Some(shortcuts::VOLUME_STEP); }
                if i.key_pressed(egui::Key::ArrowDown) { volume_by = Some(-shortcuts::VOLUME_STEP); }
                if i.key_pressed(egui::Key::M) { self.state.toggle_mute(); }
                if i.key_pressed(egui::Key::Q) { self.state.show_queue = !self.state.show_queue; }
                if i.key_pressed(egui::Key::F) { self.state.is_fullscreen = !self.state.is_fullscreen; }
                if i.key_pressed(egui::Key::L) {
                    if self.state.is_fullscreen { self.state.toggle_lyrics_visibility(); } else { self.state.open_lyrics(); }
                }
            }
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
        if let Some(delta) = seek_by {
            self.state.seek_commit(shortcuts::seek_target(now_secs, delta, duration));
        }
        if let Some(delta) = volume_by {
            self.state.set_volume(shortcuts::step_volume(self.state.volume, delta));
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.track_window_size(ctx);
        self.handle_dev_screenshot(ctx);
        self.handle_dev_bench(ctx);
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

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let (Some(path), Some(size)) = (&self.window_state_path, self.window_size) {
            let _ = window_state::save(path, size);
        }
        self.state.stats.finish();
        self.state.stats.handle().flush(std::time::Duration::from_secs(1));
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
        self.state.observe_stats();
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

/// Mide el CPU del proceso entre `WARMUP` y `WARMUP + secs` (solo desarrollo).
struct Bench {
    secs: f64,
    started: Instant,
    from: Option<u64>,
    frames: u64,
    frames_from: u64,
}

impl Bench {
    const WARMUP: f64 = 4.0;
    /// Ticks de CPU por segundo en Linux (`getconf CLK_TCK`).
    const CLK_TCK: f64 = 100.0;

    fn new(secs: f64) -> Self {
        Self { secs, started: Instant::now(), from: None, frames: 0, frames_from: 0 }
    }

    fn cpu_ticks() -> u64 {
        let stat = std::fs::read_to_string("/proc/self/stat").unwrap_or_default();
        let after_comm = stat.rsplit_once(')').map(|(_, rest)| rest).unwrap_or("");
        let fields: Vec<&str> = after_comm.split_whitespace().collect();
        let tick = |i: usize| fields.get(i).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        tick(11) + tick(12)
    }

    fn poll(&mut self) -> Option<String> {
        self.frames += 1;
        let elapsed = self.started.elapsed().as_secs_f64();
        match self.from {
            None if elapsed >= Self::WARMUP => {
                self.from = Some(Self::cpu_ticks());
                self.frames_from = self.frames;
                None
            }
            Some(from) if elapsed >= Self::WARMUP + self.secs => {
                let used = (Self::cpu_ticks() - from) as f64 / Self::CLK_TCK;
                let frames = (self.frames - self.frames_from).max(1) as f64;
                Some(format!(
                    "[bench] CPU {:.1} % | {:.0} fps | {:.2} ms de CPU por frame",
                    used / self.secs * 100.0,
                    frames / self.secs,
                    used / frames * 1000.0
                ))
            }
            _ => None,
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
    // Las capturas y la galería de desarrollo conviven con una instancia abierta.
    let dev_run = ["--shot", "--gallery", "--allow-multiple"].iter().any(|f| std::env::args().any(|a| a == *f));
    let _instance_lock = if dev_run {
        None
    } else {
        match single_instance::acquire(&single_instance::default_path()) {
            Ok(single_instance::Acquire::Acquired(lock)) => Some(lock),
            Ok(single_instance::Acquire::AlreadyRunning) => {
                eprintln!("Simple Player ya está abierto.");
                return Ok(());
            }
            Err(e) => {
                eprintln!("[aviso] No se pudo comprobar la instancia única: {e}");
                None
            }
        }
    };

    let icon = load_icon(ICON_PNG);

    // Tamaño de la ventana: `--window-size AxB` (pruebas) > el guardado > el predeterminado.
    // Las corridas de desarrollo no leen ni escriben el archivo del usuario.
    let dev_window = ["--shot", "--bench", "--gallery"].iter().any(|f| std::env::args().any(|a| a == *f));
    let saved_path = window_state::window_state_path(
        std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    );
    let forced = ui::gallery::arg_value("--window-size").and_then(|v| window_state::parse_size_arg(&v));
    let initial = forced
        .or_else(|| if dev_window { None } else { window_state::load(&saved_path) })
        .unwrap_or(window_state::DEFAULT_SIZE);
    let window_state_path = (!dev_window && forced.is_none()).then_some(saved_path);

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([initial.width, initial.height])
        .with_min_inner_size(if forced.is_some() { [200.0, 200.0] } else { [window_state::MIN_SIZE.0, window_state::MIN_SIZE.1] })
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
        Box::new(move |cc| Ok(Box::new(App::new(cc, window_state_path)))),
    )
}
