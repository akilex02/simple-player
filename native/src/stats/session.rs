use super::model::{PlayEvent, SongSnapshot};
use std::time::Instant;

/// Mínimo de tiempo real escuchado para que una sesión cuente.
pub const MIN_LISTEN_MS: u64 = 5_000;

struct Active {
    song: SongSnapshot,
    started_at: i64,
    listened_ms: u64,
    last_instant: Instant,
    playing: bool,
    last_playing_epoch: i64,
}

/// Mide el tiempo real escuchado de la canción en curso. No tiene reloj
/// propio: el tiempo llega por parámetro, así se prueba sin esperar.
pub struct SessionTracker {
    active: Option<Active>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self { active: None }
    }

    pub fn is_active(&self) -> bool {
        self.active.is_some()
    }

    pub fn start(&mut self, song: SongSnapshot, now: Instant, epoch_ms: i64, playing: bool) -> Option<PlayEvent> {
        let previous = self.finish(now, epoch_ms);
        self.active = Some(Active {
            song,
            started_at: epoch_ms,
            listened_ms: 0,
            last_instant: now,
            playing,
            last_playing_epoch: epoch_ms,
        });
        previous
    }

    pub fn observe(&mut self, playing: bool, now: Instant, epoch_ms: i64) {
        let Some(a) = &mut self.active else { return };
        if a.playing {
            a.listened_ms += elapsed_ms(a.last_instant, now);
            a.last_playing_epoch = epoch_ms;
        }
        a.last_instant = now;
        a.playing = playing;
    }

    pub fn finish(&mut self, now: Instant, epoch_ms: i64) -> Option<PlayEvent> {
        let mut a = self.active.take()?;
        if a.playing {
            a.listened_ms += elapsed_ms(a.last_instant, now);
            a.last_playing_epoch = epoch_ms;
        }
        if a.listened_ms < MIN_LISTEN_MS {
            return None;
        }
        Some(PlayEvent {
            started_at: a.started_at,
            ended_at: a.last_playing_epoch.max(a.started_at),
            listened_ms: a.listened_ms,
            song: a.song,
        })
    }
}

fn elapsed_ms(from: Instant, to: Instant) -> u64 {
    to.saturating_duration_since(from).as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const E0: i64 = 1_700_000_000_000;

    fn song(path: &str) -> SongSnapshot {
        SongSnapshot {
            path: path.into(),
            title: format!("T {path}"),
            artist: "A".into(),
            album: "B".into(),
            duration_ms: 200_000,
        }
    }

    fn at(t0: Instant, ms: u64) -> Instant {
        t0 + Duration::from_millis(ms)
    }

    #[test]
    fn acumula_solo_mientras_suena() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        assert!(t.start(song("a"), t0, E0, true).is_none());
        t.observe(true, at(t0, 4_000), E0 + 4_000);
        t.observe(false, at(t0, 6_000), E0 + 6_000);
        t.observe(false, at(t0, 60_000), E0 + 60_000);
        let e = t.finish(at(t0, 60_000), E0 + 60_000).expect("cuenta: 6 s escuchados");
        assert_eq!(e.listened_ms, 6_000);
        assert_eq!(e.started_at, E0);
    }

    #[test]
    fn ended_at_es_el_ultimo_instante_sonando_si_se_cierra_en_pausa() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        t.observe(false, at(t0, 8_000), E0 + 8_000);
        // la app queda horas en pausa y se cierra mucho después
        let e = t.finish(at(t0, 3 * 3_600_000), E0 + 3 * 3_600_000).unwrap();
        assert_eq!(e.ended_at, E0 + 8_000);
        assert_eq!(e.listened_ms, 8_000);
    }

    #[test]
    fn menos_de_cinco_segundos_no_genera_evento() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        t.observe(true, at(t0, 4_999), E0 + 4_999);
        assert!(t.finish(at(t0, 4_999), E0 + 4_999).is_none());
    }

    #[test]
    fn exactamente_cinco_segundos_si_cuenta() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        let e = t.finish(at(t0, 5_000), E0 + 5_000).expect("5 s justos cuentan");
        assert_eq!(e.listened_ms, 5_000);
    }

    #[test]
    fn pausa_y_reanuda_suma_solo_los_tramos_sonando() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        t.observe(false, at(t0, 4_000), E0 + 4_000); // sonó 4 s
        t.observe(true, at(t0, 10_000), E0 + 10_000); // 6 s en pausa: no suman
        t.observe(true, at(t0, 13_000), E0 + 13_000); // sonó 3 s más
        let e = t.finish(at(t0, 13_000), E0 + 13_000).unwrap();
        assert_eq!(e.listened_ms, 7_000);
        assert_eq!(e.ended_at, E0 + 13_000);
    }

    #[test]
    fn empezar_otra_cancion_devuelve_el_evento_de_la_anterior() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        let prev = t.start(song("b"), at(t0, 8_000), E0 + 8_000, true).expect("la anterior cuenta");
        assert_eq!(prev.song.path, "a");
        assert_eq!(prev.listened_ms, 8_000);
        let next = t.finish(at(t0, 20_000), E0 + 20_000).unwrap();
        assert_eq!(next.song.path, "b");
        assert_eq!(next.started_at, E0 + 8_000);
        assert_eq!(next.listened_ms, 12_000);
    }

    #[test]
    fn repetir_la_misma_cancion_cuenta_como_otra_reproduccion() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        let first = t.start(song("a"), at(t0, 200_000), E0 + 200_000, true).expect("primera vuelta");
        assert_eq!(first.listened_ms, 200_000);
        assert!(t.finish(at(t0, 205_000), E0 + 205_000).is_some());
    }

    #[test]
    fn una_sesion_que_empieza_en_pausa_no_suma_hasta_que_suena() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, false);
        t.observe(false, at(t0, 30_000), E0 + 30_000);
        t.observe(true, at(t0, 31_000), E0 + 31_000);
        t.observe(true, at(t0, 37_000), E0 + 37_000);
        let e = t.finish(at(t0, 37_000), E0 + 37_000).unwrap();
        assert_eq!(e.listened_ms, 6_000);
    }

    #[test]
    fn finish_sin_sesion_no_devuelve_nada_y_finish_es_idempotente() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        assert!(t.finish(t0, E0).is_none());
        t.start(song("a"), t0, E0, true);
        assert!(t.finish(at(t0, 6_000), E0 + 6_000).is_some());
        assert!(t.finish(at(t0, 7_000), E0 + 7_000).is_none());
    }

    #[test]
    fn ended_at_nunca_es_anterior_a_started_at() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        // el reloj del sistema retrocede durante la sesión
        let e = t.finish(at(t0, 6_000), E0 - 10_000).unwrap();
        assert!(e.ended_at >= e.started_at);
    }
}
