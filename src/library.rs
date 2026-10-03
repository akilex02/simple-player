use crate::paths::{get_covers_dir, get_library_cache_path, path_hash};
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
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

const SUPPORTED_EXT: [&str; 8] = ["mp3", "flac", "ogg", "wav", "m4a", "aac", "opus", "wma"];

pub fn is_supported_audio(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).map(|ext| SUPPORTED_EXT.contains(&ext.to_lowercase().as_str())).unwrap_or(false)
}

/// Biblioteca guardada en disco (arranque rápido); `None` si no hay caché o está vacía.
#[allow(dead_code)] // se conecta en la Tarea 4
pub fn load_cached_library() -> Option<Vec<Song>> {
    let content = fs::read_to_string(get_library_cache_path()).ok()?;
    let mut songs: Vec<Song> = serde_json::from_str(&content).ok()?;
    if songs.is_empty() {
        return None;
    }
    let covers_dir = get_covers_dir();
    for song in &mut songs {
        if let Some(c) = &mut song.cover_art {
            if let Some(filename) = c.strip_prefix("cover://") {
                // Migra las URI antiguas `cover://` a rutas absolutas.
                *c = covers_dir.join(filename).to_string_lossy().to_string();
            }
        }
    }
    Some(songs)
}

/// Canciones de todas las carpetas, en orden y sin repetir rutas. Una carpeta que no existe o no se
/// puede leer se omite con un aviso. No escribe la caché.
#[allow(dead_code)] // se conecta en la Tarea 4
pub fn scan_folders(folders: &[String], covers_dir: &Path) -> Vec<Song> {
    let mut seen = HashSet::new();
    let mut songs = Vec::new();
    for folder in folders {
        let root = Path::new(folder);
        if !root.is_dir() {
            eprintln!("[aviso] La carpeta «{folder}» no existe o no se puede leer; se omite.");
            continue;
        }
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && is_supported_audio(path) && seen.insert(path.to_string_lossy().to_string()) {
                songs.push(extract_song_info(path, covers_dir));
            }
        }
    }
    songs
}

/// `scan_folders` más la escritura de la caché (JSON ligero para el próximo arranque).
#[allow(dead_code)] // se conecta en la Tarea 4
pub fn scan_and_cache(folders: &[String]) -> Vec<Song> {
    let songs = scan_folders(folders, &get_covers_dir());
    if let Ok(json) = serde_json::to_string(&songs) {
        let _ = fs::write(get_library_cache_path(), json);
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_tree(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sp_lib_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"no es audio de verdad").unwrap();
    }

    fn paths(songs: &[Song]) -> Vec<String> {
        let mut p: Vec<String> = songs.iter().map(|s| s.path.clone()).collect();
        p.sort();
        p
    }

    #[test]
    fn solo_se_reconocen_las_extensiones_de_audio() {
        assert!(is_supported_audio(Path::new("/m/a.MP3")));
        assert!(is_supported_audio(Path::new("/m/b.flac")));
        assert!(!is_supported_audio(Path::new("/m/c.txt")));
        assert!(!is_supported_audio(Path::new("/m/sin_extension")));
    }

    #[test]
    fn une_las_canciones_de_dos_carpetas() {
        let root = temp_tree("two");
        touch(&root.join("a/uno.mp3"));
        touch(&root.join("b/dos.flac"));
        let covers = root.join("covers");
        let songs = scan_folders(&[root.join("a").to_string_lossy().into(), root.join("b").to_string_lossy().into()], &covers);
        assert_eq!(songs.len(), 2);
        assert_eq!(songs[0].title, "uno");
        assert_eq!(songs[1].title, "dos");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn una_carpeta_dentro_de_otra_no_repite_canciones() {
        let root = temp_tree("nested");
        touch(&root.join("a/uno.mp3"));
        touch(&root.join("a/sub/dos.mp3"));
        let (a, sub) = (root.join("a").to_string_lossy().to_string(), root.join("a/sub").to_string_lossy().to_string());
        let songs = scan_folders(&[a, sub], &root.join("covers"));
        assert_eq!(songs.len(), 2, "{:?}", paths(&songs));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn la_misma_carpeta_dos_veces_no_duplica() {
        let root = temp_tree("same");
        touch(&root.join("a/uno.mp3"));
        let a = root.join("a").to_string_lossy().to_string();
        assert_eq!(scan_folders(&[a.clone(), a], &root.join("covers")).len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn una_carpeta_inexistente_se_omite_y_las_demas_se_escanean() {
        let root = temp_tree("missing");
        touch(&root.join("a/uno.mp3"));
        let folders = [root.join("no-existe").to_string_lossy().to_string(), root.join("a").to_string_lossy().to_string()];
        let songs = scan_folders(&folders, &root.join("covers"));
        assert_eq!(songs.len(), 1);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sin_carpetas_no_hay_canciones() {
        let root = temp_tree("none");
        assert!(scan_folders(&[], &root.join("covers")).is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn los_archivos_que_no_son_audio_se_ignoran() {
        let root = temp_tree("ignore");
        touch(&root.join("a/uno.mp3"));
        touch(&root.join("a/notas.txt"));
        touch(&root.join("a/portada.jpg"));
        assert_eq!(scan_folders(&[root.join("a").to_string_lossy().into()], &root.join("covers")).len(), 1);
        let _ = fs::remove_dir_all(&root);
    }
}
