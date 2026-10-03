pub mod bands;
pub mod engine;

use std::time::Instant;

/// Un mensaje del elemento `spectrum`, ya con su tiempo de flujo.
#[derive(Clone, Debug)]
pub struct SpectrumFrame {
    /// `stream-time` del mensaje, en segundos.
    pub time: f64,
    /// Magnitudes en dB, bandas lineales en frecuencia.
    pub bands: Vec<f32>,
    /// Cuándo llegó a la app (solo para el HUD).
    pub arrived: Instant,
}

/// Lo que dibujan los visualizadores: barras normalizadas 0..1 y sus picos.
#[derive(Clone, Debug, Default)]
pub struct VizFrame {
    pub bars: Vec<f32>,
    pub peaks: Vec<f32>,
}
