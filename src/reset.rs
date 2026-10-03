//! Restablecer de fábrica: borra ajustes, tamaño de ventana, estado de reproducción y caché de biblioteca.
//! No toca la base de estadísticas ni los archivos de música.
use crate::paths::{get_covers_dir, get_library_cache_path, get_playback_state_path};
use std::path::{Path, PathBuf};

pub struct ResetReport {
    pub removed: usize,
    pub errors: Vec<String>,
}

/// Archivos y carpetas que se borran. `settings_file` y `window_file` se reciben para poder probarlo.
pub fn factory_paths(settings_file: &Path, window_file: &Path) -> (Vec<PathBuf>, Vec<PathBuf>) {
    (
        vec![settings_file.to_path_buf(), window_file.to_path_buf(), get_playback_state_path(), get_library_cache_path()],
        vec![get_covers_dir()],
    )
}

/// Borra lo que exista; lo que ya no está no cuenta como error.
pub fn remove_all(files: &[PathBuf], dirs: &[PathBuf]) -> ResetReport {
    let mut report = ResetReport { removed: 0, errors: Vec::new() };
    for file in files {
        match std::fs::remove_file(file) {
            Ok(()) => report.removed += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => report.errors.push(format!("{}: {e}", file.display())),
        }
    }
    for dir in dirs {
        match std::fs::remove_dir_all(dir) {
            Ok(()) => report.removed += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => report.errors.push(format!("{}: {e}", dir.display())),
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sp_reset_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn borra_los_archivos_y_las_carpetas_indicados_y_nada_mas() {
        let root = temp_dir("all");
        let (settings, window, playback, cache) =
            (root.join("settings.json"), root.join("window.json"), root.join("playback_state.json"), root.join("library_cache.json"));
        let covers = root.join("covers");
        let stats = root.join("stats.db");
        for f in [&settings, &window, &playback, &cache, &stats] {
            fs::write(f, "x").unwrap();
        }
        fs::create_dir_all(&covers).unwrap();
        fs::write(covers.join("a.jpg"), "x").unwrap();

        let report = remove_all(&[settings.clone(), window.clone(), playback.clone(), cache.clone()], &[covers.clone()]);
        assert_eq!(report.removed, 5);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        for f in [&settings, &window, &playback, &cache, &covers] {
            assert!(!f.exists(), "{f:?} debía borrarse");
        }
        assert!(stats.exists(), "la base de estadísticas no se toca");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn lo_que_ya_no_existe_no_es_un_error() {
        let root = temp_dir("missing");
        let report = remove_all(&[root.join("no-existe.json")], &[root.join("no-existe-dir")]);
        assert_eq!(report.removed, 0);
        assert!(report.errors.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn la_lista_de_fabrica_son_exactamente_los_archivos_del_spec() {
        let (files, dirs) = factory_paths(Path::new("/c/settings.json"), Path::new("/c/window.json"));
        let names: Vec<String> = files.iter().filter_map(|p| p.file_name()).map(|n| n.to_string_lossy().to_string()).collect();
        assert_eq!(names, ["settings.json", "window.json", "playback_state.json", "library_cache.json"]);
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].file_name().unwrap().to_string_lossy(), "covers");
        assert!(files.iter().chain(dirs.iter()).all(|p| !p.to_string_lossy().contains("stats.db")));
    }
}
