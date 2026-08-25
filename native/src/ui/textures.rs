use eframe::egui;
use std::collections::HashMap;

/// Carga carátulas desde disco como texturas de egui, cacheadas por ruta
/// para no releer/redecodificar el JPEG en cada frame.
///
/// Presupuesto por frame: decodificar+subir a GPU es trabajo síncrono en el
/// hilo de UI. Sin límite, un scroll rápido sobre la lista hace que decenas
/// de filas nuevas entren a la vista en el mismo frame e intenten cargar su
/// portada todas de golpe — eso es lo que causaba la caída a ~1fps. Con el
/// presupuesto, las que no alcanzan este frame muestran el emoji de
/// respaldo y se resuelven en los siguientes frames, sin bloquear el scroll.
#[derive(Default)]
pub struct TextureCache {
    cache: HashMap<String, egui::TextureHandle>,
    budget_this_frame: u32,
}

const MAX_DIMENSION: u32 = 256;

impl TextureCache {
    /// Llamar una vez al inicio de cada frame, antes de dibujar cualquier UI.
    pub fn begin_frame(&mut self, budget: u32) {
        self.budget_this_frame = budget;
    }

    pub fn get_or_load(&mut self, ctx: &egui::Context, path: &str) -> Option<egui::TextureHandle> {
        if let Some(tex) = self.cache.get(path) {
            return Some(tex.clone());
        }
        if self.budget_this_frame == 0 {
            return None;
        }
        self.budget_this_frame -= 1;

        let img = image::open(path).ok()?;
        // Los thumbnails nunca se muestran a más de ~220px — bajar la
        // resolución acá acota el costo de subida a GPU, sobre todo con
        // portadas embebidas de alta resolución.
        let img = img
            .resize(MAX_DIMENSION, MAX_DIMENSION, image::imageops::FilterType::Triangle)
            .to_rgba8();
        let (w, h) = img.dimensions();
        let color_image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], img.as_raw());
        let handle = ctx.load_texture(path, color_image, egui::TextureOptions::LINEAR);

        self.cache.insert(path.to_string(), handle.clone());
        Some(handle)
    }
}
