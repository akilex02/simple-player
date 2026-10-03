use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_player as gst_player;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Wrapper sobre `gst_player::Player`, compartido entre el loop principal de
/// egui, el hilo de atajos globales y el hilo de MPRIS — mismo patrón que
/// `GstState` en la versión Tauri.
#[derive(Clone)]
pub struct AudioPlayer {
    pub inner: Arc<Mutex<gst_player::Player>>,
}

impl AudioPlayer {
    pub fn play(&self, song_path: &str) -> Result<(), String> {
        let player = self.inner.lock().map_err(|e| e.to_string())?;
        let uri = path_to_uri(song_path);
        player.set_uri(Some(uri.as_str()));
        player.play();
        Ok(())
    }

    pub fn pause(&self) -> Result<(), String> {
        self.inner.lock().map_err(|e| e.to_string())?.pause();
        Ok(())
    }

    pub fn resume(&self) -> Result<(), String> {
        self.inner.lock().map_err(|e| e.to_string())?.play();
        Ok(())
    }

    /// Seek nativo de GStreamer — respuesta acelerada por hardware, 0ms.
    pub fn seek(&self, position_secs: f64) -> Result<(), String> {
        let player = self.inner.lock().map_err(|e| e.to_string())?;
        let nanos = (position_secs.max(0.0) * 1_000_000_000.0) as u64;
        player.seek(gst::ClockTime::from_nseconds(nanos));
        Ok(())
    }

    pub fn set_volume(&self, volume: f64) -> Result<(), String> {
        self.inner
            .lock()
            .map_err(|e| e.to_string())?
            .set_volume(volume.clamp(0.0, 1.0));
        Ok(())
    }

    pub fn set_normalization(&self, enabled: bool) -> Result<(), String> {
        let player = self.inner.lock().map_err(|e| e.to_string())?;
        let pipeline = player.pipeline();
        match build_audio_filter_bin(enabled) {
            Some(filter) => pipeline.set_property("audio-filter", &filter),
            None => {
                let null_elem: Option<&gst::Element> = None;
                pipeline.set_property("audio-filter", null_elem);
            }
        }
        Ok(())
    }

    /// Posición con resolución de nanosegundos, sin bloquear: `None` si otro
    /// hilo (MPRIS, atajos) tiene el lock o no hay posición todavía.
    pub fn try_position_secs(&self) -> Option<f64> {
        let player = self.inner.try_lock().ok()?;
        Some(player.position()?.nseconds() as f64 / 1e9)
    }

    /// Posición de reproducción actual en segundos.
    pub fn position_secs(&self) -> u64 {
        if let Ok(player) = self.inner.lock() {
            if let Some(pos) = player.position() {
                return pos.seconds();
            }
        }
        0
    }
}

/// Convert a filesystem path to a valid percent-encoded GStreamer URI (file:///...)
fn path_to_uri(path: &str) -> String {
    if path.starts_with("file://") || path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        gst::glib::filename_to_uri(path, None)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| format!("file://{}", path))
    }
}

/// Construye el bin de `audio-filter`: siempre incluye `spectrum` (analizador FFT
/// para el visualizador) y, si se pide y el plugin de replaygain existe, lo
/// encadena con `rgvolume ! rglimiter` para la normalización de volumen.
pub fn build_audio_filter_bin(normalize: bool) -> Option<gst::Bin> {
    let has_spectrum = gst::ElementFactory::find("spectrum").is_some();
    let has_replaygain = normalize
        && gst::ElementFactory::find("rgvolume").is_some()
        && gst::ElementFactory::find("rglimiter").is_some();

    // 1024 bandas (~21 Hz cada una) para tener resolución en graves; la UI las
    // agrupa en barras logarítmicas. 25 ms = 40 mensajes/seg, que el visualizador
    // interpola según su `stream-time`.
    let spectrum_desc = "spectrum name=spectrum bands=1024 interval=25000000 message-magnitude=true threshold=-60";

    let desc = match (has_spectrum, has_replaygain) {
        (true, true) => format!("{spectrum_desc} ! rgvolume fallback-gain=0.0 ! rglimiter"),
        (true, false) => spectrum_desc.to_string(),
        (false, true) => "rgvolume fallback-gain=0.0 ! rglimiter".to_string(),
        (false, false) => return None,
    };

    gst::parse::bin_from_description(&desc, true).ok()
}

