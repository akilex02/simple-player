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

use eframe::egui;
use events::AppEvent;
use gstreamer::prelude::*;
use state::AppState;
use std::sync::mpsc;

struct App {
    state: AppState,
    hotkeys: hotkeys::Hotkeys,
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    last_spectrum: Vec<f32>,
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

        self.hotkeys.poll(&self.tx);
        self.handle_events();
        self.handle_keyboard_shortcuts(ctx);
        self.state.tick();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Simple Player — prototipo nativo (Fase 2)");
            ui.label(format!("Canciones en biblioteca: {}", self.state.songs.len()));

            ui.separator();
            match self.state.current_song() {
                Some(song) => {
                    ui.label(format!("{} — {}", song.title, song.artist));
                    ui.label(format!(
                        "{:.0}s / {}s",
                        self.state.current_time, song.duration_secs
                    ));
                }
                None => {
                    ui.label("Ninguna canción seleccionada");
                }
            }

            ui.horizontal(|ui| {
                if ui.button("⏮").clicked() {
                    self.state.handle_prev_song();
                }
                if ui.button(if self.state.is_playing { "⏸" } else { "▶" }).clicked() {
                    self.state.toggle_play_pause();
                }
                if ui.button("⏭").clicked() {
                    self.state.handle_next_song();
                }
                if ui.button("🔀 Shuffle + reproducir todo").clicked() {
                    let songs: Vec<_> = self.state.sorted_songs().into_iter().cloned().collect();
                    self.state.start_shuffle_play(songs);
                }
            });

            ui.separator();
            ui.label("Espectro (32 bandas, dB):");
            let (_, painter_rect) = ui.allocate_space(egui::vec2(ui.available_width(), 120.0));
            let painter = ui.painter_at(painter_rect);
            let gap = 4.0;
            let bar_width = (painter_rect.width() - gap * 31.0) / 32.0;
            for (i, db) in self.last_spectrum.iter().enumerate() {
                let norm = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
                let h = norm * painter_rect.height();
                let x = painter_rect.left() + i as f32 * (bar_width + gap);
                let rect = egui::Rect::from_min_size(
                    egui::pos2(x, painter_rect.bottom() - h),
                    egui::vec2(bar_width, h),
                );
                painter.rect_filled(rect, 2.0, theme::ACCENT_PINK);
            }

            ui.separator();
            ui.label("Primeras 15 canciones (orden actual):");
            for song in self.state.sorted_songs().into_iter().take(15).cloned().collect::<Vec<_>>() {
                if ui.selectable_label(false, format!("{} — {}", song.title, song.artist)).clicked() {
                    self.state.handle_play_song_from_list(&song, None);
                }
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
