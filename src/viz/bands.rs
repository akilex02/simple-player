//! Graves, medios y agudos a partir de las barras del visualizador.
use super::engine::smooth;

const BASS_SHARE: f32 = 0.12;
const MID_SHARE: f32 = 0.45;
const ATTACK_TAU: f32 = 0.04;
const RELEASE_TAU: f32 = 0.18;

/// Energía por rango de frecuencia, 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bands {
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
}

/// Medias crudas por rango (sin suavizar): primeras 12 % de barras = graves,
/// siguientes 45 % = medios y el resto = agudos.
pub fn split(bars: &[f32]) -> Bands {
    let n = bars.len();
    if n == 0 {
        return Bands::default();
    }
    let bass_end = ((n as f32 * BASS_SHARE).round() as usize).clamp(1, n);
    let mid_end = (bass_end + (n as f32 * MID_SHARE).round() as usize).clamp(bass_end, n);
    let mean = |s: &[f32]| {
        if s.is_empty() {
            0.0
        } else {
            s.iter().map(|v| v.clamp(0.0, 1.0)).sum::<f32>() / s.len() as f32
        }
    };
    Bands { bass: mean(&bars[..bass_end]), mid: mean(&bars[bass_end..mid_end]), high: mean(&bars[mid_end..]) }
}

/// Suaviza las bandas entre frames (ataque rápido, caída lenta) para que no parpadeen.
pub struct BandAnalyzer {
    current: Bands,
}

impl BandAnalyzer {
    pub fn new() -> Self {
        Self { current: Bands::default() }
    }

    pub fn update(&mut self, bars: &[f32], dt: f32) -> Bands {
        let target = split(bars);
        let step = |current: f32, target: f32| smooth(current, target, dt, ATTACK_TAU, RELEASE_TAU).clamp(0.0, 1.0);
        self.current = Bands {
            bass: step(self.current.bass, target.bass),
            mid: step(self.current.mid, target.mid),
            high: step(self.current.high, target.high),
        };
        self.current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars_with(n: usize, ones: std::ops::Range<usize>) -> Vec<f32> {
        (0..n).map(|i| if ones.contains(&i) { 1.0 } else { 0.0 }).collect()
    }

    #[test]
    fn sin_barras_todo_es_cero() {
        assert_eq!(split(&[]), Bands::default());
    }

    #[test]
    fn el_silencio_da_ceros() {
        assert_eq!(split(&vec![0.0; 100]), Bands::default());
    }

    #[test]
    fn solo_los_graves_llenan_solo_bass() {
        // 100 barras: graves = primeras 12.
        let b = split(&bars_with(100, 0..12));
        assert_eq!(b, Bands { bass: 1.0, mid: 0.0, high: 0.0 });
    }

    #[test]
    fn solo_los_agudos_llenan_solo_high() {
        // medios = barras 12..57; agudos = 57..100.
        let b = split(&bars_with(100, 57..100));
        assert_eq!(b, Bands { bass: 0.0, mid: 0.0, high: 1.0 });
    }

    #[test]
    fn una_sola_barra_no_hace_panico() {
        let b = split(&[1.0]);
        assert_eq!(b.bass, 1.0);
        assert_eq!(b.mid, 0.0);
        assert_eq!(b.high, 0.0);
    }

    #[test]
    fn los_valores_fuera_de_rango_se_acotan() {
        let b = split(&vec![5.0; 100]);
        assert_eq!(b, Bands { bass: 1.0, mid: 1.0, high: 1.0 });
    }

    #[test]
    fn el_ataque_es_mas_rapido_que_la_caida() {
        let loud = vec![1.0; 100];
        let quiet = vec![0.0; 100];
        let rise = BandAnalyzer::new().update(&loud, 0.016).bass;

        let mut analyzer = BandAnalyzer::new();
        for _ in 0..300 {
            analyzer.update(&loud, 0.016);
        }
        let fall = 1.0 - analyzer.update(&quiet, 0.016).bass;
        assert!(rise > fall, "ataque {rise} debe superar caída {fall}");
    }

    #[test]
    fn la_salida_siempre_queda_entre_cero_y_uno() {
        let mut analyzer = BandAnalyzer::new();
        for _ in 0..100 {
            let b = analyzer.update(&vec![9.0; 64], 0.016);
            assert!((0.0..=1.0).contains(&b.bass) && (0.0..=1.0).contains(&b.mid) && (0.0..=1.0).contains(&b.high));
        }
    }
}
