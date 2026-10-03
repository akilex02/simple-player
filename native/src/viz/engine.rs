use super::{SpectrumFrame, VizFrame};
use std::collections::VecDeque;
use std::time::Instant;

pub const BAR_COUNT: usize = 48;
const HISTORY_CAP: usize = 64;
const MAX_GAP_SECS: f64 = 0.25;
const NYQUIST_HZ: f32 = 22050.0;
const F_LO: f32 = 40.0;
const F_HI: f32 = 16000.0;
const ATTACK_TAU: f32 = 0.03;
const RELEASE_TAU: f32 = 0.15;
const MIN_DB: f32 = -60.0;
const REWIND_TOLERANCE_SECS: f64 = 0.05;
const PEAK_HOLD_SECS: f32 = 0.35;
const PEAK_FALL_PER_SEC: f32 = 0.9;
const TILT_DB_PER_OCTAVE: f32 = 3.0;
const TILT_REF_HZ: f32 = 500.0;
const AGC_FLOOR_DB: f32 = -40.0;
const AGC_RELEASE_DB_PER_SEC: f32 = 3.0;
const AGC_HEADROOM_DB: f32 = 6.0;
const RANGE_DB: f32 = 50.0;

pub struct Sampled {
    pub bands: Vec<f32>,
    /// `t` quedó más allá del último frame recibido: se está sosteniendo el último.
    pub underrun: bool,
}

/// Bandas interpoladas en el instante `t`, o `None` si no hay historial.
pub fn interpolate(history: &VecDeque<SpectrumFrame>, t: f64) -> Option<Sampled> {
    let first = history.front()?;
    let last = history.back()?;
    if t <= first.time {
        return Some(Sampled { bands: first.bands.clone(), underrun: false });
    }
    if t >= last.time {
        return Some(Sampled { bands: last.bands.clone(), underrun: t > last.time });
    }
    let i = history.iter().position(|f| f.time > t)? - 1;
    let (a, b) = (&history[i], &history[i + 1]);
    let gap = b.time - a.time;
    if gap > MAX_GAP_SECS {
        return Some(Sampled { bands: a.bands.clone(), underrun: false });
    }
    let alpha = ((t - a.time) / gap) as f32;
    let bands = a.bands.iter().zip(&b.bands).map(|(x, y)| x + (y - x) * alpha).collect();
    Some(Sampled { bands, underrun: false })
}

/// Agrupa bandas lineales en `n_out` barras con escala logarítmica entre `f_lo` y `f_hi`.
pub fn rebin_log(bands: &[f32], n_out: usize, f_lo: f32, f_hi: f32, nyquist: f32) -> Vec<f32> {
    if bands.is_empty() {
        return vec![MIN_DB; n_out];
    }
    let bin_w = nyquist / bands.len() as f32;
    let last = bands.len() - 1;
    let edge = |i: usize| f_lo * (f_hi / f_lo).powf(i as f32 / n_out as f32);
    (0..n_out)
        .map(|i| {
            let lo = edge(i) / bin_w - 0.5;
            let hi = edge(i + 1) / bin_w - 0.5;
            if hi - lo < 1.0 {
                let x = ((lo + hi) / 2.0).clamp(0.0, last as f32);
                let (k, frac) = (x.floor() as usize, x.fract());
                let next = (k + 1).min(last);
                bands[k] + (bands[next] - bands[k]) * frac
            } else {
                let k0 = lo.ceil().max(0.0) as usize;
                let k1 = (hi.floor() as usize).min(last);
                bands[k0..=k1.max(k0)].iter().cloned().fold(MIN_DB, f32::max)
            }
        })
        .collect()
}

/// Suavizado exponencial con ataque rápido y caída lenta, independiente del `dt`.
pub fn smooth(current: f32, target: f32, dt: f32, attack_tau: f32, release_tau: f32) -> f32 {
    let tau = if target > current { attack_tau } else { release_tau };
    current + (target - current) * (1.0 - (-dt / tau).exp())
}

pub struct PeakHold {
    pub value: f32,
    hold_left: f32,
}

impl PeakHold {
    pub fn new() -> Self {
        Self { value: 0.0, hold_left: 0.0 }
    }

    pub fn update(&mut self, v: f32, dt: f32) -> f32 {
        if v >= self.value {
            self.value = v;
            self.hold_left = PEAK_HOLD_SECS;
        } else if self.hold_left > 0.0 {
            self.hold_left = (self.hold_left - dt).max(0.0);
        } else {
            self.value = (self.value - PEAK_FALL_PER_SEC * dt).max(v);
        }
        self.value
    }
}

pub struct VizEngine {
    history: VecDeque<SpectrumFrame>,
    frame: VizFrame,
    peaks: Vec<PeakHold>,
    ref_db: f32,
    underrun: bool,
}

