/// Reemplaza los eventos de Tauri (`emit`/`listen`). Los hilos de fondo
/// (MPRIS, atajos globales, bus de GStreamer) mandan por este canal; el loop
/// de `App::update()` los consume con `try_recv()` al inicio de cada frame.
#[derive(Debug, Clone)]
pub enum AppEvent {
    /// Magnitudes del elemento `spectrum` (32 bandas, en dB), ~20 veces/seg.
    Spectrum(Vec<f32>),
    MediaPrev,
    MediaNext,
    MediaPlayPause,
    MediaPlaying(bool),
}
