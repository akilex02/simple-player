mod audio;
mod events;
mod hotkeys;
mod library;
mod lyrics;
mod mpris;
mod paths;
mod persistence;
mod theme;

use eframe::egui;
use events::AppEvent;
use gstreamer::prelude::*;
use std::sync::mpsc;

struct App {
    audio: audio::AudioPlayer,
    hotkeys: hotkeys::Hotkeys,
    tx: mpsc::Sender<AppEvent>,
    rx: mpsc::Receiver<AppEvent>,
    last_spectrum: Vec<f32>,
    songs_loaded: usize,
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

        let mpris_tx = tx.clone();
        let _mpris_sender = mpris::spawn_mpris_thread(std::sync::Arc::clone(&audio.inner), mpris_tx);

        let hotkeys = hotkeys::Hotkeys::register().expect("No se pudieron registrar los atajos globales");

        // Verificación de Fase 1: confirmar que el escaneo de biblioteca (Rust puro,
        // reutilizado sin cambios) funciona desde este nuevo binario.
        let songs = library::scan_music_folder(None);
        println!("[library] {} canciones cargadas desde caché/carpeta", songs.len());

        Self {
            audio,
            hotkeys,
            tx,
            rx,
            last_spectrum: vec![-60.0; 32],
            songs_loaded: songs.len(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint();

        self.hotkeys.poll(&self.tx);
        while let Ok(event) = self.rx.try_recv() {
            match event {
                AppEvent::Spectrum(values) => self.last_spectrum = values,
                AppEvent::MediaPrev => println!("[event] media-prev"),
                AppEvent::MediaNext => println!("[event] media-next"),
                AppEvent::MediaPlayPause => println!("[event] media-playpause"),
                AppEvent::MediaPlaying(playing) => println!("[event] media-playing: {playing}"),
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Simple Player — prototipo nativo (Fase 0/1)");
            ui.label(format!("Canciones en biblioteca: {}", self.songs_loaded));
            ui.label(format!("Posición: {}s", self.audio.position_secs()));

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
