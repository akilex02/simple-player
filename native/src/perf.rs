use eframe::egui;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Ventana deslizante de muestras con percentiles por rango más cercano.
pub struct Samples {
    buf: VecDeque<f64>,
    cap: usize,
}

impl Samples {
    pub fn new(cap: usize) -> Self {
        Self { buf: VecDeque::new(), cap }
    }

    pub fn push(&mut self, v: f64) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(v);
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn percentile(&self, p: f64) -> f64 {
        nearest_rank(self.buf.iter().copied().collect(), p)
    }

    /// p95 de la desviación absoluta respecto a la mediana.
    pub fn jitter_p95(&self) -> f64 {
        if self.buf.is_empty() {
            return 0.0;
        }
        let median = median(self.buf.iter().copied().collect());
        let deviations = self.buf.iter().map(|v| (v - median).abs()).collect();
        nearest_rank(deviations, 0.95)
    }
}

fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(|a, b| a.total_cmp(b));
    let n = values.len();
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

fn nearest_rank(mut values: Vec<f64>, p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let rank = (p * values.len() as f64).ceil() as usize;
    values[rank.clamp(1, values.len()) - 1]
}

/// Proporción de eventos verdaderos en una ventana deslizante.
pub struct FlagRatio {
    buf: VecDeque<bool>,
    cap: usize,
}

impl FlagRatio {
    pub fn new(cap: usize) -> Self {
        Self { buf: VecDeque::new(), cap }
    }

    pub fn push(&mut self, v: bool) {
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(v);
    }

    pub fn ratio(&self) -> f64 {
        if self.buf.is_empty() {
            return 0.0;
        }
        self.buf.iter().filter(|v| **v).count() as f64 / self.buf.len() as f64
    }
}

pub struct PerfHud {
    pub visible: bool,
    frame_cpu_ms: Samples,
    frame_interval_ms: Samples,
    spectrum_interval_ms: Samples,
    tick_ms: Samples,
    spectrum_lead_ms: Samples,
    underrun: FlagRatio,
    frame_start: Option<Instant>,
    last_frame_start: Option<Instant>,
    last_spectrum_at: Option<Instant>,
}

impl PerfHud {
    pub fn new() -> Self {
        Self {
            visible: false,
            frame_cpu_ms: Samples::new(600),
            frame_interval_ms: Samples::new(600),
            spectrum_interval_ms: Samples::new(200),
            tick_ms: Samples::new(600),
            spectrum_lead_ms: Samples::new(600),
            underrun: FlagRatio::new(600),
            frame_start: None,
            last_frame_start: None,
            last_spectrum_at: None,
        }
    }

    pub fn on_spectrum(&mut self, now: Instant) {
        if let Some(prev) = self.last_spectrum_at {
            self.spectrum_interval_ms.push(now.duration_since(prev).as_secs_f64() * 1000.0);
        }
        self.last_spectrum_at = Some(now);
    }

    pub fn begin_frame(&mut self, now: Instant) {
        if let Some(prev) = self.last_frame_start {
            self.frame_interval_ms.push(now.duration_since(prev).as_secs_f64() * 1000.0);
        }
        self.last_frame_start = Some(now);
        self.frame_start = Some(now);
    }

    pub fn end_frame(&mut self, now: Instant) {
        if let Some(start) = self.frame_start.take() {
            self.frame_cpu_ms.push(now.duration_since(start).as_secs_f64() * 1000.0);
        }
    }

    pub fn record_tick(&mut self, elapsed: Duration) {
        self.tick_ms.push(elapsed.as_secs_f64() * 1000.0);
    }

    /// Cuánto va el último frame de espectro por delante del reloj (negativo = atrasado).
    pub fn record_spectrum_lead(&mut self, lead_ms: f64) {
        self.spectrum_lead_ms.push(lead_ms);
    }

