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

/// Tras un seek, el reproductor tarda en reportar la posición nueva y sigue
/// informando la anterior unos instantes. Mientras dura esa ventana se
/// muestra la posición esperada (destino + tiempo transcurrido).
pub struct SeekGuard {
    target: f64,
    at: Option<Instant>,
}

const SEEK_SETTLE_SECS: f64 = 0.4;
/// Si lo reportado ya está así de cerca de lo esperado, el seek terminó.
const SEEK_CLOSE_ENOUGH_SECS: f64 = 0.25;

impl SeekGuard {
    pub fn new() -> Self {
        Self { target: 0.0, at: None }
    }

    pub fn on_seek(&mut self, target: f64, now: Instant) {
        self.target = target;
        self.at = Some(now);
    }

    /// Posición que debe mostrarse dado lo que reportó el reproductor.
    pub fn resolve(&mut self, reported: f64, now: Instant) -> f64 {
        let Some(at) = self.at else { return reported };
        let elapsed = now.saturating_duration_since(at).as_secs_f64();
        let expected = self.target + elapsed;
        if elapsed >= SEEK_SETTLE_SECS || (reported - expected).abs() <= SEEK_CLOSE_ENOUGH_SECS {
            self.at = None;
            return reported;
        }
        expected
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

    // ── SeekGuard ──────────────────────────────────────────────────────────
    #[test]
    fn sin_seek_se_confia_en_lo_reportado() {
        let mut g = SeekGuard::new();
        assert_eq!(g.resolve(12.3, Instant::now()), 12.3);
    }

    #[test]
    fn justo_despues_de_un_seek_se_muestra_el_destino_aunque_el_reproductor_diga_otra_cosa() {
        let t0 = Instant::now();
        let mut g = SeekGuard::new();
        g.on_seek(40.0, t0);
        let shown = g.resolve(12.0, ms(t0, 100));
        assert!((shown - 40.1).abs() < 1e-6, "{shown}");
    }

    #[test]
    fn cuando_lo_reportado_alcanza_lo_esperado_se_confia_en_el_reproductor() {
        let t0 = Instant::now();
        let mut g = SeekGuard::new();
        g.on_seek(40.0, t0);
        assert_eq!(g.resolve(40.08, ms(t0, 100)), 40.08);
        // y la guarda queda liberada: un valor lejano ya no se sobrescribe
        assert_eq!(g.resolve(55.0, ms(t0, 150)), 55.0);
    }

    #[test]
    fn pasada_la_ventana_se_confia_en_el_reproductor_aunque_difiera() {
        let t0 = Instant::now();
        let mut g = SeekGuard::new();
        g.on_seek(40.0, t0);
        assert_eq!(g.resolve(12.0, ms(t0, 900)), 12.0);
    }
}
