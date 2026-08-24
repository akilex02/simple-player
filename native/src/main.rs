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
                self.show_player_bar(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(theme::BG_DARK).inner_margin(28.0))
            .show(ctx, |ui| {
                ui::header::show(ui, &mut self.state);
                ui.add_space(16.0);

                let show_artists_grid =
                    self.state.active_tab == ActiveTab::Artists && self.state.selected_artist.is_none();

                if show_artists_grid {
                    ui::artists_grid::show(ui, &mut self.state);
                } else {
                    ui::song_table::show(ui, &mut self.state);
                }
            });
    }
}

impl App {
    fn show_player_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            match self.state.current_song() {
                Some(song) => {
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new(&song.title).color(theme::TEXT_MAIN).strong());
                        ui.label(egui::RichText::new(&song.artist).color(theme::ACCENT_PINK).size(12.0));
                    });
                }
                None => {
                    ui.label(egui::RichText::new("Ninguna canción").color(theme::TEXT_MUTED));
                }
            }

            ui.add_space(24.0);
            if ui.button("⏮").clicked() {
                self.state.handle_prev_song();
            }
            if ui.button(if self.state.is_playing { "⏸" } else { "▶" }).clicked() {
                self.state.toggle_play_pause();
            }
            if ui.button("⏭").clicked() {
                self.state.handle_next_song();
            }
            if ui
                .add(egui::Button::new("🔀").fill(if self.state.is_shuffle {
                    theme::ACCENT_PINK
                } else {
                    theme::BG_CARD_HOVER
                }))
                .clicked()
            {
                self.state.toggle_shuffle();
            }
            if ui.button("🔁").clicked() {
                self.state.toggle_repeat_mode();
            }

            ui.add_space(16.0);
            if let Some(song) = self.state.current_song() {
                ui.label(format!(
                    "{} / {}",
                    ui::format_time(self.state.current_time as u64),
                    ui::format_time(song.duration_secs)
                ));
            }

            ui.add_space(16.0);
            ui.label("Espectro:");
            let (_, rect) = ui.allocate_space(egui::vec2(220.0, 60.0));
            let painter = ui.painter_at(rect);
            let gap = 2.0;
            let bar_width = (rect.width() - gap * 31.0) / 32.0;
            for (i, db) in self.last_spectrum.iter().enumerate() {
                let norm = ((db + 60.0) / 60.0).clamp(0.0, 1.0);
                let h = norm * rect.height();
                let x = rect.left() + i as f32 * (bar_width + gap);
                let bar = egui::Rect::from_min_size(
                    egui::pos2(x, rect.bottom() - h),
                    egui::vec2(bar_width, h),
                );
                painter.rect_filled(bar, 1.0, theme::ACCENT_PINK);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut volume = self.state.volume as f32;
                if ui.add(egui::Slider::new(&mut volume, 0.0..=1.0).show_value(false)).changed() {
                    self.state.set_volume(volume as f64);
                }
                if ui
                    .add(egui::Button::new(if self.state.is_normalize_volume { "NORM ✓" } else { "NORM" }))
                    .clicked()
                {
                    self.state.toggle_normalize_volume();
                }
            });
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
