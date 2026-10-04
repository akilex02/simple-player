//! Ajustes del usuario (`settings.json`). Lógica pura salvo `load`/`save`.
use crate::ui::visualizers::VisualizerMode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const VERSION: u32 = 1;
const DEFAULT_VISUALIZER: &str = "Barras";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,
    /// Rutas absolutas, sin repetidos ni vacías.
    pub music_folders: Vec<String>,
    /// Etiqueta de `VisualizerMode`.
    pub default_visualizer: String,
    pub remember_window_size: bool,
    /// El usuario pidió no volver a ver el aviso de registrar el AppImage en el escritorio.
    pub integration_prompt_dismissed: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self { version: VERSION, music_folders: Vec::new(), default_visualizer: DEFAULT_VISUALIZER.into(), remember_window_size: true, integration_prompt_dismissed: false }
    }
}

/// Recorta espacios y la barra final (salvo la raíz).
fn normalize_folder(folder: &str) -> String {
    let trimmed = folder.trim();
    if trimmed.len() > 1 { trimmed.trim_end_matches('/').to_string() } else { trimmed.to_string() }
}

impl Settings {
    /// Quita carpetas vacías o repetidas (conserva el orden) y corrige un visualizador que ya no existe.
    pub fn sanitized(mut self) -> Settings {
        let mut folders: Vec<String> = Vec::new();
        for folder in &self.music_folders {
            let folder = normalize_folder(folder);
            if !folder.is_empty() && !folders.contains(&folder) {
                folders.push(folder);
            }
        }
        self.music_folders = folders;
        self.default_visualizer = VisualizerMode::from_name(&self.default_visualizer).map_or(DEFAULT_VISUALIZER, |m| m.label()).to_string();
        self
    }

    /// `true` si se agregó (no estaba y no es vacía).
    pub fn add_folder(&mut self, folder: &str) -> bool {
        let folder = normalize_folder(folder);
        if folder.is_empty() || self.music_folders.contains(&folder) {
            return false;
        }
        self.music_folders.push(folder);
        true
    }

    /// `true` si estaba y se quitó.
    pub fn remove_folder(&mut self, folder: &str) -> bool {
        let folder = normalize_folder(folder);
        let before = self.music_folders.len();
        self.music_folders.retain(|f| *f != folder);
        self.music_folders.len() != before
    }

    pub fn visualizer_mode(&self) -> VisualizerMode {
        VisualizerMode::from_name(&self.default_visualizer).unwrap_or(VisualizerMode::Bars)
    }
}

pub fn settings_path(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf {
    let base = match xdg_config_home.filter(|v| !v.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => PathBuf::from(home.unwrap_or("/tmp")).join(".config"),
    };
    base.join("simple-player").join("settings.json")
}

/// `None` si el JSON no es válido o es de una versión más nueva que la que entiende esta app.
pub fn parse(json: &str) -> Option<Settings> {
    let settings: Settings = serde_json::from_str(json).ok()?;
    (settings.version <= VERSION).then(|| settings.sanitized())
}

pub fn to_json(settings: &Settings) -> String {
    serde_json::to_string_pretty(settings).unwrap_or_else(|_| "{}".to_string())
}

pub fn load(path: &Path) -> Option<Settings> {
    parse(&std::fs::read_to_string(path).ok()?)
}

pub fn save(path: &Path, settings: &Settings) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, to_json(settings))
}

/// Ajustes iniciales cuando todavía no hay `settings.json`: conserva la carpeta que ya usaba el usuario.
pub fn migrated(playback_folder: Option<&str>) -> Settings {
    let mut settings = Settings::default();
    if let Some(folder) = playback_folder {
        settings.add_folder(folder);
    }
    settings
}

/// Lee `settings.json`. Si no existe, migra desde el estado de reproducción y **escribe de inmediato**
/// (así la carpeta no se pierde cuando `playback_state.json` deje de guardarla). Si existe pero está
/// corrupto, usa los valores por defecto sin migrar ni sobrescribir el archivo.
pub fn load_or_migrate(path: &Path, playback_folder: Option<&str>) -> Settings {
    if path.exists() {
        return load(path).unwrap_or_else(|| {
            eprintln!("[aviso] No se pudieron leer los ajustes de {}; se usan los valores por defecto.", path.display());
            Settings::default()
        });
    }
    let settings = migrated(playback_folder);
    if let Err(e) = save(path, &settings) {
        eprintln!("[aviso] No se pudieron guardar los ajustes: {e}");
    }
    settings
}

