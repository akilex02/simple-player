use super::model::SongSnapshot;
use super::service::StatsHandle;
use super::session::SessionTracker;
use std::time::Instant;

pub fn epoch_ms_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// Une el tracker de sesión con el servicio: lo que `AppState` llama.
pub struct StatsRecorder {
    tracker: SessionTracker,
    handle: StatsHandle,
}

impl StatsRecorder {
    pub fn new(handle: StatsHandle) -> Self {
        Self { tracker: SessionTracker::new(), handle }
    }

    /// ¿Hay una sesión abierta? Falta cuando suena la cola restaurada al arrancar sin pasar por `play_index`.
    pub fn has_session(&self) -> bool {
        self.tracker.is_active()
    }

    pub fn handle(&self) -> &StatsHandle {
        &self.handle
    }

    pub fn song_started(&mut self, song: SongSnapshot, playing: bool) {
        self.song_started_at(song, playing, Instant::now(), epoch_ms_now());
    }

    pub fn finish(&mut self) {
        self.finish_at(Instant::now(), epoch_ms_now());
    }

    pub fn song_started_at(&mut self, song: SongSnapshot, playing: bool, now: Instant, epoch_ms: i64) {
        if let Some(event) = self.tracker.start(song, now, epoch_ms, playing) {
            self.handle.record(event);
        }
    }

    pub fn observe_at(&mut self, playing: bool, now: Instant, epoch_ms: i64) {
        self.tracker.observe(playing, now, epoch_ms);
    }

    pub fn finish_at(&mut self, now: Instant, epoch_ms: i64) {
        if let Some(event) = self.tracker.finish(now, epoch_ms) {
            self.handle.record(event);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::location::StatsLocation;
    use crate::stats::model::StatsRange;
    use std::sync::Arc;
    use std::time::Duration;

    fn song(path: &str) -> SongSnapshot {
        SongSnapshot { path: path.into(), title: path.into(), artist: "A".into(), album: "B".into(), duration_ms: 200_000 }
    }

    fn recorder() -> StatsRecorder {
        StatsRecorder::new(StatsHandle::spawn(StatsLocation::Memory, Arc::new(|| {})))
    }

    fn plays(r: &StatsRecorder) -> u32 {
        r.handle().request_summary(StatsRange::All);
        r.handle().flush(Duration::from_secs(2));
        r.handle().latest_summary().map(|(_, s)| s.totals.plays).unwrap_or(0)
    }

    const E0: i64 = 1_700_000_000_000;

    #[test]
    fn cambiar_de_cancion_guarda_la_anterior_si_paso_de_5_segundos() {
        let t0 = Instant::now();
        let mut r = recorder();
        r.song_started_at(song("a"), true, t0, E0);
        r.song_started_at(song("b"), true, t0 + Duration::from_secs(30), E0 + 30_000);
        assert_eq!(plays(&r), 1);
    }

    #[test]
    fn una_cancion_saltada_antes_de_5_segundos_no_se_guarda() {
        let t0 = Instant::now();
        let mut r = recorder();
        r.song_started_at(song("a"), true, t0, E0);
        r.song_started_at(song("b"), true, t0 + Duration::from_secs(3), E0 + 3_000);
        assert_eq!(plays(&r), 0);
    }

    #[test]
    fn cerrar_la_app_guarda_la_cancion_en_curso() {
        let t0 = Instant::now();
        let mut r = recorder();
        r.song_started_at(song("a"), true, t0, E0);
        r.observe_at(true, t0 + Duration::from_secs(20), E0 + 20_000);
        r.finish_at(t0 + Duration::from_secs(20), E0 + 20_000);
        assert_eq!(plays(&r), 1);
    }

    #[test]
    fn un_recorder_con_el_servicio_deshabilitado_no_falla() {
        let t0 = Instant::now();
        let mut r = StatsRecorder::new(StatsHandle::disabled("prueba"));
        r.song_started_at(song("a"), true, t0, E0);
        r.finish_at(t0 + Duration::from_secs(30), E0 + 30_000);
        assert_eq!(plays(&r), 0);
    }

    #[test]
    fn el_reloj_real_devuelve_una_fecha_razonable() {
        assert!(epoch_ms_now() > 1_700_000_000_000);
    }
}