impl VizEngine {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            frame: VizFrame { bars: vec![0.0; BAR_COUNT], peaks: vec![0.0; BAR_COUNT] },
            peaks: (0..BAR_COUNT).map(|_| PeakHold::new()).collect(),
            ref_db: AGC_FLOOR_DB,
            underrun: false,
        }
    }

    pub fn ingest(&mut self, frames: impl IntoIterator<Item = SpectrumFrame>) {
        for frame in frames {
            if self.history.back().is_some_and(|last| frame.time < last.time - REWIND_TOLERANCE_SECS) {
                self.history.clear();
            }
            self.history.push_back(frame);
            if self.history.len() > HISTORY_CAP {
                self.history.pop_front();
            }
        }
    }

    pub fn update(&mut self, t: f64, dt: f32, playing: bool) -> &VizFrame {
        let targets = match playing.then(|| interpolate(&self.history, t)).flatten() {
            Some(sampled) => {
                self.underrun = sampled.underrun;
                self.normalize(&sampled.bands, dt)
            }
            None => {
                self.underrun = false;
                vec![0.0; BAR_COUNT]
            }
        };
        for (i, target) in targets.into_iter().enumerate() {
            let bar = smooth(self.frame.bars[i], target, dt, ATTACK_TAU, RELEASE_TAU);
            self.frame.bars[i] = bar;
            self.frame.peaks[i] = self.peaks[i].update(bar, dt);
        }
        &self.frame
    }

    /// Re-binning logarítmico, inclinación espectral y normalización adaptativa → 0..1.
    fn normalize(&mut self, bands: &[f32], dt: f32) -> Vec<f32> {
        let mut db = rebin_log(bands, BAR_COUNT, F_LO, F_HI, NYQUIST_HZ);
        let ratio = F_HI / F_LO;
        for (i, v) in db.iter_mut().enumerate() {
            let center = F_LO * ratio.powf((i as f32 + 0.5) / BAR_COUNT as f32);
            *v += TILT_DB_PER_OCTAVE * (center / TILT_REF_HZ).log2();
        }
        let max = db.iter().cloned().fold(MIN_DB, f32::max);
        self.ref_db = if max > self.ref_db { max } else { (self.ref_db - AGC_RELEASE_DB_PER_SEC * dt).max(max) };
        self.ref_db = self.ref_db.max(AGC_FLOOR_DB);
        let floor = self.ref_db + AGC_HEADROOM_DB - RANGE_DB;
        db.iter().map(|v| ((v - floor) / RANGE_DB).clamp(0.0, 1.0)).collect()
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn newest_time(&self) -> Option<f64> {
        self.history.back().map(|f| f.time)
    }

    pub fn underrun(&self) -> bool {
        self.underrun
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(time: f64, bands: Vec<f32>) -> SpectrumFrame {
        SpectrumFrame { time, bands, arrived: Instant::now() }
    }

    fn hist(frames: Vec<SpectrumFrame>) -> VecDeque<SpectrumFrame> {
        frames.into_iter().collect()
    }

    // ── interpolate ────────────────────────────────────────────────────────
    #[test]
    fn interpola_linealmente_entre_dos_frames() {
        let h = hist(vec![frame(1.0, vec![-60.0, -20.0]), frame(1.1, vec![-40.0, 0.0])]);
        let s = interpolate(&h, 1.05).unwrap();
        assert!((s.bands[0] + 50.0).abs() < 1e-3 && (s.bands[1] + 10.0).abs() < 1e-3);
        assert!(!s.underrun);
    }

    #[test]
    fn antes_del_primer_frame_usa_el_primero() {
        let h = hist(vec![frame(1.0, vec![-30.0]), frame(1.1, vec![-10.0])]);
        let s = interpolate(&h, 0.5).unwrap();
        assert_eq!(s.bands, vec![-30.0]);
        assert!(!s.underrun);
    }

    #[test]
    fn mas_alla_del_ultimo_frame_sostiene_el_ultimo_y_marca_underrun() {
        let h = hist(vec![frame(1.0, vec![-30.0]), frame(1.1, vec![-10.0])]);
        let s = interpolate(&h, 1.5).unwrap();
        assert_eq!(s.bands, vec![-10.0]);
        assert!(s.underrun);
    }

    #[test]
    fn un_hueco_grande_sostiene_el_frame_anterior_sin_interpolar() {
        let h = hist(vec![frame(1.0, vec![-50.0]), frame(2.0, vec![0.0])]);
        let s = interpolate(&h, 1.5).unwrap();
        assert_eq!(s.bands, vec![-50.0]);
    }

    #[test]
    fn sin_historial_no_hay_muestra() {
        assert!(interpolate(&VecDeque::new(), 1.0).is_none());
    }

    // ── ingest ─────────────────────────────────────────────────────────────
    #[test]
    fn un_salto_hacia_atras_en_el_tiempo_descarta_el_historial_viejo() {
        let mut e = VizEngine::new();
        e.ingest([frame(5.0, vec![-10.0]), frame(5.025, vec![-10.0])]);
        e.ingest([frame(1.0, vec![-10.0])]);
        assert_eq!(e.history_len(), 1);
        assert_eq!(e.newest_time(), Some(1.0));
    }

    #[test]
    fn el_historial_esta_acotado() {
        let mut e = VizEngine::new();
        e.ingest((0..200).map(|i| frame(i as f64 * 0.025, vec![-10.0])));
        assert_eq!(e.history_len(), 64);
        assert_eq!(e.newest_time(), Some(199.0 * 0.025));
    }

    // ── rebin_log ──────────────────────────────────────────────────────────
    fn spike_at(hz: f32, n_bins: usize) -> Vec<f32> {
        let bin_w = NYQUIST_HZ / n_bins as f32;
        let mut v = vec![-60.0; n_bins];
        v[(hz / bin_w) as usize] = 0.0;
        v
    }

    fn argmax(v: &[f32]) -> usize {
        v.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).unwrap().0
    }

    #[test]
    fn rebin_devuelve_la_cantidad_pedida() {
        let out = rebin_log(&vec![-60.0; 1024], 48, F_LO, F_HI, NYQUIST_HZ);
        assert_eq!(out.len(), 48);
    }

    #[test]
    fn un_grave_cae_en_las_barras_bajas() {
        let out = rebin_log(&spike_at(100.0, 1024), 48, F_LO, F_HI, NYQUIST_HZ);
        assert!(argmax(&out) < 12, "argmax = {}", argmax(&out));
    }

    #[test]
    fn un_agudo_cae_en_las_barras_altas() {
        let out = rebin_log(&spike_at(10000.0, 1024), 48, F_LO, F_HI, NYQUIST_HZ);
        assert!(argmax(&out) > 36, "argmax = {}", argmax(&out));
        assert_eq!(out[0], -60.0);
    }

    // ── smooth / peaks ─────────────────────────────────────────────────────
    #[test]
    fn el_ataque_es_mas_rapido_que_la_caida() {
        let up = smooth(0.0, 1.0, 0.016, ATTACK_TAU, RELEASE_TAU);
        let down = 1.0 - smooth(1.0, 0.0, 0.016, ATTACK_TAU, RELEASE_TAU);
        assert!(up > 3.0 * down, "up {up} down {down}");
    }

    #[test]
    fn el_suavizado_no_depende_de_como_se_parta_el_dt() {
        let one = smooth(0.0, 1.0, 0.016, ATTACK_TAU, RELEASE_TAU);
        let half = smooth(smooth(0.0, 1.0, 0.008, ATTACK_TAU, RELEASE_TAU), 1.0, 0.008, ATTACK_TAU, RELEASE_TAU);
        assert!((one - half).abs() < 1e-5);
    }

    #[test]
    fn el_pico_se_sostiene_y_luego_cae() {
        let mut p = PeakHold::new();
        p.update(0.8, 0.016);
        assert_eq!(p.update(0.2, 0.2), 0.8);
        for _ in 0..40 {
            p.update(0.2, 0.05);
        }
        assert!(p.value < 0.8 && p.value >= 0.2);
    }

    // ── engine ─────────────────────────────────────────────────────────────
    fn engine_with_flat_signal(db: f32) -> VizEngine {
        let mut e = VizEngine::new();
        e.ingest((0..80).map(|i| frame(i as f64 * 0.025, vec![db; 1024])));
        e
    }

    fn run(e: &mut VizEngine, from: f64, secs: f64, playing: bool) {
        let dt = 0.016_f32;
        let mut t = from;
        while t < from + secs {
            e.update(t, dt, playing);
            t += dt as f64;
        }
    }

    #[test]
    fn con_senal_las_barras_suben() {
        let mut e = engine_with_flat_signal(-10.0);
        run(&mut e, 0.2, 1.0, true);
        let f = e.update(1.2, 0.016, true);
        assert!(f.bars.iter().cloned().fold(0.0, f32::max) > 0.5);
        assert!(f.bars.iter().all(|b| (0.0..=1.0).contains(b)));
    }

    #[test]
    fn en_pausa_las_barras_decaen_a_cero() {
        let mut e = engine_with_flat_signal(-10.0);
        run(&mut e, 0.2, 1.0, true);
        run(&mut e, 1.2, 3.0, false);
        let f = e.update(4.2, 0.016, false);
        assert!(f.bars.iter().all(|b| *b < 0.01), "{:?}", f.bars);
    }

    #[test]
    fn la_normalizacion_adaptativa_iguala_senales_de_distinto_volumen() {
        let mut quiet = engine_with_flat_signal(-50.0);
        let mut loud = engine_with_flat_signal(-5.0);
        run(&mut quiet, 0.2, 1.5, true);
        run(&mut loud, 0.2, 1.5, true);
        let max = |e: &mut VizEngine| e.update(1.7, 0.016, true).bars.iter().cloned().fold(0.0, f32::max);
        let (q, l) = (max(&mut quiet), max(&mut loud));
        assert!(q > 0.7, "quiet {q}");
        assert!((q - l).abs() < 0.08, "quiet {q} loud {l}");
    }

    #[test]
    fn los_picos_nunca_quedan_debajo_de_las_barras() {
        let mut e = engine_with_flat_signal(-10.0);
        let f = e.update(0.5, 0.016, true).clone();
        assert!(f.bars.iter().zip(&f.peaks).all(|(b, p)| p >= b));
    }
}
