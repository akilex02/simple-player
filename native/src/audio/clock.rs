use std::time::Instant;

/// Posición de reproducción estimada entre consultas al reproductor: avanza
/// con el reloj monotónico del sistema y se corrige con `resync`.
pub struct PlaybackClock {
    base_pos: f64,
    base_at: Instant,
    playing: bool,
}

/// Diferencia a partir de la cual se asume un salto (seek, cambio de pista)
/// y se salta directo a la posición real en vez de corregir suavemente.
const SNAP_THRESHOLD_SECS: f64 = 0.25;
/// Fracción del error que se corrige en cada resync suave.
const BLEND: f64 = 0.2;

impl PlaybackClock {
    pub fn new(now: Instant) -> Self {
        Self { base_pos: 0.0, base_at: now, playing: false }
    }

    pub fn now(&self, at: Instant) -> f64 {
        if self.playing {
            self.base_pos + at.saturating_duration_since(self.base_at).as_secs_f64()
        } else {
            self.base_pos
        }
    }

    pub fn set_playing(&mut self, playing: bool, at: Instant) {
        if playing == self.playing {
            return;
        }
        self.base_pos = self.now(at);
        self.base_at = at;
        self.playing = playing;
    }

    pub fn resync(&mut self, actual_pos: f64, at: Instant) {
        let estimated = self.now(at);
        let error = actual_pos - estimated;
        self.base_pos = if error.abs() > SNAP_THRESHOLD_SECS { actual_pos } else { estimated + error * BLEND };
        self.base_at = at;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ms(t0: Instant, n: u64) -> Instant {
        t0 + Duration::from_millis(n)
    }

    #[test]
    fn en_pausa_la_posicion_no_avanza() {
        let t0 = Instant::now();
        let mut c = PlaybackClock::new(t0);
        c.resync(10.0, t0);
        assert_eq!(c.now(ms(t0, 500)), 10.0);
    }

    #[test]
    fn reproduciendo_avanza_con_el_reloj_del_sistema() {
        let t0 = Instant::now();
        let mut c = PlaybackClock::new(t0);
        c.resync(10.0, t0);
        c.set_playing(true, t0);
        assert!((c.now(ms(t0, 500)) - 10.5).abs() < 1e-9);
    }

    #[test]
    fn pausar_congela_la_posicion_en_ese_instante() {
        let t0 = Instant::now();
        let mut c = PlaybackClock::new(t0);
        c.resync(10.0, t0);
        c.set_playing(true, t0);
        c.set_playing(false, ms(t0, 300));
        assert!((c.now(ms(t0, 900)) - 10.3).abs() < 1e-9);
    }

    #[test]
    fn un_salto_grande_se_adopta_de_inmediato() {
        let t0 = Instant::now();
        let mut c = PlaybackClock::new(t0);
        c.resync(10.0, t0);
        c.set_playing(true, t0);
        c.resync(60.0, ms(t0, 100));
        assert!((c.now(ms(t0, 100)) - 60.0).abs() < 1e-9);
    }

    #[test]
    fn un_error_pequeno_se_corrige_gradualmente() {
        let t0 = Instant::now();
        let mut c = PlaybackClock::new(t0);
        c.resync(10.0, t0);
        c.set_playing(true, t0);
        // estimado a los 100 ms: 10.1; real: 10.15 → error 0.05
        c.resync(10.15, ms(t0, 100));
        let after = c.now(ms(t0, 100));
        assert!(after > 10.1 && after < 10.15, "corrección parcial, obtuvo {after}");
    }
}
