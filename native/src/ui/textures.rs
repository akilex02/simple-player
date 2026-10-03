use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};

/// Tamaño en que se decodifica una carátula, según dónde se dibuja: filas de
/// la tabla, tarjetas, o pantalla completa.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CoverSize {
    Thumb,
    Card,
    Large,
}

impl CoverSize {
    pub fn px(self) -> u32 {
        match self {
            CoverSize::Thumb => 64,
            CoverSize::Card => 256,
            CoverSize::Large => 512,
        }
    }

    fn index(self) -> usize {
        self as usize
    }

    /// Elige el tamaño según los píxeles físicos en que se va a dibujar.
    pub fn for_physical_px(px: f32) -> Self {
        if px <= 72.0 {
            CoverSize::Thumb
        } else if px <= 288.0 {
            CoverSize::Card
        } else {
            CoverSize::Large
        }
    }
}

pub struct EvictionCandidate<K> {
    pub key: K,
    pub bytes: usize,
    pub last_used: u64,
}

/// Qué entradas sacar para volver a `cap` bytes: las menos usadas recientemente
/// primero, sin tocar las usadas desde `protect_from` (frame actual o anterior).
pub fn pick_evictions<K: Clone>(entries: &[EvictionCandidate<K>], cap: usize, protect_from: u64) -> Vec<K> {
    let mut total: usize = entries.iter().map(|e| e.bytes).sum();
    let mut by_age: Vec<&EvictionCandidate<K>> = entries.iter().collect();
    by_age.sort_by_key(|e| e.last_used);

    let mut evicted = Vec::new();
    for e in by_age {
        if total <= cap {
            break;
        }
        if e.last_used >= protect_from {
            continue;
        }
        total -= e.bytes;
        evicted.push(e.key.clone());
    }
    evicted
}

/// Cola de decodificaciones pendientes: lo pedido más recientemente sale
/// primero (lo que está en pantalla ahora) y lo que dejó de pedirse se descarta.
pub struct JobQueue<K> {
    jobs: Vec<Job<K>>,
    next_seq: u64,
}

struct Job<K> {
    key: K,
    stamp: u64,
    seq: u64,
}

const STALE_FRAMES: u64 = 2;

impl<K: PartialEq + Clone> JobQueue<K> {
    pub fn new() -> Self {
        Self { jobs: Vec::new(), next_seq: 0 }
    }

    pub fn request(&mut self, key: K, frame: u64) {
        match self.jobs.iter_mut().find(|j| j.key == key) {
            Some(job) => job.stamp = job.stamp.max(frame),
            None => {
                self.jobs.push(Job { key, stamp: frame, seq: self.next_seq });
                self.next_seq += 1;
            }
        }
    }

    pub fn next_batch(&mut self, frame: u64, n: usize) -> Vec<K> {
        self.jobs.retain(|j| frame.saturating_sub(j.stamp) <= STALE_FRAMES);
        self.jobs.sort_by_key(|j| (std::cmp::Reverse(j.stamp), j.seq));
        let take = n.min(self.jobs.len());
        self.jobs.drain(..take).map(|j| j.key).collect()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.jobs.len()
    }
}

struct Entry {
    handle: egui::TextureHandle,
    bytes: usize,
    last_used: u64,
}

#[derive(Default)]
struct SizeCache {
    entries: HashMap<String, Entry>,
    pending: HashSet<String>,
    failed: HashSet<String>,
}

struct DecodedImage {
    size: CoverSize,
    path: String,
    w: usize,
    h: usize,
    rgba: Vec<u8>,
}

enum DecodeResult {
    Ok(DecodedImage),
    Err(CoverSize, String),
}

type Key = (CoverSize, String);

const DEFAULT_BUDGET_BYTES: usize = 64 * 1024 * 1024;
const MAX_INFLIGHT: usize = 4;
const MAX_UPLOADS_PER_FRAME: usize = 4;

/// Carátulas como texturas de egui: decodificadas en hilos de fondo al tamaño
/// que se necesita (64 / 256 / 512 px), con memoria acotada por LRU y una cola
/// que decodifica primero lo que está en pantalla. El hilo de UI solo sube a
/// GPU (`load_texture`) lo ya decodificado.
pub struct TextureCache {
    caches: [SizeCache; 3],
    queue: JobQueue<Key>,
    frame: u64,
    budget_bytes: usize,
    ctx: Option<egui::Context>,
    channel: Option<(Sender<DecodeResult>, Receiver<DecodeResult>)>,
}

impl Default for TextureCache {
    fn default() -> Self {
        Self::with_budget(DEFAULT_BUDGET_BYTES)
    }
}

impl TextureCache {
    pub fn with_budget(budget_bytes: usize) -> Self {
        Self {
            caches: Default::default(),
            queue: JobQueue::new(),
            frame: 0,
            budget_bytes,
            ctx: None,
            channel: None,
        }
    }

