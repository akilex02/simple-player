use crate::paths::get_playback_state_path;
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
}
