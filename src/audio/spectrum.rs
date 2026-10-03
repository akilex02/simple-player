use crate::viz::SpectrumFrame;
use gstreamer as gst;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const RING_CAP: usize = 64;

/// Cola entre el hilo de streaming de GStreamer (productor) y el hilo de UI
/// (consumidor). El consumidor usa `try_lock` para no bloquearse nunca.
#[derive(Default)]
pub struct SpectrumRing {
    inner: Mutex<VecDeque<SpectrumFrame>>,
}

impl SpectrumRing {
    pub fn push(&self, frame: SpectrumFrame) {
        if let Ok(mut queue) = self.inner.lock() {
            if queue.len() == RING_CAP {
                queue.pop_front();
            }
            queue.push_back(frame);
        }
    }

    /// Saca todos los frames pendientes; vacío si el productor tiene el lock.
    pub fn try_drain(&self) -> Vec<SpectrumFrame> {
        match self.inner.try_lock() {
            Ok(mut queue) => queue.drain(..).collect(),
            Err(_) => Vec::new(),
        }
    }
}

/// Alimenta `ring` con las magnitudes del elemento `spectrum`.
/// Usamos un sync handler (no `add_watch`) porque `gst_player::Player` ya
/// instala su propio watch async en este bus para su manejo interno de
/// señales — un bus solo admite UN watch a la vez, así que un segundo
/// `add_watch` fallaría en silencio. El sync handler es un mecanismo
/// aparte (se dispara al momento de publicarse el mensaje, en el hilo
/// que lo publica) hecho justamente para "espiar" sin robarle mensajes
/// a quien ya los consume — por eso siempre devolvemos `Pass`.
pub fn install_spectrum_watch(bus: &gst::Bus, ring: Arc<SpectrumRing>) {
    bus.set_sync_handler(move |_, msg| {
        if let gst::MessageView::Element(elem) = msg.view() {
            if let Some(s) = elem.structure() {
                if s.name() == "spectrum" {
                    if let Some(frame) = parse_frame(s) {
                        ring.push(frame);
                    }
                }
            }
        }
        gst::BusSyncReply::Pass
    });
}

fn parse_frame(s: &gst::StructureRef) -> Option<SpectrumFrame> {
    let magnitude = s.get::<gst::List>("magnitude").ok()?;
    let bands = magnitude.iter().filter_map(|v| v.get::<f32>().ok()).collect();
    let time = s
        .get::<gst::ClockTime>("stream-time")
        .or_else(|_| s.get::<gst::ClockTime>("running-time"))
        .ok()?;
    Some(SpectrumFrame { time: time.nseconds() as f64 / 1e9, bands, arrived: Instant::now() })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(time: f64) -> SpectrumFrame {
        SpectrumFrame { time, bands: vec![-10.0], arrived: Instant::now() }
    }

    fn spectrum_structure(stream_time_ms: u64) -> gst::Structure {
        gst::init().unwrap();
        gst::Structure::builder("spectrum")
            .field("stream-time", gst::ClockTime::from_mseconds(stream_time_ms))
            .field("magnitude", gst::List::new([-10.0f32, -20.0f32, -60.0f32]))
            .build()
    }

    #[test]
    fn parse_frame_lee_stream_time_y_magnitudes_de_un_mensaje_real() {
        let frame = parse_frame(&spectrum_structure(1500)).expect("mensaje válido");
        assert_eq!(frame.time, 1.5);
        assert_eq!(frame.bands, vec![-10.0, -20.0, -60.0]);
    }

    #[test]
    fn parse_frame_descarta_mensajes_sin_tiempo() {
        gst::init().unwrap();
        let s = gst::Structure::builder("spectrum").field("magnitude", gst::List::new([-10.0f32])).build();
        assert!(parse_frame(&s).is_none());
    }

    #[test]
    fn drain_devuelve_los_frames_en_orden_y_vacia_la_cola() {
        let ring = SpectrumRing::default();
        ring.push(frame(1.0));
        ring.push(frame(2.0));
        let times: Vec<f64> = ring.try_drain().iter().map(|f| f.time).collect();
        assert_eq!(times, vec![1.0, 2.0]);
        assert!(ring.try_drain().is_empty());
    }

    #[test]
    fn la_cola_descarta_los_mas_antiguos_al_llenarse() {
        let ring = SpectrumRing::default();
        for i in 0..100 {
            ring.push(frame(i as f64));
        }
        let drained = ring.try_drain();
        assert_eq!(drained.len(), 64);
        assert_eq!(drained[0].time, 36.0);
    }

    #[test]
    fn drain_no_bloquea_si_el_productor_tiene_el_lock() {
        let ring = SpectrumRing::default();
        ring.push(frame(1.0));
        let _guard = ring.inner.lock().unwrap();
        assert!(ring.try_drain().is_empty());
    }
}