    /// (texturas en memoria, bytes que ocupan).
    pub fn stats(&self) -> (usize, usize) {
        let all = self.caches.iter().flat_map(|c| c.entries.values());
        all.fold((0, 0), |(n, b), e| (n + 1, b + e.bytes))
    }

    /// Llamar una vez al inicio de cada frame, antes de dibujar la UI: sube a
    /// GPU lo decodificado, expulsa lo que sobre del presupuesto y lanza las
    /// decodificaciones pendientes (máx. `budget` nuevas por frame).
    pub fn begin_frame(&mut self, ctx: &egui::Context, budget: u32) {
        self.frame += 1;
        self.ctx = Some(ctx.clone());

        let results: Vec<DecodeResult> = match &self.channel {
            Some((_, rx)) => rx.try_iter().take(MAX_UPLOADS_PER_FRAME).collect(),
            None => Vec::new(),
        };
        if results.len() == MAX_UPLOADS_PER_FRAME {
            ctx.request_repaint();
        }
        for result in results {
            match result {
                DecodeResult::Ok(img) => {
                    let cache = &mut self.caches[img.size.index()];
                    cache.pending.remove(&img.path);
                    let color = egui::ColorImage::from_rgba_unmultiplied([img.w, img.h], &img.rgba);
                    let name = format!("{:?}:{}", img.size, img.path);
                    let handle = ctx.load_texture(name, color, egui::TextureOptions::LINEAR);
                    cache.entries.insert(img.path, Entry { handle, bytes: img.w * img.h * 4, last_used: self.frame });
                }
                DecodeResult::Err(size, path) => {
                    let cache = &mut self.caches[size.index()];
                    cache.pending.remove(&path);
                    cache.failed.insert(path);
                }
            }
        }

        self.evict_over_budget();
        self.dispatch(ctx, budget as usize);
    }

    fn evict_over_budget(&mut self) {
        let candidates: Vec<EvictionCandidate<(usize, String)>> = self
            .caches
            .iter()
            .enumerate()
            .flat_map(|(i, c)| {
                c.entries.iter().map(move |(path, e)| EvictionCandidate {
                    key: (i, path.clone()),
                    bytes: e.bytes,
                    last_used: e.last_used,
                })
            })
            .collect();
        for (i, path) in pick_evictions(&candidates, self.budget_bytes, self.frame.saturating_sub(1)) {
            self.caches[i].entries.remove(&path);
        }
    }

    fn dispatch(&mut self, ctx: &egui::Context, budget: usize) {
        let inflight: usize = self.caches.iter().map(|c| c.pending.len()).sum();
        let slots = MAX_INFLIGHT.saturating_sub(inflight).min(budget);
        for (size, path) in self.queue.next_batch(self.frame, slots) {
            self.caches[size.index()].pending.insert(path.clone());
            let tx = self.channel.get_or_insert_with(channel).0.clone();
            let ctx = ctx.clone();
            std::thread::spawn(move || {
                let result = match decode(&path, size) {
                    Some(img) => DecodeResult::Ok(img),
                    None => DecodeResult::Err(size, path),
                };
                let _ = tx.send(result);
                ctx.request_repaint();
            });
        }
    }

    pub fn get_or_load(&mut self, path: &str, size: CoverSize) -> Option<egui::TextureHandle> {
        let cache = &mut self.caches[size.index()];
        if let Some(entry) = cache.entries.get_mut(path) {
            entry.last_used = self.frame;
            return Some(entry.handle.clone());
        }
        if !cache.pending.contains(path) && !cache.failed.contains(path) {
            self.queue.request((size, path.to_string()), self.frame);
            if let Some(ctx) = &self.ctx {
                ctx.request_repaint();
            }
        }
        None
    }
}

