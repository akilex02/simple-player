use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};

/// Carga carátulas desde disco como texturas de egui, cacheadas por ruta
/// para no releer/redecodificar el JPEG en cada frame.
///
/// La decodificación (I/O de disco + `image::open` + resize) se hace en un
/// hilo de fondo por cada carátula pendiente, no en el hilo de UI: eso es lo
/// que causaba stutter severo (percibido como caídas a 0fps, no reflejadas
/// en la métrica de `unstable_dt`) al hacer scroll rápido — decenas de
/// decodificaciones síncronas de JPEGs de varios cientos de KB bloqueaban el
/// hilo principal de egui. El hilo principal solo hace `ctx.load_texture(...)`
/// (subida a GPU) una vez que el hilo de fondo entrega el buffer RGBA ya
/// decodificado por un canal `mpsc`.
///
/// Presupuesto por frame (`begin_frame(budget)`): limita cuántas
/// decodificaciones *nuevas* se pueden lanzar en un mismo frame, para que un
/// scroll rápido no dispare decenas de hilos de golpe.
#[derive(Default)]
pub struct TextureCache {
    cache: HashMap<String, egui::TextureHandle>,
    pending: HashSet<String>,
    failed: HashSet<String>,
    budget_this_frame: u32,
    channel: Option<(Sender<DecodeResult>, Receiver<DecodeResult>)>,
}

struct DecodedImage {
    path: String,
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

enum DecodeResult {
    Ok(DecodedImage),
    Err(String),
}

const MAX_DIMENSION: u32 = 256;

impl TextureCache {
    fn channel(&mut self) -> &(Sender<DecodeResult>, Receiver<DecodeResult>) {
        self.channel.get_or_insert_with(channel)
    }

    /// Llamar una vez al inicio de cada frame, antes de dibujar cualquier UI:
    /// resetea el presupuesto y sube a GPU las carátulas que el hilo de
    /// fondo ya terminó de decodificar desde el frame anterior.
    pub fn begin_frame(&mut self, ctx: &egui::Context, budget: u32) {
        self.budget_this_frame = budget;

        let results: Vec<DecodeResult> = match &self.channel {
            Some((_, rx)) => rx.try_iter().collect(),
            None => Vec::new(),
        };
        for result in results {
            match result {
                DecodeResult::Ok(img) => {
                    self.pending.remove(&img.path);
                    let color_image =
                        egui::ColorImage::from_rgba_unmultiplied([img.w, img.h], &img.rgba);
                    let handle = ctx.load_texture(&img.path, color_image, egui::TextureOptions::LINEAR);
                    self.cache.insert(img.path, handle);
                }
                DecodeResult::Err(path) => {
                    self.pending.remove(&path);
                    self.failed.insert(path);
                }
            }
        }
    }

    pub fn get_or_load(&mut self, path: &str) -> Option<egui::TextureHandle> {
        if let Some(tex) = self.cache.get(path) {
            return Some(tex.clone());
        }
        if self.pending.contains(path) || self.failed.contains(path) {
            return None;
        }
        if self.budget_this_frame == 0 {
            return None;
        }
        self.budget_this_frame -= 1;
        self.pending.insert(path.to_string());

        let tx = self.channel().0.clone();
        let path_owned = path.to_string();
        std::thread::spawn(move || {
            let decoded = image::open(&path_owned).ok().map(|img| {
                let img = img
                    .resize(MAX_DIMENSION, MAX_DIMENSION, image::imageops::FilterType::Triangle)
                    .to_rgba8();
                let (w, h) = img.dimensions();
                DecodedImage {
                    path: path_owned.clone(),
                    w: w as usize,
                    h: h as usize,
                    rgba: img.into_raw(),
                }
            });
            let result = match decoded {
                Some(img) => DecodeResult::Ok(img),
                None => DecodeResult::Err(path_owned),
            };
            let _ = tx.send(result);
        });

        None
    }
}
