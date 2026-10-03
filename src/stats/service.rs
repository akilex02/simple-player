use super::location::StatsLocation;
use super::model::{PlayEvent, StatsRange, StatsSummary};
use super::store::Store;
use super::summary::build_summary;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type Repaint = Arc<dyn Fn() + Send + Sync>;

enum Message {
    Record(PlayEvent),
    RecordBatch(Vec<PlayEvent>),
    Summary(StatsRange),
    Flush(Sender<()>),
}

#[derive(Default)]
struct Shared {
    latest: Mutex<Option<(StatsRange, Arc<StatsSummary>)>>,
    events_version: AtomicU64,
    disabled: Mutex<Option<String>>,
}

/// Puerta de entrada al servicio de estadísticas. Se clona barato; todas las
/// operaciones retornan sin esperar al disco.
#[derive(Clone)]
pub struct StatsHandle {
    tx: Option<Sender<Message>>,
    shared: Arc<Shared>,
}

impl StatsHandle {
    pub fn disabled(reason: &str) -> Self {
        let shared = Arc::new(Shared::default());
        *shared.disabled.lock().unwrap_or_else(|e| e.into_inner()) = Some(reason.to_string());
        Self { tx: None, shared }
    }

    pub fn spawn(location: StatsLocation, repaint: Repaint) -> Self {
        let opened = match &location {
            StatsLocation::File(path) => {
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                Store::open(path)
            }
            StatsLocation::Memory => Store::open_in_memory(),
        };
        let store = match opened {
            Ok(store) => store,
            Err(e) => {
                eprintln!("[aviso] Estadísticas desactivadas: {}", e.0);
                return Self::disabled(&format!("No se pudo abrir la base de estadísticas: {}", e.0));
            }
        };

        let shared = Arc::new(Shared::default());
        let (tx, rx) = mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        let spawned = std::thread::Builder::new()
            .name("stats".into())
            .spawn(move || worker(store, rx, worker_shared, repaint));
        if let Err(e) = spawned {
            return Self::disabled(&format!("No se pudo iniciar el hilo de estadísticas: {e}"));
        }
        Self { tx: Some(tx), shared }
    }

    fn send(&self, message: Message) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(message);
        }
    }

    pub fn record(&self, event: PlayEvent) {
        self.send(Message::Record(event));
    }

    pub fn record_batch(&self, events: Vec<PlayEvent>) {
        self.send(Message::RecordBatch(events));
    }

    pub fn request_summary(&self, range: StatsRange) {
        self.send(Message::Summary(range));
    }

    pub fn latest_summary(&self) -> Option<(StatsRange, Arc<StatsSummary>)> {
        self.shared.latest.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Sube solo cuando se guarda un evento; la pantalla lo observa para volver a pedir el resumen.
    pub fn events_version(&self) -> u64 {
        self.shared.events_version.load(Ordering::SeqCst)
    }

    pub fn disabled_reason(&self) -> Option<String> {
        self.shared.disabled.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.tx.is_some()
    }

    /// Espera (hasta `timeout`) a que el hilo procese todo lo enviado antes.
    pub fn flush(&self, timeout: Duration) {
        let Some(tx) = &self.tx else { return };
        let (done_tx, done_rx) = mpsc::channel();
        if tx.send(Message::Flush(done_tx)).is_ok() {
            let _ = done_rx.recv_timeout(timeout);
        }
    }
}

