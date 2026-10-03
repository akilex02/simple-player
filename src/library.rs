use crate::paths::{get_covers_dir, get_library_cache_path, path_hash};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Song {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u64,
    pub cover_art: Option<String>,
}

pub fn select_folder() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string())
}

pub fn scan_music_folder(folder_path: Option<String>) -> Vec<Song> {
    let cache_file = get_library_cache_path();

    // Fast startup: serve from disk cache on first run
    if folder_path.is_none() && cache_file.exists() {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(mut songs) = serde_json::from_str::<Vec<Song>>(&content) {
                if !songs.is_empty() {
                    let covers_dir = get_covers_dir();
                    for song in &mut songs {
                        if let Some(ref mut c) = song.cover_art {
                            if c.starts_with("cover://") {
                                // Migrate old cover:// URIs to absolute paths
                                let filename = c.trim_start_matches("cover://");
                                *c = covers_dir.join(filename).to_string_lossy().to_string();
                            }
                        }
                    }
                    return songs;
                }
            }
        }
    }

    let target_dir = folder_path
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|h| format!("{}/Música", h))
                .unwrap_or_else(|_| "/home".to_string())
        });

    let supported_ext = ["mp3", "flac", "ogg", "wav", "m4a", "aac", "opus", "wma"];
    let covers_dir = get_covers_dir();

    let songs: Vec<Song> = WalkDir::new(&target_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| supported_ext.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .map(|e| extract_song_info(e.path(), &covers_dir))
        .collect();

    // Persist lean JSON (paths only, no base64) for instant future startups
    if let Ok(json) = serde_json::to_string(&songs) {
        let _ = fs::write(&cache_file, json);
    }

    songs
}

fn extract_song_info(path: &Path, covers_dir: &Path) -> Song {
    let mut title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut artist = "Artista Desconocido".to_string();
    let mut album = "Álbum Desconocido".to_string();
    let mut duration_secs = 0u64;
    let mut cover_art: Option<String> = None;

    if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
        duration_secs = tagged_file.properties().duration().as_secs();

        if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
            if let Some(t) = tag.title().filter(|t| !t.trim().is_empty()) {
                title = t.trim().to_string();
            }
            if let Some(a) = tag.artist().filter(|a| !a.trim().is_empty()) {
                artist = a.trim().to_string();
            }
            if let Some(al) = tag.album().filter(|al| !al.trim().is_empty()) {
                album = al.trim().to_string();
            }

            if let Some(picture) = tag.pictures().first() {
                let hash = path_hash(&path.to_string_lossy());
                let cover_path = covers_dir.join(format!("{}.jpg", hash));

                if !cover_path.exists() {
                    let _ = fs::write(&cover_path, picture.data());
                }
                cover_art = Some(cover_path.to_string_lossy().to_string());
            }
        }
    }

    Song {
        path: path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        duration_secs,
        cover_art,
    }
}