/// Carpetas de `--music-folder <ruta>` (repetible), para las corridas de desarrollo.
pub fn dev_folders_from_args(args: &[String]) -> Vec<String> {
    args.windows(2).filter(|w| w[0] == "--music-folder").map(|w| w[1].clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::visualizers::VisualizerMode;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sp_settings_{}_{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn los_valores_por_defecto_son_los_del_spec() {
        let s = Settings::default();
        assert_eq!(s.version, 1);
        assert!(s.music_folders.is_empty());
        assert_eq!(s.default_visualizer, "Barras");
        assert!(s.remember_window_size);
    }

    #[test]
    fn ida_y_vuelta_por_json() {
        let s = Settings { music_folders: vec!["/a".into(), "/b".into()], default_visualizer: "Anillo".into(), remember_window_size: false, ..Settings::default() };
        assert_eq!(parse(&to_json(&s)), Some(s));
    }

    #[test]
    fn faltan_campos_se_usan_los_por_defecto() {
        let s = parse(r#"{"music_folders": ["/x"]}"#).unwrap();
        assert_eq!(s.music_folders, vec!["/x".to_string()]);
        assert_eq!(s.default_visualizer, "Barras");
        assert!(s.remember_window_size);
    }

    #[test]
    fn un_json_corrupto_o_vacio_no_se_acepta() {
        assert_eq!(parse(""), None);
        assert_eq!(parse("{ no es json"), None);
        assert_eq!(parse("[1, 2]"), None);
    }

    #[test]
    fn una_version_mayor_no_se_acepta() {
        assert_eq!(parse(r#"{"version": 2, "music_folders": ["/x"]}"#), None);
    }

    #[test]
    fn sanitized_quita_vacias_y_repetidas_y_conserva_el_orden() {
        let s = Settings { music_folders: vec!["/b".into(), "".into(), "  ".into(), "/a".into(), "/b".into(), "/a/".into()], ..Settings::default() }.sanitized();
        assert_eq!(s.music_folders, vec!["/b".to_string(), "/a".to_string()]);
    }

    #[test]
    fn sanitized_cambia_un_visualizador_que_ya_no_existe() {
        for old in ["Radial", "Resplandor", "", "xyz"] {
            let s = Settings { default_visualizer: old.into(), ..Settings::default() }.sanitized();
            assert_eq!(s.default_visualizer, "Barras", "{old}");
        }
        let ok = Settings { default_visualizer: "constelacion".into(), ..Settings::default() }.sanitized();
        assert_eq!(ok.default_visualizer, "Constelación");
    }

    #[test]
    fn el_modo_sale_de_la_etiqueta() {
        let s = Settings { default_visualizer: "Osciloscopio".into(), ..Settings::default() };
        assert_eq!(s.visualizer_mode(), VisualizerMode::Wave);
        assert_eq!(Settings::default().visualizer_mode(), VisualizerMode::Bars);
    }

    #[test]
    fn agregar_y_quitar_carpetas() {
        let mut s = Settings::default();
        assert!(s.add_folder("/musica"));
        assert!(!s.add_folder("/musica"), "repetida");
        assert!(!s.add_folder("/musica/"), "repetida con barra final");
        assert!(!s.add_folder("   "), "vacía");
        assert!(s.add_folder("/otra"));
        assert_eq!(s.music_folders, vec!["/musica".to_string(), "/otra".to_string()]);
        assert!(s.remove_folder("/musica"));
        assert!(!s.remove_folder("/musica"), "ya no está");
        assert_eq!(s.music_folders, vec!["/otra".to_string()]);
    }

    #[test]
    fn la_ruta_respeta_xdg_config_home() {
        assert_eq!(settings_path(Some("/cfg"), Some("/home/u")), std::path::PathBuf::from("/cfg/simple-player/settings.json"));
        assert_eq!(settings_path(None, Some("/home/u")), std::path::PathBuf::from("/home/u/.config/simple-player/settings.json"));
        assert_eq!(settings_path(Some(""), Some("/home/u")), std::path::PathBuf::from("/home/u/.config/simple-player/settings.json"));
    }

    #[test]
    fn guardar_crea_el_directorio_y_se_puede_volver_a_leer() {
        let dir = temp_dir("save");
        let path = dir.join("nuevo").join("settings.json");
        let s = Settings { music_folders: vec!["/m".into()], ..Settings::default() };
        save(&path, &s).unwrap();
        assert_eq!(load(&path), Some(s));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn migra_la_carpeta_del_estado_de_reproduccion() {
        assert_eq!(migrated(Some("/vieja")).music_folders, vec!["/vieja".to_string()]);
        assert!(migrated(None).music_folders.is_empty());
        assert!(migrated(Some("  ")).music_folders.is_empty());
    }

    #[test]
    fn sin_archivo_migra_y_escribe_de_inmediato() {
        let dir = temp_dir("migrate");
        let path = dir.join("settings.json");
        let s = load_or_migrate(&path, Some("/vieja"));
        assert_eq!(s.music_folders, vec!["/vieja".to_string()]);
        assert_eq!(load(&path).map(|s| s.music_folders), Some(vec!["/vieja".to_string()]), "el archivo debe existir ya");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn con_archivo_valido_no_migra() {
        let dir = temp_dir("existing");
        let path = dir.join("settings.json");
        save(&path, &Settings { music_folders: vec!["/nueva".into()], ..Settings::default() }).unwrap();
        assert_eq!(load_or_migrate(&path, Some("/vieja")).music_folders, vec!["/nueva".to_string()]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn un_archivo_corrupto_da_valores_por_defecto_sin_migrar_ni_sobrescribirlo() {
        let dir = temp_dir("corrupt");
        let path = dir.join("settings.json");
        std::fs::write(&path, "{ corrupto").unwrap();
        let s = load_or_migrate(&path, Some("/vieja"));
        assert_eq!(s, Settings::default());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ corrupto");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn las_carpetas_de_desarrollo_salen_de_los_argumentos() {
        let args: Vec<String> = ["bin", "--music-folder", "/a", "--shot", "x.png", "--music-folder", "/b", "--music-folder"].iter().map(|s| s.to_string()).collect();
        assert_eq!(dev_folders_from_args(&args), vec!["/a".to_string(), "/b".to_string()]);
        assert!(dev_folders_from_args(&["bin".to_string()]).is_empty());
    }

    #[test]
    fn no_volver_a_preguntar_por_el_registro_se_recuerda_y_un_archivo_viejo_lo_deja_en_falso() {
        assert!(!Settings::default().integration_prompt_dismissed);
        assert!(!parse(r#"{"music_folders": ["/x"]}"#).unwrap().integration_prompt_dismissed, "archivo anterior sin el campo");
        let s = Settings { integration_prompt_dismissed: true, ..Settings::default() };
        assert_eq!(parse(&to_json(&s)), Some(s));
    }
}