fn worker(mut store: Store, rx: Receiver<Message>, shared: Arc<Shared>, repaint: Repaint) {
    while let Ok(message) = rx.recv() {
        match message {
            Message::Record(event) => match store.insert(&event) {
                Ok(()) => {
                    shared.events_version.fetch_add(1, Ordering::SeqCst);
                    repaint();
                }
                Err(e) => eprintln!("[stats] no se pudo guardar un evento: {}", e.0),
            },
            Message::RecordBatch(events) => match store.insert_batch(&events) {
                Ok(()) => {
                    shared.events_version.fetch_add(1, Ordering::SeqCst);
                    repaint();
                }
                Err(e) => eprintln!("[stats] no se pudo guardar el lote: {}", e.0),
            },
            Message::Summary(range) => match build_summary(&store, range, &chrono::Local::now()) {
                Ok(summary) => {
                    *shared.latest.lock().unwrap_or_else(|e| e.into_inner()) = Some((range, Arc::new(summary)));
                    repaint();
                }
                Err(e) => eprintln!("[stats] no se pudo calcular el resumen: {}", e.0),
            },
            Message::Flush(done) => {
                let _ = done.send(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::model::SongSnapshot;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn event(path: &str) -> PlayEvent {
        let now = chrono::Local::now().timestamp_millis();
        PlayEvent {
            started_at: now - 70_000,
            ended_at: now - 10_000,
            listened_ms: 60_000,
            song: SongSnapshot { path: path.into(), title: "T".into(), artist: "A".into(), album: "B".into(), duration_ms: 200_000 },
        }
    }

    fn noop() -> Repaint {
        Arc::new(|| {})
    }

    #[test]
    fn un_evento_registrado_aparece_en_el_resumen_tras_el_flush() {
        let h = StatsHandle::spawn(StatsLocation::Memory, noop());
        assert!(h.is_enabled());
        h.record(event("a"));
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_secs(2));
        let (range, summary) = h.latest_summary().expect("hay resumen");
        assert_eq!(range, StatsRange::All);
        assert_eq!(summary.totals.plays, 1);
    }

    #[test]
    fn la_version_de_eventos_sube_con_cada_evento_y_no_con_los_resumenes() {
        let h = StatsHandle::spawn(StatsLocation::Memory, noop());
        assert_eq!(h.events_version(), 0);
        h.record(event("a"));
        h.record_batch(vec![event("b"), event("c")]);
        h.flush(Duration::from_secs(2));
        let after_events = h.events_version();
        assert!(after_events >= 2, "{after_events}");
        h.request_summary(StatsRange::Week);
        h.flush(Duration::from_secs(2));
        assert_eq!(h.events_version(), after_events, "un resumen no debe disparar otra petición");
    }

    #[test]
    fn el_servicio_despierta_la_ui_al_guardar_y_al_calcular() {
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&wakes);
        let h = StatsHandle::spawn(
            StatsLocation::Memory,
            Arc::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }),
        );
        h.record(event("a"));
        h.request_summary(StatsRange::Today);
        h.flush(Duration::from_secs(2));
        assert!(wakes.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn una_ruta_imposible_deja_el_servicio_deshabilitado_sin_entrar_en_panico() {
        let h = StatsHandle::spawn(StatsLocation::File(PathBuf::from("/proc/no-existe/stats.db")), noop());
        assert!(!h.is_enabled());
        assert!(h.disabled_reason().is_some());
        h.record(event("a"));
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_millis(100));
        assert!(h.latest_summary().is_none());
    }

    #[test]
    fn un_handle_deshabilitado_expone_el_motivo() {
        let h = StatsHandle::disabled("prueba");
        assert_eq!(h.disabled_reason().as_deref(), Some("prueba"));
        assert!(!h.is_enabled());
    }

    #[test]
    fn los_datos_en_archivo_sobreviven_a_reabrir() {
        let path = std::env::temp_dir().join(format!("sp_service_{}.db", std::process::id()));
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", path.display()));
        }
        {
            let h = StatsHandle::spawn(StatsLocation::File(path.clone()), noop());
            h.record(event("a"));
            h.flush(Duration::from_secs(2));
        }
        let h = StatsHandle::spawn(StatsLocation::File(path.clone()), noop());
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_secs(2));
        assert_eq!(h.latest_summary().unwrap().1.totals.plays, 1);
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", path.display()));
        }
    }
}
