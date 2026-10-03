/// Reemplaza los eventos de Tauri (`emit`/`listen`). Los hilos de fondo
/// (MPRIS, atajos globales, bus de GStreamer) mandan por este canal; el loop
/// de `App::update()` los consume con `try_recv()` al inicio de cada frame.
#[derive(Debug, Clone)]
pub enum AppEvent {
    MediaPrev,
    MediaNext,
    MediaPlayPause,
    MediaPlaying(bool),
}

/// Emisor de eventos para hilos de fondo: entrega el evento y despierta a la UI,
/// que en reposo no se repinta sola.
#[derive(Clone)]
pub struct EventSender {
    tx: std::sync::mpsc::Sender<AppEvent>,
    ctx: eframe::egui::Context,
}

impl EventSender {
    pub fn new(tx: std::sync::mpsc::Sender<AppEvent>, ctx: eframe::egui::Context) -> Self {
        Self { tx, ctx }
    }

    pub fn send(&self, event: AppEvent) -> Result<(), std::sync::mpsc::SendError<AppEvent>> {
        let result = self.tx.send(event);
        self.ctx.request_repaint();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{mpsc, Arc};

    #[test]
    fn send_entrega_el_evento_y_despierta_la_ui() {
        let ctx = eframe::egui::Context::default();
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&wakes);
        ctx.set_request_repaint_callback(move |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        let (tx, rx) = mpsc::channel();
        let sender = EventSender::new(tx, ctx);

        sender.send(AppEvent::MediaNext).unwrap();

        assert!(matches!(rx.try_recv(), Ok(AppEvent::MediaNext)));
        assert!(wakes.load(Ordering::SeqCst) >= 1, "la UI debía despertarse");
    }
}