    pub fn record_underrun(&mut self, underrun: bool) {
        self.underrun.push(underrun);
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn show(&self, ctx: &egui::Context, latency_ms: f64) {
        if !self.visible {
            return;
        }
        let age = self
            .last_spectrum_at
            .map(|t| t.elapsed().as_secs_f64() * 1000.0)
            .unwrap_or(0.0);
        let row = |label: &str, s: &Samples| {
            format!(
                "{label:<18} p50 {:>6.2}  p95 {:>6.2}  p99 {:>6.2} ms",
                s.percentile(0.5),
                s.percentile(0.95),
                s.percentile(0.99)
            )
        };
        let lines = [
            "Rendimiento (F3 para ocultar)".to_string(),
            row("CPU por frame", &self.frame_cpu_ms),
            row("Intervalo frames", &self.frame_interval_ms),
            row("Intervalo espectro", &self.spectrum_interval_ms),
            format!("Jitter espectro p95 {:.2} ms", self.spectrum_jitter_p95()),
            format!("Edad último espectro {age:.1} ms"),
            row("Espectro adelantado", &self.spectrum_lead_ms),
            format!("Viz sin datos nuevos {:.0} % (underrun)", self.underrun.ratio() * 100.0),
            format!("Compensación latencia {latency_ms:+.0} ms (F4 -10 / F5 +10)"),
            row("state.tick()", &self.tick_ms),
        ];
        egui::Area::new(egui::Id::new("perf_hud"))
            .fixed_pos(egui::pos2(8.0, 8.0))
            .order(egui::Order::Debug)
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_black_alpha(190))
                    .rounding(6.0)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        for line in lines {
                            ui.label(
                                egui::RichText::new(line)
                                    .monospace()
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(0x66, 0xff, 0x99)),
                            );
                        }
                    });
            });
    }

    #[cfg(test)]
    pub fn spectrum_interval_samples(&self) -> usize {
        self.spectrum_interval_ms.len()
    }

    #[cfg(test)]
    pub fn spectrum_interval_p50(&self) -> f64 {
        self.spectrum_interval_ms.percentile(0.5)
    }

    pub fn spectrum_jitter_p95(&self) -> f64 {
        self.spectrum_interval_ms.jitter_p95()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn filled(values: impl IntoIterator<Item = f64>, cap: usize) -> Samples {
        let mut s = Samples::new(cap);
        for v in values {
            s.push(v);
        }
        s
    }

    #[test]
    fn percentiles_por_rango_mas_cercano() {
        let s = filled((1..=100).map(|v| v as f64), 100);
        assert_eq!(s.percentile(0.50), 50.0);
        assert_eq!(s.percentile(0.95), 95.0);
        assert_eq!(s.percentile(0.99), 99.0);
    }

    #[test]
    fn ventana_descarta_las_muestras_mas_antiguas() {
        let s = filled((1..=10).map(|v| v as f64), 4);
        assert_eq!(s.len(), 4);
        assert_eq!(s.percentile(0.0), 7.0);
        assert_eq!(s.percentile(1.0), 10.0);
    }

    #[test]
    fn sin_muestras_devuelve_cero() {
        let s = Samples::new(8);
        assert_eq!(s.percentile(0.95), 0.0);
        assert_eq!(s.jitter_p95(), 0.0);
    }

    #[test]
    fn jitter_mide_desviacion_respecto_a_la_mediana() {
        let s = filled([40.0, 60.0].into_iter().cycle().take(10), 16);
        assert_eq!(s.jitter_p95(), 10.0);
    }

    #[test]
    fn jitter_es_cero_con_intervalo_constante() {
        let s = filled(std::iter::repeat(50.0).take(10), 16);
        assert_eq!(s.jitter_p95(), 0.0);
    }

    #[test]
    fn ratio_cuenta_solo_la_ventana() {
        let mut r = FlagRatio::new(4);
        for v in [true, true, true, true, false, false] {
            r.push(v);
        }
        assert_eq!(r.ratio(), 0.5);
    }

    #[test]
    fn ratio_vacio_es_cero() {
        assert_eq!(FlagRatio::new(4).ratio(), 0.0);
    }

    #[test]
    fn hud_registra_intervalos_entre_mensajes_de_espectro() {
        let mut hud = PerfHud::new();
        let t0 = Instant::now();
        hud.on_spectrum(t0);
        assert_eq!(hud.spectrum_interval_samples(), 0);
        hud.on_spectrum(t0 + Duration::from_millis(50));
        hud.on_spectrum(t0 + Duration::from_millis(100));
        hud.on_spectrum(t0 + Duration::from_millis(170));
        assert_eq!(hud.spectrum_interval_samples(), 3);
        assert_eq!(hud.spectrum_interval_p50(), 50.0);
        assert_eq!(hud.spectrum_jitter_p95(), 20.0);
    }
}
