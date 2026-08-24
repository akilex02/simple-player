use crate::events::AppEvent;
use gstreamer as gst;
use std::sync::mpsc::Sender;

/// Reenvía las magnitudes del elemento `spectrum` por el canal de eventos.
/// Usamos un sync handler (no `add_watch`) porque `gst_player::Player` ya
/// instala su propio watch async en este bus para su manejo interno de
/// señales — un bus solo admite UN watch a la vez, así que un segundo
/// `add_watch` fallaría en silencio. El sync handler es un mecanismo
/// aparte (se dispara al momento de publicarse el mensaje, en el hilo
/// que lo publica) hecho justamente para "espiar" sin robarle mensajes
/// a quien ya los consume — por eso siempre devolvemos `Pass`.
pub fn install_spectrum_watch(bus: &gst::Bus, tx: Sender<AppEvent>) {
    bus.set_sync_handler(move |_, msg| {
        if let gst::MessageView::Element(elem) = msg.view() {
            if let Some(s) = elem.structure() {
                if s.name() == "spectrum" {
                    if let Ok(magnitude) = s.get::<gst::List>("magnitude") {
                        let values: Vec<f32> = magnitude
                            .iter()
                            .filter_map(|v| v.get::<f32>().ok())
                            .collect();
                        let _ = tx.send(AppEvent::Spectrum(values));
                    }
                }
            }
        }
        gst::BusSyncReply::Pass
    });
}
