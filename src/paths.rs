use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

pub fn get_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let d = PathBuf::from(home).join(".cache").join("music-player");
    let _ = fs::create_dir_all(&d);
    d
}

pub fn get_library_cache_path() -> PathBuf {
    get_cache_dir().join("library_cache.json")
}

pub fn get_covers_dir() -> PathBuf {
    let d = get_cache_dir().join("covers");
    let _ = fs::create_dir_all(&d);
    d
}

pub fn get_playback_state_path() -> PathBuf {
    get_cache_dir().join("playback_state.json")
}

/// Archivo diminuto con la canción y el segundo en que se quedó (se escribe a menudo; el estado grande no).
pub fn get_playback_position_path() -> PathBuf {
    get_cache_dir().join("playback_position.json")
}

pub fn path_hash(path: &str) -> String {
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    format!("{:x}", h.finish())
}