/// Por defecto gst-player salta al keyframe más cercano: en algunos formatos
/// eso desvía el seek hasta ~1 s. Las letras necesitan el instante exacto.
fn enable_accurate_seek(player: &gst_player::Player) {
    let mut config = player.config();
    config.set_seek_accurate(true);
    let _ = player.set_config(config);
}

/// Inicializa GStreamer, crea el `Player`, selecciona el audio-sink que de
/// verdad puede abrir el dispositivo, e instala el audio-filter inicial
/// (spectrum + normalización). Sin los env vars de WebKit — no aplican sin WebView.
pub fn init() -> AudioPlayer {
    // Configure GStreamer plugin search paths
    let candidate_paths = [
        "/usr/lib/gstreamer-1.0",
        "/usr/lib64/gstreamer-1.0",
        "/usr/lib/x86_64-linux-gnu/gstreamer-1.0",
        "/usr/local/lib/gstreamer-1.0",
    ];
    let existing_paths: Vec<String> = candidate_paths
        .iter()
        .filter(|p| Path::new(p).exists())
        .map(|s| s.to_string())
        .collect();

    if !existing_paths.is_empty() {
        let current_path = std::env::var("GST_PLUGIN_PATH_1_0").unwrap_or_default();
        let new_path = if current_path.is_empty() {
            existing_paths.join(":")
        } else {
            format!("{}:{}", current_path, existing_paths.join(":"))
        };
        std::env::set_var("GST_PLUGIN_PATH_1_0", new_path);
    }

    gst::init().expect("No se pudo inicializar GStreamer");

    let registry = gst::Registry::get();
    for dir in &existing_paths {
        let _ = registry.scan_path(dir);
    }

    let dispatcher = gst_player::PlayerGMainContextSignalDispatcher::new(None);
    let player = gst_player::Player::new(None::<gst_player::PlayerVideoRenderer>, Some(dispatcher));

    player.set_volume(0.8);
    enable_accurate_seek(&player);
    let pipeline = player.pipeline();

    for sink_name in ["pulsesink", "pipewiresink", "alsasink"] {
        if let Ok(sink) = gst::ElementFactory::make(sink_name).build() {
            if sink.set_state(gst::State::Ready) == Ok(gst::StateChangeSuccess::Success) {
                let _ = sink.set_state(gst::State::Null);
                pipeline.set_property("audio-sink", &sink);
                println!("[GStreamer] Successfully verified and set audio-sink to '{}'", sink_name);
                break;
            } else {
                let _ = sink.set_state(gst::State::Null);
                println!("[GStreamer] Sink '{}' built but failed to reach READY state, trying next...", sink_name);
            }
        }
    }

    player.connect_error(|_, err| {
        eprintln!("[GStreamer Error] {}", err);
    });

    if let Some(filter) = build_audio_filter_bin(true) {
        pipeline.set_property("audio-filter", &filter);
    }

    AudioPlayer {
        inner: Arc::new(Mutex::new(player)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_seek_preciso_queda_activado_en_el_reproductor() {
        gst::init().unwrap();
        let player = gst_player::Player::new(None::<gst_player::PlayerVideoRenderer>, None::<gst_player::PlayerSignalDispatcher>);
        assert!(!player.config().is_seek_accurate(), "por defecto salta a keyframes");
        enable_accurate_seek(&player);
        assert!(player.config().is_seek_accurate());
    }
}
