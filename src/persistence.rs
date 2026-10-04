use crate::paths::{get_playback_position_path, get_playback_state_path};
use serde::{Deserialize, Serialize};
use std::fs;

/// Persisted UI state (folder, queue, current song) saved to disk on each song change
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlaybackState {
    /// Solo se lee, para migrar a `settings.json`; ya no se escribe.
    #[serde(default)]
    pub folder_path: Option<String>,
    pub queue_paths: Vec<String>,
    pub current_index: Option<usize>,
    pub volume: f64,
    pub is_shuffle: bool,
    pub repeat_mode: String,
}

pub fn save_playback_state(state: &PlaybackState) -> Result<(), String> {
    let json = serde_json::to_string(state).map_err(|e| e.to_string())?;
    fs::write(get_playback_state_path(), json).map_err(|e| e.to_string())
}

pub fn load_playback_state() -> Option<PlaybackState> {
    let path = get_playback_state_path();
    if !path.exists() {
        return None;
    }
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

/// Dónde se quedó la canción actual. Va en su propio archivo (unas decenas de bytes) para poder
/// actualizarlo con frecuencia sin reescribir el estado completo con la cola.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PlaybackPosition {
    pub path: String,
    pub secs: f64,
}

/// A menos de esto del final se empieza la canción de cero (se cerró justo al terminar).
const END_MARGIN_SECS: f64 = 2.0;
/// Cuánto debe moverse la posición para volver a guardarla.
const MIN_MOVE_SECS: f64 = 5.0;

/// Posición a restaurar: la guardada acotada a la canción, o 0 si es inválida o está en los últimos segundos.
pub fn restore_position(saved_secs: f64, duration_secs: u64) -> f64 {
    if !saved_secs.is_finite() || saved_secs <= 0.0 {
        return 0.0;
    }
    if duration_secs > 0 && saved_secs >= duration_secs as f64 - END_MARGIN_SECS {
        return 0.0;
    }
    saved_secs
}

/// ¿Vale la pena volver a escribir la posición? (si no se movió, no se toca el disco)
pub fn position_moved(last_saved: Option<f64>, current: f64) -> bool {
    last_saved.map_or(true, |last| (current - last).abs() >= MIN_MOVE_SECS)
}

pub fn position_json(position: &PlaybackPosition) -> String {
    serde_json::to_string(position).unwrap_or_else(|_| "{}".to_string())
}

pub fn parse_position(json: &str) -> Option<PlaybackPosition> {
    let position: PlaybackPosition = serde_json::from_str(json).ok()?;
    (!position.path.is_empty() && position.secs.is_finite() && position.secs >= 0.0).then_some(position)
}

pub fn save_playback_position(position: &PlaybackPosition) -> Result<(), String> {
    fs::write(get_playback_position_path(), position_json(position)).map_err(|e| e.to_string())
}

pub fn load_playback_position() -> Option<PlaybackPosition> {
    parse_position(&fs::read_to_string(get_playback_position_path()).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_estado_viejo_con_carpeta_sigue_leyendose_y_uno_nuevo_sin_ella_tambien() {
        let viejo: PlaybackState = serde_json::from_str(r#"{"folder_path":"/m","queue_paths":[],"current_index":null,"volume":0.5,"is_shuffle":false,"repeat_mode":"off"}"#).unwrap();
        assert_eq!(viejo.folder_path.as_deref(), Some("/m"));
        let nuevo: PlaybackState = serde_json::from_str(r#"{"queue_paths":[],"current_index":null,"volume":0.5,"is_shuffle":false,"repeat_mode":"off"}"#).unwrap();
        assert_eq!(nuevo.folder_path, None);
    }

    #[test]
    fn la_posicion_guardada_se_acota_a_la_cancion() {
        assert_eq!(restore_position(42.5, 200), 42.5);
        assert_eq!(restore_position(0.0, 200), 0.0);
        assert_eq!(restore_position(-3.0, 200), 0.0, "negativa");
        assert_eq!(restore_position(f64::NAN, 200), 0.0, "inválida");
        assert_eq!(restore_position(f64::INFINITY, 200), 0.0, "inválida");
        assert_eq!(restore_position(500.0, 200), 0.0, "más allá del final: se empieza de cero");
    }

    #[test]
    fn cerca_del_final_se_empieza_de_cero() {
        assert_eq!(restore_position(197.9, 200), 197.9, "a más de 2 s del final se respeta");
        assert_eq!(restore_position(198.5, 200), 0.0, "en los últimos 2 s se descarta");
        assert_eq!(restore_position(200.0, 200), 0.0);
    }

    #[test]
    fn sin_duracion_conocida_se_respeta_la_posicion() {
        assert_eq!(restore_position(30.0, 0), 30.0);
    }

    #[test]
    fn la_posicion_va_y_viene_por_json_y_rechaza_lo_invalido() {
        let p = PlaybackPosition { path: "/m/a.mp3".into(), secs: 61.25 };
        assert_eq!(parse_position(&position_json(&p)), Some(p));
        assert_eq!(parse_position("{ roto"), None);
        assert_eq!(parse_position(r#"{"path":"","secs":10.0}"#), None, "sin ruta");
        assert_eq!(parse_position(r#"{"path":"/a","secs":-1.0}"#), None, "negativa");
        assert_eq!(parse_position(r#"{"secs":10.0}"#), None, "sin ruta");
    }

    #[test]
    fn solo_se_vuelve_a_guardar_si_la_posicion_se_movio_lo_suficiente() {
        assert!(position_moved(None, 12.0), "nunca se guardó");
        assert!(!position_moved(Some(100.0), 103.0), "menos de 5 s");
        assert!(position_moved(Some(100.0), 105.0));
        assert!(position_moved(Some(100.0), 20.0), "un salto hacia atrás también cuenta");
    }
}