fn decode(path: &str, size: CoverSize) -> Option<DecodedImage> {
    let img = image::open(path)
        .ok()?
        .resize(size.px(), size.px(), image::imageops::FilterType::Triangle)
        .to_rgba8();
    let (w, h) = img.dimensions();
    Some(DecodedImage { size, path: path.to_string(), w: w as usize, h: h as usize, rgba: img.into_raw() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(key: &'static str, bytes: usize, last_used: u64) -> EvictionCandidate<&'static str> {
        EvictionCandidate { key, bytes, last_used }
    }

    #[test]
    fn el_tamano_depende_de_los_pixeles_fisicos() {
        assert_eq!(CoverSize::for_physical_px(30.0), CoverSize::Thumb);
        assert_eq!(CoverSize::for_physical_px(60.0), CoverSize::Thumb);
        assert_eq!(CoverSize::for_physical_px(160.0), CoverSize::Card);
        assert_eq!(CoverSize::for_physical_px(600.0), CoverSize::Large);
    }

    #[test]
    fn bajo_el_tope_no_se_expulsa_nada() {
        let e = [cand("a", 100, 1), cand("b", 100, 2)];
        assert!(pick_evictions(&e, 200, 10).is_empty());
    }

    #[test]
    fn se_expulsan_los_menos_recientes_hasta_caber() {
        let e = [cand("nuevo", 100, 9), cand("viejo", 100, 1), cand("medio", 100, 5)];
        assert_eq!(pick_evictions(&e, 200, 100), ["viejo"]);
        assert_eq!(pick_evictions(&e, 100, 100), ["viejo", "medio"]);
    }

    #[test]
    fn nunca_se_expulsan_las_entradas_en_uso() {
        let e = [cand("visible1", 100, 9), cand("visible2", 100, 10), cand("viejo", 100, 1)];
        assert_eq!(pick_evictions(&e, 50, 9), ["viejo"]);
    }

    #[test]
    fn lo_pedido_en_el_frame_mas_reciente_sale_primero() {
        let mut q = JobQueue::new();
        q.request("a", 10);
        q.request("b", 11);
        assert_eq!(q.next_batch(11, 1), ["b"]);
        assert_eq!(q.next_batch(11, 5), ["a"]);
    }

    #[test]
    fn dentro_del_mismo_frame_se_respeta_el_orden_de_pedido() {
        let mut q = JobQueue::new();
        for k in ["arriba", "medio", "abajo"] {
            q.request(k, 5);
        }
        assert_eq!(q.next_batch(5, 3), ["arriba", "medio", "abajo"]);
    }

    #[test]
    fn pedir_dos_veces_lo_mismo_no_lo_duplica_y_lo_refresca() {
        let mut q = JobQueue::new();
        q.request("a", 1);
        q.request("b", 2);
        q.request("a", 3);
        assert_eq!(q.len(), 2);
        assert_eq!(q.next_batch(3, 1), ["a"]);
    }

    #[test]
    fn lo_que_dejo_de_pedirse_se_descarta() {
        let mut q = JobQueue::new();
        q.request("fuera_de_pantalla", 1);
        q.request("visible", 10);
        assert_eq!(q.next_batch(10, 5), ["visible"]);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn el_lote_respeta_el_maximo_y_deja_el_resto_en_cola() {
        let mut q = JobQueue::new();
        for k in ["a", "b", "c"] {
            q.request(k, 1);
        }
        assert_eq!(q.next_batch(1, 2).len(), 2);
        assert_eq!(q.len(), 1);
    }

    // ── TextureCache (egui sin ventana) ────────────────────────────────────
    fn write_png(dir: &std::path::Path, name: &str) -> String {
        let path = dir.join(name);
        image::RgbaImage::from_pixel(100, 100, image::Rgba([200, 50, 90, 255])).save(&path).unwrap();
        path.to_string_lossy().to_string()
    }

    fn load_blocking(cache: &mut TextureCache, ctx: &egui::Context, path: &str, size: CoverSize) {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            cache.begin_frame(ctx, 3);
            if cache.get_or_load(path, size).is_some() {
                return;
            }
            assert!(std::time::Instant::now() < deadline, "la carátula no cargó: {path}");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }

    #[test]
    fn la_memoria_de_texturas_queda_acotada_y_se_expulsa_la_menos_reciente() {
        let dir = std::env::temp_dir().join(format!("sp_tex_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (a, b, c) = (write_png(&dir, "a.png"), write_png(&dir, "b.png"), write_png(&dir, "c.png"));

        let ctx = egui::Context::default();
        // Cada miniatura 64×64 pesa 16 384 B: caben dos, no tres.
        let mut cache = TextureCache::with_budget(40_000);
        for path in [&a, &b, &c] {
            load_blocking(&mut cache, &ctx, path, CoverSize::Thumb);
        }
        cache.begin_frame(&ctx, 3);
        cache.begin_frame(&ctx, 3);

        let (count, bytes) = cache.stats();
        assert_eq!(count, 2, "bytes {bytes}");
        assert!(bytes <= 40_000);
        assert!(cache.get_or_load(&c, CoverSize::Thumb).is_some());
        assert!(cache.get_or_load(&a, CoverSize::Thumb).is_none(), "la más antigua debía expulsarse");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn la_decodificacion_respeta_el_tamano_pedido() {
        let dir = std::env::temp_dir().join(format!("sp_tex_size_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let big = dir.join("big.png");
        image::RgbaImage::from_pixel(1000, 1000, image::Rgba([1, 2, 3, 255])).save(&big).unwrap();
        let path = big.to_string_lossy().to_string();

        let thumb = decode(&path, CoverSize::Thumb).unwrap();
        let card = decode(&path, CoverSize::Card).unwrap();
        assert_eq!((thumb.w, thumb.h), (64, 64));
        assert_eq!((card.w, card.h), (256, 256));
        std::fs::remove_dir_all(&dir).ok();
    }
}
