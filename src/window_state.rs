use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Tamaño con el que abre la app la primera vez.
pub const DEFAULT_SIZE: WindowSize = WindowSize { width: 1050.0, height: 750.0 };
/// Por debajo de esto la barra inferior ya no cabe sin encimarse.
pub const MIN_SIZE: (f32, f32) = (1000.0, 650.0);
const MAX_SIZE: (f32, f32) = (7680.0, 4320.0);

/// Tamaño interior de la ventana, en puntos lógicos de egui.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowSize {
    pub width: f32,
    pub height: f32,
}

impl WindowSize {
    /// Acota el tamaño a un rango usable; `None` si no es un número válido.
    pub fn sanitized(self) -> Option<WindowSize> {
        if !self.width.is_finite() || !self.height.is_finite() {
            return None;
        }
        Some(WindowSize {
            width: self.width.clamp(MIN_SIZE.0, MAX_SIZE.0),
            height: self.height.clamp(MIN_SIZE.1, MAX_SIZE.1),
        })
    }
}

pub fn window_state_path(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf {
    let base = match xdg_config_home.filter(|v| !v.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => PathBuf::from(home.unwrap_or("/tmp")).join(".config"),
    };
    base.join("simple-player").join("window.json")
}

/// ¿Se lee y se guarda el tamaño de `window.json`? Solo en una ejecución normal (ni desarrollo ni
/// `--window-size`) y si el usuario no lo desactivó en Configuración.
pub fn use_saved_size(dev_run: bool, forced: bool, remember: bool) -> bool {
    !dev_run && !forced && remember
}

/// Tras un restablecimiento de fábrica no se vuelve a escribir `window.json` al cerrar:
/// el tamaño debe volver al valor por defecto la próxima vez.
pub fn save_allowed(reset_done: bool) -> bool {
    !reset_done
}

pub fn parse(json: &str) -> Option<WindowSize> {
    serde_json::from_str::<WindowSize>(json).ok()?.sanitized()
}

pub fn to_json(size: WindowSize) -> String {
    serde_json::to_string(&size).unwrap_or_default()
}

/// Lee el tamaño guardado; `None` si no existe o no es válido.
pub fn load(path: &Path) -> Option<WindowSize> {
    parse(&std::fs::read_to_string(path).ok()?)
}

pub fn save(path: &Path, size: WindowSize) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, to_json(size))
}

/// `ANCHOxALTO` (por ejemplo `854x658`), para pruebas de desarrollo.
pub fn parse_size_arg(text: &str) -> Option<WindowSize> {
    let (w, h) = text.split_once(['x', 'X'])?;
    // Opción de desarrollo: se respeta tal cual (sin el mínimo) para probar tamaños chicos.
    let size = WindowSize { width: w.trim().parse().ok()?, height: h.trim().parse().ok()? };
    (size.width.is_finite() && size.height.is_finite() && size.width > 0.0 && size.height > 0.0).then_some(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(w: f32, h: f32) -> WindowSize {
        WindowSize { width: w, height: h }
    }

    #[test]
    fn el_tamano_por_defecto_es_1050_por_750() {
        assert_eq!(DEFAULT_SIZE, size(1050.0, 750.0));
    }

    #[test]
    fn la_ruta_respeta_xdg_config_home_y_si_no_usa_dot_config() {
        assert_eq!(window_state_path(Some("/cfg"), Some("/home/u")), PathBuf::from("/cfg/simple-player/window.json"));
        assert_eq!(
            window_state_path(None, Some("/home/u")),
            PathBuf::from("/home/u/.config/simple-player/window.json")
        );
        assert_eq!(
            window_state_path(Some(""), Some("/home/u")),
            PathBuf::from("/home/u/.config/simple-player/window.json")
        );
    }

    #[test]
    fn ida_y_vuelta_por_json() {
        assert_eq!(parse(&to_json(size(1234.0, 777.0))), Some(size(1234.0, 777.0)));
    }

    #[test]
    fn un_json_roto_o_incompleto_no_produce_tamano() {
        assert_eq!(parse("no es json"), None);
        assert_eq!(parse("{\"width\": 800.0}"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn los_tamanos_absurdos_se_acotan() {
        assert_eq!(size(10.0, 10.0).sanitized(), Some(size(1000.0, 650.0)));
        assert_eq!(size(99999.0, 99999.0).sanitized(), Some(size(7680.0, 4320.0)));
        assert_eq!(size(1050.0, 750.0).sanitized(), Some(size(1050.0, 750.0)));
    }

    #[test]
    fn un_tamano_no_numerico_se_descarta() {
        assert_eq!(size(f32::NAN, 700.0).sanitized(), None);
        assert_eq!(size(900.0, f32::INFINITY).sanitized(), None);
    }

    #[test]
    fn el_json_guardado_con_un_tamano_enorme_se_acota_al_leerlo() {
        assert_eq!(parse("{\"width\": 50000.0, \"height\": 100.0}"), Some(size(7680.0, 650.0)));
    }

    #[test]
    fn guarda_y_lee_en_un_archivo_creando_la_carpeta() {
        let dir = std::env::temp_dir().join(format!("sp_window_{}", std::process::id()));
        let path = dir.join("simple-player").join("window.json");
        let _ = std::fs::remove_dir_all(&dir);
        save(&path, size(1111.0, 666.0)).unwrap();
        assert_eq!(load(&path), Some(size(1111.0, 666.0)));
        assert_eq!(load(&dir.join("no-existe.json")), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn el_argumento_de_desarrollo_acepta_ancho_por_alto() {
        assert_eq!(parse_size_arg("854x658"), Some(size(854.0, 658.0)));
        assert_eq!(parse_size_arg("854X658"), Some(size(854.0, 658.0)));
        assert_eq!(parse_size_arg("854"), None);
        assert_eq!(parse_size_arg("axb"), None);
    }

    #[test]
    fn el_tamano_guardado_solo_se_usa_en_una_ejecucion_normal_con_la_opcion_activa() {
        assert!(use_saved_size(false, false, true));
        assert!(!use_saved_size(false, false, false), "el usuario lo desactivó");
        assert!(!use_saved_size(true, false, true), "corrida de desarrollo");
        assert!(!use_saved_size(false, true, true), "--window-size manda");
    }

    #[test]
    fn tras_un_restablecimiento_no_se_guarda_el_tamano_al_cerrar() {
        assert!(save_allowed(false));
        assert!(!save_allowed(true));
    }
}
