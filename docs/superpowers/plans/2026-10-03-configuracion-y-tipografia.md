# Configuración y tipografía unificada — Plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Unificar toda la tipografía en GTA Art Deco y agregar una pantalla de Configuración (carpetas de música múltiples, preferencias, exportar/importar/borrar el historial y restablecer de fábrica).

**Architecture:** Lógica pura y probada sin ventana (`theme/fonts.rs`, `settings.rs`, `library::scan_folders`, `stats/transfer.rs`, `reset.rs`, ayudantes de la pantalla); las acciones de historial pasan por mensajes al hilo de `StatsService`; la pantalla es una tarjeta de vidrio por sección en `ui/screens/settings.rs`.

**Tech Stack:** Rust, egui/eframe 0.29, `serde_json`, `rusqlite` (ya presentes), `rfd` (diálogos, ya presente), `chrono` (ya presente); sin dependencias nuevas.

**Spec:** `docs/superpowers/specs/2026-10-03-configuracion-y-tipografia-design.md`

## Global Constraints

- Tipografía: texto normal `GTAArtDeco_Regular.ttf`, negritas `GTAArtDeco_Bold.ttf`, títulos `GTAArtDeco_CondensedBold.ttf`; Noto Sans Regular solo como respaldo; `NotoSans-Bold.ttf` se elimina del repositorio. Nombres de familia: `FONT_REGULAR = "GTAArtDeco"`, `FONT_BOLD = "GTAArtDecoBold"`, `FONT_CONDENSED = "GTAArtDecoCondensed"`. `theme::bold()` y `theme::deco()` conservan nombre y firma.
- Ajustes: `$XDG_CONFIG_HOME/simple-player/settings.json` (por defecto `~/.config/simple-player/settings.json`), JSON con `"version": 1`; campos `music_folders: Vec<String>`, `default_visualizer: String` (por defecto `"Barras"`), `remember_window_size: bool` (por defecto `true`).
- Migración: si `settings.json` no existe y `playback_state.json` trae `folder_path`, `music_folders = [folder_path]` y el archivo se **escribe de inmediato**; `playback_state.json` deja de escribir `folder_path`.
- Sin carpetas ni caché, la biblioteca queda vacía (se elimina el valor implícito `~/Música`). Una carpeta inexistente se omite con aviso y sigue en los ajustes. Se quitan duplicados por ruta.
- Historial exportado: JSON `{"app":"simple-player","version":1,"exported_at":<ms>,"events":[{started_at, ended_at, listened_ms, path, title, artist, album, duration_ms}]}`. Duplicado = mismo `started_at`, `ended_at` y `path`. Inválido = `listened_ms == 0`, `listened_ms > i64::MAX`, tiempos negativos o `ended_at < started_at`. La importación es **una sola transacción**.
- Restablecer de fábrica borra `settings.json`, `window.json`, `playback_state.json`, `library_cache.json` y `covers/`; **conserva** la base de estadísticas y los archivos de música. "Borrar historial" y "Restablecer de fábrica" piden confirmación en la misma fila.
- Las corridas de desarrollo (`--shot`, `--bench`, `--gallery`) nunca leen ni escriben `settings.json`, `window.json` ni la base real; el restablecer de fábrica queda deshabilitado en ellas. `--music-folder <ruta>` (repetible) siembra carpetas en esos ajustes en memoria.
- Tamaño mínimo de ventana 1000×650 y por defecto 1050×750 (sin cambios). Sin repintado continuo extra en la pantalla de Configuración.
- Texto de interfaz en español. Sin advertencias del compilador. Sin dependencias nuevas.
- Sin push ni merge: todo se commitea en la rama `nueva-version`.
- Commits terminan con `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Comandos de prueba (desde la raíz): `cargo test <filtro>`; suite completa `cargo test`.

**Aclaraciones al spec (decididas al planificar):**
- La confirmación pendiente de la pantalla se cancela al salir de ella con un contador de frames (`SettingsUi::begin_frame`), sin tocar `AppState`.
- `library_cache.json` sigue siendo una lista plana de canciones; `scan_music_folder` se sustituye por `load_cached_library`, `scan_folders` (puro, sin escribir caché) y `scan_and_cache`.
- La carátula de caché se limpia borrando **los archivos** de `covers/` (la carpeta se conserva) y re-escaneando.
- Mientras no existe la pantalla (Tareas 4–5) el sidebar conserva una sección "Carpeta" mínima ("Agregar carpeta") para que la app siga siendo usable entre commits; la Tarea 6 la elimina.

## Review Focus

Entradas o condiciones que el spec implica pero que ninguna tarea cubriría sola; cada una tiene su prueba:

1. **Pérdida de la carpeta al migrar:** primera ejecución con carpeta en `playback_state.json`; si `settings.json` no se escribe al migrar y `persist()` deja de guardar `folder_path`, la carpeta se pierde. → Tarea 2 (`load_or_migrate` escribe) y Tarea 4.
2. **`settings.json` corrupto o de versión mayor:** valores por defecto, sin pánico y **sin** migrar encima. → Tarea 2.
3. **Carpeta que ya no existe o sin permisos:** se omite sin pánico y las demás carpetas siguen escaneándose. → Tarea 3.
4. **Archivo de importación corrupto o con números desbordados:** no se inserta nada; un fallo a mitad de lote no deja eventos parciales. → Tarea 8.
5. **Restablecer de fábrica:** no borra la base de estadísticas ni archivos fuera de la lista, y es inocuo si algún archivo ya no existe. → Tarea 10.

---

### Task 1: Tipografía unificada en Art Deco

**Files:**
- Create: `src/theme/fonts.rs`
- Modify: `src/theme/mod.rs` (declarar `pub mod fonts;`, quitar las constantes y `install_fonts` antiguos, importar de `fonts`)
- Delete: `assets/fonts/NotoSans-Bold.ttf`
- Test: `src/theme/fonts.rs` (módulo `tests`)

**Interfaces:**
- Consumes: `egui_phosphor::add_to_fonts`, archivos de `assets/fonts/`.
- Produces: `theme::fonts::{FONT_REGULAR, FONT_BOLD, FONT_CONDENSED, build_font_definitions() -> egui::FontDefinitions}`; `theme::bold(size)`/`theme::deco(size)` sin cambios de firma.

- [ ] **Step 1: Write the failing tests**

Crear `src/theme/fonts.rs` con solo esto (y `pub mod fonts;` en `src/theme/mod.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::FontFamily;

    fn chain(fonts: &eframe::egui::FontDefinitions, family: FontFamily) -> Vec<String> {
        fonts.families.get(&family).cloned().unwrap_or_default()
    }

    #[test]
    fn la_interfaz_normal_empieza_con_art_deco_regular() {
        let fonts = build_font_definitions();
        assert_eq!(chain(&fonts, FontFamily::Proportional)[0], FONT_REGULAR);
    }

    #[test]
    fn las_negritas_empiezan_con_art_deco_bold() {
        let fonts = build_font_definitions();
        assert_eq!(chain(&fonts, FontFamily::Name(FONT_BOLD.into()))[0], FONT_BOLD);
    }

    #[test]
    fn los_titulos_empiezan_con_art_deco_condensado() {
        let fonts = build_font_definitions();
        assert_eq!(chain(&fonts, FontFamily::Name(FONT_CONDENSED.into()))[0], FONT_CONDENSED);
    }

    #[test]
    fn noto_sans_queda_como_respaldo_en_las_tres_familias() {
        let fonts = build_font_definitions();
        for family in [FontFamily::Proportional, FontFamily::Name(FONT_BOLD.into()), FontFamily::Name(FONT_CONDENSED.into())] {
            let names = chain(&fonts, family.clone());
            let at = names.iter().position(|n| n == "NotoSans");
            assert!(matches!(at, Some(i) if i > 0), "{family:?}: {names:?}");
        }
    }

    #[test]
    fn los_iconos_de_phosphor_siguen_disponibles() {
        let fonts = build_font_definitions();
        assert!(chain(&fonts, FontFamily::Proportional).iter().any(|n| n == "phosphor"));
        assert!(chain(&fonts, FontFamily::Name(FONT_BOLD.into())).iter().any(|n| n == "phosphor"));
    }

    #[test]
    fn toda_fuente_de_una_cadena_tiene_sus_datos() {
        let fonts = build_font_definitions();
        for (family, names) in &fonts.families {
            for name in names {
                assert!(fonts.font_data.contains_key(name), "{family:?} usa «{name}» sin datos");
            }
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test theme::fonts 2>&1 | tail -15`
Expected: error de compilación `cannot find function build_font_definitions` / `cannot find value FONT_REGULAR` (RED).

- [ ] **Step 3: Write minimal implementation**

Insertar **antes** de `#[cfg(test)]` en `src/theme/fonts.rs`:

```rust
//! Armado de las fuentes: GTA Art Deco en toda la interfaz; Noto Sans solo como respaldo de glifos.
use eframe::egui::{FontData, FontDefinitions, FontFamily};

pub const FONT_REGULAR: &str = "GTAArtDeco";
pub const FONT_BOLD: &str = "GTAArtDecoBold";
pub const FONT_CONDENSED: &str = "GTAArtDecoCondensed";
const FONT_FALLBACK: &str = "NotoSans";

pub fn build_font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let files: &[(&str, &[u8])] = &[
        (FONT_REGULAR, include_bytes!("../../assets/fonts/GTAArtDeco_Regular.ttf")),
        (FONT_BOLD, include_bytes!("../../assets/fonts/GTAArtDeco_Bold.ttf")),
        (FONT_CONDENSED, include_bytes!("../../assets/fonts/GTAArtDeco_CondensedBold.ttf")),
        (FONT_FALLBACK, include_bytes!("../../assets/fonts/NotoSans-Regular.ttf")),
    ];
    for (name, bytes) in files {
        fonts.font_data.insert(name.to_string(), FontData::from_static(bytes));
    }

    // Interfaz: Art Deco primero; luego Noto Sans (respaldo de glifos), Phosphor (íconos) y las
    // fuentes por defecto de egui, que aportan cobertura de emoji.
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, FONT_REGULAR.into());
    proportional.insert(1, FONT_FALLBACK.into());
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let fallback: Vec<String> =
        fonts.families[&FontFamily::Proportional].iter().filter(|name| name.as_str() != FONT_REGULAR).cloned().collect();
    for primary in [FONT_BOLD, FONT_CONDENSED] {
        let mut chain = vec![primary.to_string()];
        chain.extend(fallback.iter().cloned());
        fonts.families.insert(FontFamily::Name(primary.into()), chain);
    }
    fonts
}
```

En `src/theme/mod.rs`:
- Borrar las tres constantes `FONT_CONDENSED`, `FONT_REGULAR`, `FONT_BOLD` y la función `install_fonts` completa.
- Añadir `use fonts::{FONT_BOLD, FONT_CONDENSED};` junto a los demás `use`.
- Donde estaba `install_fonts(ctx);` dentro de `pub fn install`, poner `ctx.set_fonts(fonts::build_font_definitions());`.
- Quitar `FontData` y `FontDefinitions` del `use eframe::egui::{...}` si quedan sin uso.
- `git rm assets/fonts/NotoSans-Bold.ttf`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test 2>&1 | grep -E "^test result|^error|^warning" -A4`
Expected: suite completa en verde (6 pruebas nuevas) y sin advertencias. Si `phosphor` no es el nombre de la fuente de íconos, ajustar las dos aserciones al nombre real que imprime el fallo (es un dato de `egui_phosphor`, no del diseño).

- [ ] **Step 5: Verificar visualmente y ajustar el ancho**

Compilar y capturar las pantallas con texto denso, a dos tamaños:

```bash
cargo build --release 2>&1 | grep -E "^(error|warning)" -A5
S=/tmp/claude-1000/-home-budja8-Documents-Proyectos-simple-player-master/5badf05a-e152-478c-aeb0-3922f4d01526/scratchpad
for w in 1050x750 1000x650; do
  timeout 40 ./target/release/simple-player --allow-multiple --no-hotkeys --window-size $w --play --time 60 --shot $S/font_main_$w.png >/dev/null 2>&1
  timeout 40 ./target/release/simple-player --allow-multiple --no-hotkeys --window-size $w --stats-demo --tab stats --shot $S/font_stats_$w.png >/dev/null 2>&1
  timeout 40 ./target/release/simple-player --allow-multiple --no-hotkeys --window-size $w --queue --play --time 60 --shot $S/font_queue_$w.png >/dev/null 2>&1
  timeout 40 ./target/release/simple-player --allow-multiple --no-hotkeys --window-size $w --fullscreen --play --time 60 --shot $S/font_full_$w.png >/dev/null 2>&1
done
```

Leer cada captura con la herramienta Read y comprobar: lista de canciones (título, artista, álbum, duración sin cortarse de forma fea), barra inferior (título y tiempos), hero, tarjetas de Estadísticas, cola y pantalla completa. Donde un texto deje de caber, ajustar primero el tamaño de la escala `theme::text` (un punto menos en el valor afectado) o el truncado de esa vista; **no** rediseñar. Anotar en el mensaje del commit los ajustes hechos.

- [ ] **Step 6: Commit**

```bash
git add -A src assets
git commit -m "Tipografía unificada: GTA Art Deco en toda la interfaz

Noto Sans Regular queda solo como respaldo de glifos; NotoSans-Bold ya no se embebe.

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Ajustes (`settings.rs`) y migración

**Files:**
- Create: `src/settings.rs`
- Modify: `src/main.rs` (declarar `mod settings;`)
- Test: `src/settings.rs` (módulo `tests`)

**Interfaces:**
- Consumes: `crate::ui::visualizers::VisualizerMode` (`from_name`, `label`).
- Produces (todo `pub` en `crate::settings`): `Settings { version: u32, music_folders: Vec<String>, default_visualizer: String, remember_window_size: bool }` (`Clone, Debug, PartialEq, Serialize, Deserialize, Default`); `Settings::sanitized(self) -> Settings`; `Settings::add_folder(&mut self, folder: &str) -> bool`; `Settings::remove_folder(&mut self, folder: &str) -> bool`; `Settings::visualizer_mode(&self) -> VisualizerMode`; `settings_path(xdg_config_home: Option<&str>, home: Option<&str>) -> PathBuf`; `parse(json: &str) -> Option<Settings>`; `to_json(&Settings) -> String`; `load(path: &Path) -> Option<Settings>`; `save(path: &Path, &Settings) -> std::io::Result<()>`; `migrated(playback_folder: Option<&str>) -> Settings`; `load_or_migrate(path: &Path, playback_folder: Option<&str>) -> Settings`; `dev_folders_from_args(args: &[String]) -> Vec<String>`.

- [ ] **Step 1: Write the failing tests**

Crear `src/settings.rs` con `#![allow(dead_code)] // se quita al conectar (Tarea 4)` y solo esto, y `mod settings;` en `src/main.rs`:

```rust
#![allow(dead_code)] // se quita al conectar (Tarea 4)

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
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test settings 2>&1 | tail -15`
Expected: error de compilación (`cannot find type Settings` …) (RED).

- [ ] **Step 3: Write minimal implementation**

Insertar **antes** de `#[cfg(test)]` en `src/settings.rs` (después del `#![allow(dead_code)]`):

```rust
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
}

impl Default for Settings {
    fn default() -> Self {
        Self { version: VERSION, music_folders: Vec::new(), default_visualizer: DEFAULT_VISUALIZER.into(), remember_window_size: true }
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test settings 2>&1 | tail -15`
Expected: `16 passed`. (`--music-folder` final sin valor no cuenta: `windows(2)` lo ignora.)

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs src/main.rs
git commit -m "Ajustes de usuario: modelo, validación, migración y guardado

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Escaneo de varias carpetas

**Files:**
- Modify: `src/library.rs`
- Test: `src/library.rs` (módulo `tests` nuevo)

**Interfaces:**
- Consumes: `paths::{get_covers_dir, get_library_cache_path, path_hash}`; `extract_song_info(path, covers_dir)` (ya existe).
- Produces: `library::load_cached_library() -> Option<Vec<Song>>`; `library::scan_folders(folders: &[String], covers_dir: &Path) -> Vec<Song>`; `library::scan_and_cache(folders: &[String]) -> Vec<Song>`; `library::is_supported_audio(path: &Path) -> bool`. **Se conserva `scan_music_folder`** hasta la Tarea 4, que lo elimina.

- [ ] **Step 1: Write the failing tests**

Añadir al final de `src/library.rs`:

```rust
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
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test library 2>&1 | tail -12`
Expected: error de compilación `cannot find function scan_folders` (RED).

- [ ] **Step 3: Write minimal implementation**

En `src/library.rs`: añadir `use std::collections::HashSet;` y, junto a `scan_music_folder`, estas funciones (la de caché reutiliza el bloque de lectura que hoy vive dentro de `scan_music_folder`):

```rust
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
```

(`SUPPORTED_EXT` reemplaza al arreglo local de `scan_music_folder`; dejar ese `let supported_ext` como está o apuntarlo a la constante; la función vieja se elimina en la Tarea 4.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test library 2>&1 | tail -12`
Expected: `7 passed` y la suite completa sigue en verde (`cargo test 2>&1 | grep "test result"`).

- [ ] **Step 5: Commit**

```bash
git add src/library.rs
git commit -m "Escaneo de varias carpetas sin repetir canciones

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Ajustes en `AppState` y en el arranque

**Files:**
- Modify: `src/state.rs`, `src/library.rs`, `src/persistence.rs`, `src/main.rs`, `src/settings.rs`, `src/ui/screens/songs.rs`, `src/ui/shell/sidebar.rs`
- Test: `src/persistence.rs` (módulo `tests` nuevo, solo el campo opcional)

**Interfaces:**
- Consumes: `Settings`, `settings::{load_or_migrate, save, settings_path, dev_folders_from_args}`, `library::{load_cached_library, scan_and_cache, select_folder}`.
- Produces: `AppState.settings: Settings`; `AppState::with_settings(self, settings: Settings, path: Option<PathBuf>) -> Self`; `AppState::add_folder_via_dialog(&mut self)`; `AppState::add_music_folder(&mut self, folder: &str)`; `AppState::remove_music_folder(&mut self, folder: &str)`; `AppState::rescan_library(&mut self)`; `AppState::update_settings(&mut self, change: impl FnOnce(&mut Settings))` (aplica y guarda); `App::new(cc, window_state_path, settings, settings_path)`. Desaparecen `current_folder_path`, `select_folder_and_scan` y `scan_folder`.

- [ ] **Step 1: Write the failing test**

Añadir en `src/persistence.rs`:

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test persistence 2>&1 | tail -10`
Expected: FAIL — falta `#[serde(default)]` en `folder_path`? (`Option` ya acepta ausencia en serde; si la prueba pasa de inmediato, es válida como protección de regresión: anotarlo y seguir; el RED real de esta tarea es el de compilación al quitar `current_folder_path`.)

- [ ] **Step 3: Write the implementation**

`src/persistence.rs`: añadir `#[serde(default)]` sobre `folder_path` (documentar: «solo se lee, para migrar a `settings.json`»).

`src/state.rs`:
- Reemplazar el campo `pub current_folder_path: Option<String>` por:
  ```rust
  pub settings: Settings,
  settings_path: Option<PathBuf>,
  ```
  y en `AppState::new`: `settings: Settings::default(), settings_path: None,`. Importar `crate::settings::{self, Settings}` y `std::path::PathBuf`; actualizar el `use crate::library::{...}` a `{load_cached_library, scan_and_cache, select_folder, Song}`.
- Añadir:
  ```rust
  /// Ajustes cargados al arrancar y dónde guardarlos (`None` en corridas de desarrollo: no se escribe nada).
  pub fn with_settings(mut self, settings: Settings, path: Option<PathBuf>) -> Self {
      self.settings = settings;
      self.settings_path = path;
      self
  }

  fn save_settings(&self) {
      if let Some(path) = &self.settings_path {
          if let Err(e) = settings::save(path, &self.settings) {
              eprintln!("[aviso] No se pudieron guardar los ajustes: {e}");
          }
      }
  }

  /// Aplica un cambio a los ajustes y lo guarda.
  pub fn update_settings(&mut self, change: impl FnOnce(&mut Settings)) {
      change(&mut self.settings);
      self.save_settings();
  }

  pub fn add_folder_via_dialog(&mut self) {
      if let Some(folder) = select_folder() {
          self.add_music_folder(&folder);
      }
  }

  pub fn add_music_folder(&mut self, folder: &str) {
      if self.settings.add_folder(folder) {
          self.save_settings();
          self.rescan_library();
      }
  }

  pub fn remove_music_folder(&mut self, folder: &str) {
      if self.settings.remove_folder(folder) {
          self.save_settings();
          self.rescan_library();
      }
  }

  pub fn rescan_library(&mut self) {
      self.loading = true;
      self.set_songs(scan_and_cache(&self.settings.music_folders));
      self.loading = false;
  }
  ```
- En `init()`: sustituir `self.set_songs(scan_music_folder(None));` por `self.set_songs(load_cached_library().unwrap_or_else(|| scan_and_cache(&self.settings.music_folders)));` y **borrar** el bloque `if saved.folder_path.is_some() { self.current_folder_path = saved.folder_path; }`.
- En `persist()`: `folder_path: None,`.
- Borrar `select_folder_and_scan` y `scan_folder`.

`src/library.rs`: borrar `scan_music_folder` y los `#[allow(dead_code)]` de `load_cached_library`, `scan_folders` y `scan_and_cache`. En `src/settings.rs`: borrar el `#![allow(dead_code)]` (si algún ítem queda sin uso, conectarlo, no silenciarlo).

`src/ui/screens/songs.rs` (estado vacío):
```rust
    if state.songs.is_empty() {
        let (title, subtitle, cta) = if state.settings.music_folders.is_empty() {
            ("Elige tu carpeta de música".to_string(), "Simple Player buscará tus canciones ahí.", "Elegir carpeta")
        } else {
            ("No hay canciones en tus carpetas".to_string(), "Prueba agregando otra carpeta.", "Agregar carpeta")
        };
        if empty_state(ui, icons::FOLDER_OPEN, &title, subtitle, Some(cta)) {
            state.add_folder_via_dialog();
        }
        return;
    }
```

`src/ui/shell/sidebar.rs` (sección "CARPETA" temporal, hasta la Tarea 6): reemplazar el bloque por
```rust
    ui.add_space(space::XL);
    section_label(ui, "CARPETA");
    let count = state.settings.music_folders.len();
    let summary = match count {
        0 => "Sin carpetas".to_string(),
        1 => "1 carpeta".to_string(),
        n => format!("{n} carpetas"),
    };
    ui.horizontal(|ui| {
        ui.label(RichText::new(icons::FOLDER_OPEN).size(text::LG).color(theme::TEXT_MUTED));
        ui.label(RichText::new(summary).color(theme::TEXT_MAIN));
    });
    ui.add_space(space::SM);
    ui.add_enabled_ui(!state.loading, |ui| {
        if PillButton::new("Agregar carpeta", PillKind::Secondary).icon(icons::FOLDER_OPEN).show(ui).clicked() {
            state.add_folder_via_dialog();
        }
    });
```

`src/main.rs` — en `main()`, **antes** del bloque de tamaño de ventana, cargar los ajustes:
```rust
    // Ajustes: las corridas de desarrollo usan unos en memoria (con `--music-folder`) y nunca tocan el archivo real.
    let args: Vec<String> = std::env::args().collect();
    let dev_run = ["--shot", "--bench", "--gallery"].iter().any(|f| args.iter().any(|a| a == *f));
    let real_settings_path = settings::settings_path(std::env::var("XDG_CONFIG_HOME").ok().as_deref(), std::env::var("HOME").ok().as_deref());
    let (settings, settings_path) = if dev_run {
        (settings::Settings { music_folders: settings::dev_folders_from_args(&args), ..settings::Settings::default() }.sanitized(), None)
    } else {
        let playback_folder = persistence::load_playback_state().and_then(|s| s.folder_path);
        (settings::load_or_migrate(&real_settings_path, playback_folder.as_deref()), Some(real_settings_path))
    };
```
y pasar `settings`/`settings_path` a `App::new(cc, window_state_path, settings, settings_path)` (el cierre de `run_native` pasa a `move`, ya lo es). En `App::new` cambiar la firma, y `let mut state = AppState::new(audio, mpris_tx).with_settings(settings, settings_path); state.init();`. La variable `dev_window` existente puede reutilizar `dev_run`.

- [ ] **Step 4: Run tests and build**

Run: `cargo test 2>&1 | grep -E "^test result|^error|^warning" -A5; cargo build --release 2>&1 | grep -E "^(error|warning)" -A6`
Expected: suite en verde y sin advertencias. Probar a mano la migración con un `XDG_CONFIG_HOME` temporal:
```bash
T=$(mktemp -d); mkdir -p $T/c; ./target/release/simple-player --allow-multiple --no-hotkeys --stats-db $T/s.db & sleep 4; kill %1; ls $T/c/simple-player 2>/dev/null; rm -rf $T
```
(con `XDG_CONFIG_HOME=$T/c` y sin `--shot`; debe aparecer `settings.json`. Cerrar la app tras comprobar.)

- [ ] **Step 5: Commit**

```bash
git add -A src
git commit -m "Los ajustes viven en AppState; la biblioteca se arma con varias carpetas

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Recordar tamaño de ventana y visualizador por defecto

**Files:**
- Modify: `src/window_state.rs`, `src/main.rs`
- Test: `src/window_state.rs`

**Interfaces:**
- Consumes: `Settings.remember_window_size`, `Settings::visualizer_mode()`.
- Produces: `window_state::use_saved_size(dev_run: bool, forced: bool, remember: bool) -> bool`.

- [ ] **Step 1: Write the failing test**

Añadir a `mod tests` de `src/window_state.rs`:

```rust
    #[test]
    fn el_tamano_guardado_solo_se_usa_en_una_ejecucion_normal_con_la_opcion_activa() {
        assert!(use_saved_size(false, false, true));
        assert!(!use_saved_size(false, false, false), "el usuario lo desactivó");
        assert!(!use_saved_size(true, false, true), "corrida de desarrollo");
        assert!(!use_saved_size(false, true, true), "--window-size manda");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test window_state 2>&1 | tail -8`
Expected: error `cannot find function use_saved_size` (RED).

- [ ] **Step 3: Write minimal implementation**

En `src/window_state.rs`:

```rust
/// ¿Se lee y se guarda el tamaño de `window.json`? Solo en una ejecución normal (ni desarrollo ni
/// `--window-size`) y si el usuario no lo desactivó en Configuración.
pub fn use_saved_size(dev_run: bool, forced: bool, remember: bool) -> bool {
    !dev_run && !forced && remember
}
```

En `main()` (`src/main.rs`), sustituir el cálculo de `initial`/`window_state_path` por:

```rust
    let forced = ui::gallery::arg_value("--window-size").and_then(|v| window_state::parse_size_arg(&v));
    let use_saved = window_state::use_saved_size(dev_run, forced.is_some(), settings.remember_window_size);
    let saved_path = window_state::window_state_path(std::env::var("XDG_CONFIG_HOME").ok().as_deref(), std::env::var("HOME").ok().as_deref());
    let initial = forced
        .or_else(|| if use_saved { window_state::load(&saved_path) } else { None })
        .unwrap_or(window_state::DEFAULT_SIZE);
    let window_state_path = use_saved.then_some(saved_path);
```
(quitar la antigua variable `dev_window`.) En `App::new`, después de crear `fullscreen_view`: `fullscreen_view.mode = state.settings.visualizer_mode();` **antes** del bloque de `--viz` (que sigue pudiendo sobrescribirlo).

- [ ] **Step 4: Run tests and build**

Run: `cargo test 2>&1 | grep -E "^test result|^error|^warning" -A4; cargo build --release 2>&1 | grep -E "^(error|warning)" -A5`
Expected: suite en verde y sin advertencias.

- [ ] **Step 5: Commit**

```bash
git add src/window_state.rs src/main.rs
git commit -m "Respeta 'recordar tamaño de ventana' y el visualizador por defecto de los ajustes

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Pantalla de Configuración — navegación y sección Biblioteca

**Files:**
- Create: `src/ui/screens/settings.rs`
- Modify: `src/state.rs` (`ActiveTab::Settings`), `src/ui/screens/mod.rs`, `src/ui/shell/sidebar.rs`, `src/theme/icons.rs`, `src/main.rs` (`--tab settings`)
- Test: `src/ui/screens/settings.rs`

**Interfaces:**
- Consumes: `AppState::{settings, songs, loading, remove_music_folder, add_folder_via_dialog, rescan_library}`, `paths::get_covers_dir`, widgets `glass_panel`, `IconButton`, `PillButton`.
- Produces (privadas, probadas): `library_summary(songs: usize, folders: usize) -> String`; `format_bytes(bytes: u64) -> String`; `dir_size(dir: &Path) -> u64`; `clear_dir_files(dir: &Path) -> std::io::Result<u64>` (bytes liberados; conserva la carpeta); `SettingsUi` (con `confirm: Option<Confirm>`, `status: Option<(String, bool)>`, `begin_frame(&mut self, frame: u64)`), `Confirm { ClearHistory, FactoryReset }`. Pública: `settings::show(ui, state)`.

- [ ] **Step 1: Write the failing tests**

Crear `src/ui/screens/settings.rs` con `#![allow(dead_code)] // se quita en la Tarea 10` solo para `Confirm`/`SettingsUi` hasta que los use el bloque de Datos (si el compilador no avisa de nada, omitirlo), y estas pruebas:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("sp_cfgscreen_{}_{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn el_resumen_de_la_biblioteca_pluraliza() {
        assert_eq!(library_summary(0, 0), "Sin carpetas");
        assert_eq!(library_summary(628, 1), "628 canciones en 1 carpeta");
        assert_eq!(library_summary(1, 2), "1 canción en 2 carpetas");
        assert_eq!(library_summary(0, 3), "0 canciones en 3 carpetas");
    }

    #[test]
    fn los_bytes_se_muestran_legibles() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(12_582_912), "12.0 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.0 GB");
    }

    #[test]
    fn dir_size_suma_los_archivos_de_forma_recursiva_y_tolera_una_ruta_inexistente() {
        let dir = temp_dir("size");
        fs::write(dir.join("a"), vec![0u8; 100]).unwrap();
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("sub/b"), vec![0u8; 50]).unwrap();
        assert_eq!(dir_size(&dir), 150);
        assert_eq!(dir_size(&dir.join("no-existe")), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn clear_dir_files_borra_los_archivos_pero_conserva_la_carpeta() {
        let dir = temp_dir("clear");
        fs::write(dir.join("a.jpg"), vec![0u8; 100]).unwrap();
        fs::write(dir.join("b.jpg"), vec![0u8; 20]).unwrap();
        assert_eq!(clear_dir_files(&dir).unwrap(), 120);
        assert!(dir.is_dir());
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 0);
        assert_eq!(clear_dir_files(&dir.join("no-existe")).unwrap(), 0);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn la_confirmacion_se_cancela_si_la_pantalla_deja_de_verse() {
        let mut ui = SettingsUi::default();
        ui.begin_frame(10);
        ui.confirm = Some(Confirm::ClearHistory);
        ui.begin_frame(11);
        assert_eq!(ui.confirm, Some(Confirm::ClearHistory), "frames consecutivos la conservan");
        ui.begin_frame(15);
        assert_eq!(ui.confirm, None, "hubo frames sin mostrar la pantalla");
    }

    #[test]
    fn la_primera_vez_empieza_sin_confirmacion() {
        let mut ui = SettingsUi::default();
        ui.confirm = Some(Confirm::FactoryReset);
        ui.begin_frame(1);
        assert_eq!(ui.confirm, None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test ui::screens::settings 2>&1 | tail -10`
Expected: error de compilación `cannot find function library_summary` (RED). (Declarar `mod settings;` en `src/ui/screens/mod.rs` antes.)

- [ ] **Step 3: Write minimal implementation**

3a. `src/state.rs`: añadir `Settings` a `enum ActiveTab` (después de `Stats`). `src/main.rs`: en el `match tab.as_str()` de `--tab`, añadir `"settings" => state.select_tab(ActiveTab::Settings),`. `src/theme/icons.rs`: añadir `ARROW_CLOCKWISE, DOWNLOAD_SIMPLE, PLUS, TRASH, UPLOAD_SIMPLE, WARNING` al `pub use egui_phosphor::regular::{...}` (si algún nombre no existe, el compilador lo dice y se usa el equivalente de Phosphor).

3b. `src/ui/screens/mod.rs`: `mod settings;` y en `show`: `ActiveTab::Settings => settings::show(ui, state),` (antes del `_`).

3c. `src/ui/shell/sidebar.rs`: **eliminar** la sección "CARPETA" temporal y, tras el bucle de BIBLIOTECA, añadir:
```rust
    ui.add_space(space::XL);
    section_label(ui, "APLICACIÓN");
    if nav_item(ui, icons::GEAR, "Configuración", state.active_tab == ActiveTab::Settings).clicked() {
        state.select_tab(ActiveTab::Settings);
    }
```
(quitar imports que queden sin uso: `PillButton`, `PillKind`.)

3d. Implementar en `src/ui/screens/settings.rs` (antes de `#[cfg(test)]`):

```rust
//! Pantalla de Configuración: carpetas de música, preferencias, datos y acerca de.
use crate::paths::get_covers_dir;
use crate::state::AppState;
use crate::theme::{self, icons, radius, space, text};
use crate::ui::widgets::glass::{glass_panel, GlassKind};
use crate::ui::widgets::icon_button::IconButton;
use crate::ui::widgets::pill_button::{PillButton, PillKind};
use eframe::egui::{self, RichText};
use std::path::Path;

const GAP: f32 = 16.0;

/// Acción destructiva que espera confirmación en su propia fila.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Confirm {
    ClearHistory,
    FactoryReset,
}

/// Estado pasajero de la pantalla (vive en la memoria de egui, no en `AppState`).
#[derive(Clone, Default)]
pub struct SettingsUi {
    pub confirm: Option<Confirm>,
    /// Texto de la última acción y si es un error.
    pub status: Option<(String, bool)>,
    last_frame: Option<u64>,
}

impl SettingsUi {
    /// Cancela confirmaciones y estado si la pantalla estuvo sin mostrarse algún frame.
    pub fn begin_frame(&mut self, frame: u64) {
        if self.last_frame.map_or(true, |last| frame != last + 1 && frame != last) {
            self.confirm = None;
            self.status = None;
        }
        self.last_frame = Some(frame);
    }
}

pub fn library_summary(songs: usize, folders: usize) -> String {
    if folders == 0 {
        return "Sin carpetas".to_string();
    }
    format!(
        "{songs} {} en {folders} {}",
        if songs == 1 { "canción" } else { "canciones" },
        if folders == 1 { "carpeta" } else { "carpetas" }
    )
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b < KB {
        format!("{bytes} B")
    } else if b < KB * KB {
        format!("{:.1} KB", b / KB)
    } else if b < KB * KB * KB {
        format!("{:.1} MB", b / (KB * KB))
    } else {
        format!("{:.1} GB", b / (KB * KB * KB))
    }
}

/// Suma de los tamaños de todos los archivos bajo `dir` (0 si no existe).
pub fn dir_size(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir).into_iter().filter_map(|e| e.ok()).filter(|e| e.path().is_file()).filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum()
}

/// Borra los archivos de `dir` (conserva la carpeta) y devuelve los bytes liberados.
pub fn clear_dir_files(dir: &Path) -> std::io::Result<u64> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut freed = 0;
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_file() {
            freed += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            std::fs::remove_file(&path)?;
        }
    }
    Ok(freed)
}

pub fn show(ui: &mut egui::Ui, state: &mut AppState) {
    let mut view: SettingsUi = ui.ctx().data(|d| d.get_temp(egui::Id::new("settings_ui"))).unwrap_or_default();
    view.begin_frame(ui.ctx().frame_nr());

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.label(RichText::new("CONFIGURACIÓN").font(theme::deco(text::XL + 4.0)).color(theme::accent(ui.ctx())));
        ui.add_space(space::LG);
        library_section(ui, state, &mut view);
        ui.add_space(GAP);
    });

    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("settings_ui"), view));
}

fn section<R>(ui: &mut egui::Ui, title: &str, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    glass_panel(ui, GlassKind::Standard, radius::LG, space::XL, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).font(theme::deco(text::LG)).color(theme::accent(ui.ctx())));
        ui.add_space(space::MD);
        add_contents(ui)
    })
    .inner
}

/// Fila con etiqueta y descripción a la izquierda y el control a la derecha.
fn row(ui: &mut egui::Ui, label: &str, description: &str, add_control: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).font(theme::bold(text::BASE)));
            ui.label(RichText::new(description).size(text::SM).color(theme::TEXT_MUTED));
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add_control);
    });
}

/// Tamaño de la caché de carátulas, recalculado como mucho cada 5 s (recorrer miles de archivos cada frame sería caro).
fn covers_cache_size(ui: &egui::Ui) -> u64 {
    let id = egui::Id::new("covers_cache_size");
    let now = ui.input(|i| i.time);
    if let Some((at, size)) = ui.ctx().data(|d| d.get_temp::<(f64, u64)>(id)) {
        if now - at < 5.0 {
            return size;
        }
    }
    let size = dir_size(&get_covers_dir());
    ui.ctx().data_mut(|d| d.insert_temp(id, (now, size)));
    size
}

fn library_section(ui: &mut egui::Ui, state: &mut AppState, _view: &mut SettingsUi) {
    section(ui, "BIBLIOTECA", |ui| {
        ui.label(RichText::new(library_summary(state.songs.len(), state.settings.music_folders.len())).color(theme::TEXT_MUTED));
        ui.add_space(space::SM);

        let mut remove: Option<String> = None;
        for folder in &state.settings.music_folders {
            let exists = Path::new(folder).is_dir();
            let total = ui.available_width();
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(egui::vec2((total - 190.0).max(80.0), 28.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(RichText::new(icons::FOLDER_OPEN).size(text::LG).color(theme::TEXT_MUTED));
                    ui.add(egui::Label::new(RichText::new(folder)).truncate()).on_hover_text(folder);
                });
                if !exists {
                    ui.label(RichText::new("No se encuentra").size(text::SM).color(theme::ACCENT_SUNSET));
                }
                if IconButton::new(icons::X, 28.0).tooltip("Quitar carpeta").show(ui).clicked() {
                    remove = Some(folder.clone());
                }
            });
        }
        if let Some(folder) = remove {
            state.remove_music_folder(&folder);
        }

        ui.add_space(space::MD);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(!state.loading, |ui| {
                if PillButton::new("Agregar carpeta", PillKind::Primary).icon(icons::PLUS).show(ui).clicked() {
                    state.add_folder_via_dialog();
                }
                if PillButton::new("Re-escanear", PillKind::Secondary).icon(icons::ARROW_CLOCKWISE).show(ui).clicked() {
                    state.rescan_library();
                }
            });
        });

        ui.add_space(space::LG);
        let size = covers_cache_size(ui);
        row(ui, "Caché de carátulas", &format!("Ocupa {}. Se vuelve a generar al re-escanear.", format_bytes(size)), |ui| {
            if PillButton::new("Limpiar", PillKind::Secondary).icon(icons::TRASH).show(ui).clicked() {
                let _ = clear_dir_files(&get_covers_dir());
                ui.ctx().data_mut(|d| d.remove::<(f64, u64)>(egui::Id::new("covers_cache_size")));
                state.rescan_library();
            }
        });
    });
}
```

(El parámetro `_view` se usa en la Tarea 10; `Confirm` y los campos de `SettingsUi` quedan usados por las pruebas hasta entonces: si el compilador avisa de código sin uso, mantener el `#![allow(dead_code)]` indicado arriba y quitarlo en la Tarea 10.)

- [ ] **Step 4: Run tests, build and look at it**

Run: `cargo test 2>&1 | grep -E "^test result|^error|^warning" -A5; cargo build --release 2>&1 | grep -E "^(error|warning)" -A6`
Expected: suite en verde (6 pruebas nuevas) y sin advertencias. Capturar y revisar:

```bash
S=/tmp/claude-1000/-home-budja8-Documents-Proyectos-simple-player-master/5badf05a-e152-478c-aeb0-3922f4d01526/scratchpad
for w in 1050x750 1000x650; do
  timeout 40 ./target/release/simple-player --allow-multiple --no-hotkeys --window-size $w --tab settings --music-folder "$HOME/Music" --music-folder /no/existe --shot $S/cfg_$w.png >/dev/null 2>&1
done
```
Leer con Read: el sidebar ya muestra "Configuración" y no la sección "Carpeta"; la tarjeta Biblioteca lista las dos carpetas (la segunda con "No se encuentra"), botones y la fila de caché. Ajustar márgenes/anchos si algo se corta.

- [ ] **Step 5: Commit**

```bash
git add -A src
git commit -m "Pantalla de Configuración: navegación y sección Biblioteca

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Configuración — Apariencia y Acerca de

**Files:**
- Create: `src/ui/widgets/toggle.rs`
- Modify: `src/ui/widgets/mod.rs` (declarar `pub mod toggle;`), `src/ui/screens/settings.rs`
- Test: `src/ui/screens/settings.rs` (módulo `tests`)

**Interfaces:**
- Consumes: `AppState::update_settings`, `VisualizerMode`, `chip`, `paths::get_cache_dir`, `stats::location::default_db_path`, `settings::settings_path`.
- Produces: `widgets::toggle::toggle(ui, &mut bool) -> egui::Response` (cambia el valor al clic y marca `changed`); ayudantes puros de la pantalla `visualizer_options() -> [VisualizerMode; 7]` y `data_dirs(xdg_config, xdg_data, home) -> DataDirs { settings: PathBuf, stats: PathBuf, cache: PathBuf }`.

- [ ] **Step 1: Write the failing tests**

Añadir a `mod tests` de `src/ui/screens/settings.rs`:

```rust
    #[test]
    fn el_selector_ofrece_los_siete_modos_en_el_orden_del_ciclo() {
        let labels: Vec<&str> = visualizer_options().iter().map(|m| m.label()).collect();
        assert_eq!(labels, ["Barras", "Anillo", "Partículas", "Constelación", "Osciloscopio", "Franja", "Apagado"]);
    }

    #[test]
    fn las_carpetas_de_datos_respetan_xdg() {
        let d = data_dirs(Some("/cfg"), Some("/data"), Some("/home/u"));
        assert_eq!(d.settings, std::path::PathBuf::from("/cfg/simple-player"));
        assert_eq!(d.stats, std::path::PathBuf::from("/data/simple-player"));
        let d = data_dirs(None, None, Some("/home/u"));
        assert_eq!(d.settings, std::path::PathBuf::from("/home/u/.config/simple-player"));
        assert_eq!(d.stats, std::path::PathBuf::from("/home/u/.local/share/simple-player"));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test ui::screens::settings 2>&1 | tail -8`
Expected: error `cannot find function visualizer_options` (RED).

- [ ] **Step 3: Write minimal implementation**

`src/ui/widgets/toggle.rs`:

```rust
use crate::theme::{self, with_alpha};
use eframe::egui;

/// Interruptor de píldora: se enciende con el acento dinámico.
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(egui::vec2(44.0, 24.0), egui::Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let t = ui.ctx().animate_bool_with_time(response.id, *on, theme::motion::HOVER);
        let accent = theme::accent(ui.ctx());
        let painter = ui.painter();
        let fill = theme::lerp_color(egui::Color32::from_white_alpha(30), with_alpha(accent, 200), t);
        painter.rect_filled(rect, rect.height() / 2.0, fill);
        painter.rect_stroke(rect, rect.height() / 2.0, egui::Stroke::new(1.0_f32, theme::GLASS_BORDER));
        let knob_x = rect.left() + 12.0 + t * (rect.width() - 24.0);
        painter.circle_filled(egui::pos2(knob_x, rect.center().y), 8.0, egui::Color32::WHITE);
    }
    response
}
```

`src/ui/screens/settings.rs` — añadir imports (`crate::ui::visualizers::VisualizerMode`, `crate::ui::widgets::chip::chip`, `crate::ui::widgets::toggle::toggle`, `std::path::PathBuf`) y:

```rust
pub fn visualizer_options() -> [VisualizerMode; 7] {
    let mut modes = [VisualizerMode::Bars; 7];
    for i in 1..7 {
        modes[i] = modes[i - 1].next();
    }
    modes
}

pub struct DataDirs {
    pub settings: PathBuf,
    pub stats: PathBuf,
    pub cache: PathBuf,
}

/// Carpetas donde la app guarda sus datos (para mostrarlas y abrirlas).
pub fn data_dirs(xdg_config: Option<&str>, xdg_data: Option<&str>, home: Option<&str>) -> DataDirs {
    let parent = |p: PathBuf| p.parent().map(Path::to_path_buf).unwrap_or(p);
    DataDirs {
        settings: parent(crate::settings::settings_path(xdg_config, home)),
        stats: parent(crate::stats::location::default_db_path(xdg_data, home)),
        cache: crate::paths::get_cache_dir(),
    }
}

/// Abre una carpeta con el explorador del sistema; si falla no pasa nada.
fn open_folder(path: &Path) {
    let _ = std::process::Command::new("xdg-open").arg(path).spawn();
}

fn appearance_section(ui: &mut egui::Ui, state: &mut AppState) {
    section(ui, "APARIENCIA Y VENTANA", |ui| {
        ui.label(RichText::new("Visualizador por defecto").font(theme::bold(text::BASE)));
        ui.label(RichText::new("Con el que abre la pantalla completa. Cambiar de modo allí no modifica este valor.").size(text::SM).color(theme::TEXT_MUTED));
        ui.add_space(space::SM);
        let current = state.settings.visualizer_mode();
        let mut picked = None;
        ui.horizontal_wrapped(|ui| {
            for mode in visualizer_options() {
                if chip(ui, mode.label(), mode == current).clicked() {
                    picked = Some(mode);
                }
            }
        });
        if let Some(mode) = picked {
            state.update_settings(|s| s.default_visualizer = mode.label().to_string());
        }

        ui.add_space(space::LG);
        let mut remember = state.settings.remember_window_size;
        row(ui, "Recordar el tamaño de la ventana", "Se aplica la próxima vez que abras la app.", |ui| {
            toggle(ui, &mut remember);
        });
        if remember != state.settings.remember_window_size {
            state.update_settings(|s| s.remember_window_size = remember);
        }
    });
}

fn about_section(ui: &mut egui::Ui) {
    section(ui, "ACERCA DE", |ui| {
        ui.label(RichText::new(format!("Simple Player {}", env!("CARGO_PKG_VERSION"))).font(theme::bold(text::BASE)));
        ui.add_space(space::SM);
        let dirs = data_dirs(
            std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
            std::env::var("XDG_DATA_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        );
        for (label, dir) in [("Ajustes", &dirs.settings), ("Estadísticas", &dirs.stats), ("Caché", &dirs.cache)] {
            row(ui, label, &dir.display().to_string(), |ui| {
                if PillButton::new("Abrir carpeta", PillKind::Ghost).icon(icons::FOLDER_OPEN).show(ui).clicked() {
                    open_folder(dir);
                }
            });
            ui.add_space(space::XS);
        }
    });
}
```
y en `show`, después de `library_section(...)`: `ui.add_space(GAP); appearance_section(ui, state); ui.add_space(GAP); about_section(ui); ui.add_space(GAP);`.

- [ ] **Step 4: Run tests, build and look at it**

Run: `cargo test 2>&1 | grep -E "^test result|^error|^warning" -A5; cargo build --release 2>&1 | grep -E "^(error|warning)" -A6`, luego capturar `--tab settings` a 1050×750 y 1000×650 y leerlas: los siete chips caben (se envuelven si hace falta), el interruptor se ve encendido por defecto y las rutas largas se truncan o se parten sin romper la tarjeta.
Expected: suite en verde (2 pruebas nuevas) y sin advertencias.

- [ ] **Step 5: Commit**

```bash
git add -A src
git commit -m "Configuración: apariencia, recordar tamaño de ventana y acerca de

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Formato de exportación/importación y funciones del almacén

**Files:**
- Create: `src/stats/transfer.rs`
- Modify: `src/stats/mod.rs` (declarar `pub mod transfer;`), `src/stats/store.rs`
- Test: `src/stats/transfer.rs`, `src/stats/store.rs`

**Interfaces:**
- Consumes: `PlayEvent`, `SongSnapshot` (`stats::model`).
- Produces: `transfer::{to_json(events: &[PlayEvent], exported_at: i64) -> String, parse(json: &str) -> Result<Parsed, ImportError>}`; `Parsed { events: Vec<PlayEvent>, invalid: usize }`; `ImportError { NotJson(String), WrongApp, UnsupportedVersion(u32) }` con `impl std::fmt::Display` en español; `Store::all_events(&self) -> StoreResult<Vec<PlayEvent>>`; `Store::import_events(&mut self, events: &[PlayEvent]) -> StoreResult<ImportReport>` con `ImportReport { imported: usize, duplicates: usize }`; `Store::clear(&mut self) -> StoreResult<()>`.

- [ ] **Step 1: Write the failing tests**

`src/stats/transfer.rs` (con `pub mod transfer;` en `src/stats/mod.rs`):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::model::SongSnapshot;

    fn event(started: i64, ended: i64, listened: u64, path: &str) -> PlayEvent {
        PlayEvent {
            started_at: started,
            ended_at: ended,
            listened_ms: listened,
            song: SongSnapshot { path: path.into(), title: "T".into(), artist: "A".into(), album: "B".into(), duration_ms: 200_000 },
        }
    }

    #[test]
    fn exportar_y_leer_conserva_los_eventos() {
        let events = vec![event(1_000, 61_000, 60_000, "/a.mp3"), event(70_000, 130_000, 55_000, "/b.mp3")];
        let parsed = parse(&to_json(&events, 123)).unwrap();
        assert_eq!(parsed.events, events);
        assert_eq!(parsed.invalid, 0);
    }

    #[test]
    fn el_archivo_lleva_app_version_y_fecha() {
        let json = to_json(&[], 42);
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v["app"], "simple-player");
        assert_eq!(v["version"], 1);
        assert_eq!(v["exported_at"], 42);
        assert_eq!(v["events"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn una_lista_vacia_es_valida() {
        let parsed = parse(&to_json(&[], 1)).unwrap();
        assert!(parsed.events.is_empty());
    }

    #[test]
    fn un_json_roto_o_de_otra_app_se_rechaza() {
        assert!(matches!(parse("{ no es json"), Err(ImportError::NotJson(_))));
        assert!(matches!(parse(r#"{"app":"otra","version":1,"exported_at":0,"events":[]}"#), Err(ImportError::WrongApp)));
        assert!(matches!(parse(r#"{"version":1,"events":[]}"#), Err(ImportError::WrongApp | ImportError::NotJson(_))));
    }

    #[test]
    fn una_version_mayor_se_rechaza() {
        assert!(matches!(parse(r#"{"app":"simple-player","version":2,"exported_at":0,"events":[]}"#), Err(ImportError::UnsupportedVersion(2))));
    }

    #[test]
    fn los_eventos_invalidos_se_cuentan_y_se_omiten() {
        let json = to_json(
            &[
                event(1_000, 61_000, 60_000, "/ok.mp3"),
                event(1_000, 61_000, 0, "/sin-tiempo.mp3"),
                event(61_000, 1_000, 60_000, "/fin-antes-que-inicio.mp3"),
                event(-5, 61_000, 60_000, "/negativo.mp3"),
                event(1_000, 61_000, u64::MAX, "/desbordado.mp3"),
            ],
            1,
        );
        let parsed = parse(&json).unwrap();
        assert_eq!(parsed.events.len(), 1);
        assert_eq!(parsed.invalid, 4);
        assert_eq!(parsed.events[0].song.path, "/ok.mp3");
    }

    #[test]
    fn los_errores_se_explican_en_espanol() {
        assert!(ImportError::WrongApp.to_string().contains("Simple Player"));
        assert!(ImportError::UnsupportedVersion(9).to_string().contains("versión"));
    }
}
```

Añadir a `mod tests` de `src/stats/store.rs` (leer primero el módulo existente para reutilizar su helper de eventos; si no hay, usar el de abajo):

```rust
    fn ev(started: i64, ended: i64, listened: u64, path: &str) -> PlayEvent {
        PlayEvent {
            started_at: started,
            ended_at: ended,
            listened_ms: listened,
            song: SongSnapshot { path: path.into(), title: "T".into(), artist: "A".into(), album: "B".into(), duration_ms: 1000 },
        }
    }

    #[test]
    fn all_events_devuelve_lo_guardado_en_orden() {
        let mut store = Store::open_in_memory().unwrap();
        store.insert_batch(&[ev(100, 200, 100, "/b"), ev(10, 20, 10, "/a")]).unwrap();
        let all = store.all_events().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].song.path, "/a", "ordenados por fin");
    }

    #[test]
    fn importar_en_una_base_vacia_inserta_todo() {
        let mut store = Store::open_in_memory().unwrap();
        let report = store.import_events(&[ev(10, 20, 10, "/a"), ev(30, 40, 10, "/b")]).unwrap();
        assert_eq!((report.imported, report.duplicates), (2, 0));
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn importar_dos_veces_no_duplica() {
        let mut store = Store::open_in_memory().unwrap();
        let events = [ev(10, 20, 10, "/a"), ev(30, 40, 10, "/b")];
        store.import_events(&events).unwrap();
        let again = store.import_events(&events).unwrap();
        assert_eq!((again.imported, again.duplicates), (0, 2));
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn los_repetidos_dentro_del_mismo_archivo_se_cuentan_como_duplicados() {
        let mut store = Store::open_in_memory().unwrap();
        let report = store.import_events(&[ev(10, 20, 10, "/a"), ev(10, 20, 10, "/a")]).unwrap();
        assert_eq!((report.imported, report.duplicates), (1, 1));
    }

    #[test]
    fn un_fallo_a_mitad_del_lote_no_deja_eventos_parciales() {
        let mut store = Store::open_in_memory().unwrap();
        // `u64::MAX` como i64 es -1 y viola CHECK (listened_ms > 0) en pleno lote.
        let result = store.import_events(&[ev(10, 20, 10, "/a"), ev(30, 40, u64::MAX, "/mala"), ev(50, 60, 10, "/c")]);
        assert!(result.is_err());
        assert_eq!(store.count().unwrap(), 0, "la transacción debe revertirse entera");
    }

    #[test]
    fn clear_deja_la_tabla_vacia() {
        let mut store = Store::open_in_memory().unwrap();
        store.insert_batch(&[ev(10, 20, 10, "/a")]).unwrap();
        store.clear().unwrap();
        assert_eq!(store.count().unwrap(), 0);
        store.insert(&ev(10, 20, 10, "/a")).unwrap();
        assert_eq!(store.count().unwrap(), 1, "sigue funcionando después de limpiar");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test stats 2>&1 | grep -E "^error" -A3 | head -10`
Expected: error de compilación (`cannot find function parse`, `no method named all_events` …) (RED).

- [ ] **Step 3: Write minimal implementation**

`src/stats/transfer.rs` (antes de `#[cfg(test)]`):

```rust
//! Formato de exportación/importación del historial (JSON versionado). Sin acceso a disco ni a SQLite.
use super::model::{PlayEvent, SongSnapshot};
use serde::{Deserialize, Serialize};
use std::fmt;

pub const APP: &str = "simple-player";
pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct ExportFile {
    #[serde(default)]
    app: String,
    #[serde(default)]
    version: u32,
    #[serde(default)]
    exported_at: i64,
    #[serde(default)]
    events: Vec<ExportEvent>,
}

#[derive(Serialize, Deserialize)]
struct ExportEvent {
    started_at: i64,
    ended_at: i64,
    listened_ms: u64,
    path: String,
    title: String,
    artist: String,
    album: String,
    duration_ms: u64,
}

pub struct Parsed {
    pub events: Vec<PlayEvent>,
    /// Eventos que se omitieron por no ser válidos.
    pub invalid: usize,
}

#[derive(Debug)]
pub enum ImportError {
    NotJson(String),
    WrongApp,
    UnsupportedVersion(u32),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImportError::NotJson(e) => write!(f, "El archivo no es un historial válido: {e}"),
            ImportError::WrongApp => write!(f, "El archivo no es un historial exportado por Simple Player."),
            ImportError::UnsupportedVersion(v) => write!(f, "El archivo es de una versión más nueva ({v}) que la que entiende esta app."),
        }
    }
}

pub fn to_json(events: &[PlayEvent], exported_at: i64) -> String {
    let file = ExportFile {
        app: APP.to_string(),
        version: VERSION,
        exported_at,
        events: events
            .iter()
            .map(|e| ExportEvent {
                started_at: e.started_at,
                ended_at: e.ended_at,
                listened_ms: e.listened_ms,
                path: e.song.path.clone(),
                title: e.song.title.clone(),
                artist: e.song.artist.clone(),
                album: e.song.album.clone(),
                duration_ms: e.song.duration_ms,
            })
            .collect(),
    };
    serde_json::to_string_pretty(&file).unwrap_or_else(|_| "{}".to_string())
}

fn is_valid(e: &ExportEvent) -> bool {
    e.listened_ms > 0 && e.listened_ms <= i64::MAX as u64 && e.duration_ms <= i64::MAX as u64 && e.started_at >= 0 && e.ended_at >= 0 && e.ended_at >= e.started_at
}

pub fn parse(json: &str) -> Result<Parsed, ImportError> {
    let file: ExportFile = serde_json::from_str(json).map_err(|e| ImportError::NotJson(e.to_string()))?;
    if file.app != APP {
        return Err(ImportError::WrongApp);
    }
    if file.version > VERSION {
        return Err(ImportError::UnsupportedVersion(file.version));
    }
    let mut events = Vec::new();
    let mut invalid = 0;
    for e in file.events {
        if !is_valid(&e) {
            invalid += 1;
            continue;
        }
        events.push(PlayEvent {
            started_at: e.started_at,
            ended_at: e.ended_at,
            listened_ms: e.listened_ms,
            song: SongSnapshot { path: e.path, title: e.title, artist: e.artist, album: e.album, duration_ms: e.duration_ms },
        });
    }
    Ok(Parsed { events, invalid })
}
```

`src/stats/store.rs` (dentro de `impl Store`; añadir `use std::collections::HashSet;` y `use super::model::SongSnapshot;` si faltan):

```rust
    /// Todos los eventos, ordenados por fin (para exportar).
    pub fn all_events(&self) -> StoreResult<Vec<PlayEvent>> {
        let mut stmt = self.conn.prepare(
            "SELECT started_at, ended_at, listened_ms, song_path, title, artist, album, duration_ms FROM play_events ORDER BY ended_at, id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(PlayEvent {
                started_at: r.get(0)?,
                ended_at: r.get(1)?,
                listened_ms: r.get::<_, i64>(2)? as u64,
                song: SongSnapshot {
                    path: r.get(3)?,
                    title: r.get(4)?,
                    artist: r.get(5)?,
                    album: r.get(6)?,
                    duration_ms: r.get::<_, i64>(7)? as u64,
                },
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Inserta los eventos que no estén ya (mismo inicio, fin y ruta) en **una sola transacción**:
    /// si algo falla, no queda ninguno.
    pub fn import_events(&mut self, events: &[PlayEvent]) -> StoreResult<ImportReport> {
        let tx = self.conn.transaction()?;
        let mut report = ImportReport::default();
        {
            let mut known: HashSet<(i64, i64, String)> = {
                let mut stmt = tx.prepare("SELECT started_at, ended_at, song_path FROM play_events")?;
                let rows = stmt.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))?;
                rows.collect::<Result<_, _>>()?
            };
            let mut insert = tx.prepare(INSERT_SQL)?;
            for event in events {
                let Some((started, ended)) = Self::normalized(event) else { continue };
                if !known.insert((started, ended, event.song.path.clone())) {
                    report.duplicates += 1;
                    continue;
                }
                Self::execute_insert(&mut insert, event, started, ended)?;
                report.imported += 1;
            }
        }
        tx.commit()?;
        Ok(report)
    }

    /// Borra todo el historial y recupera el espacio.
    pub fn clear(&mut self) -> StoreResult<()> {
        self.conn.execute("DELETE FROM play_events", [])?;
        self.conn.execute_batch("VACUUM")?;
        Ok(())
    }
```
y, a nivel de módulo en `store.rs`:
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub duplicates: usize,
}
```
(Si `StoreError` no implementa `From<rusqlite::Error>` ya lo hace — el resto del archivo usa `?` igual.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test stats 2>&1 | tail -8; cargo test 2>&1 | grep -E "^test result|^warning" -A3`
Expected: `7` pruebas nuevas de `transfer` + `6` de `store` en verde y suite completa verde. Las funciones sin uso emitirán advertencias de `dead_code` hasta la Tarea 9: añadir `#[allow(dead_code)] // se conecta en la Tarea 9` solo a `all_events`, `import_events`, `clear`, `ImportReport` y a `transfer::{to_json, parse, ImportError, Parsed}` (y quitarlos en la Tarea 9).

- [ ] **Step 5: Commit**

```bash
git add -A src/stats
git commit -m "Historial: formato de exportación, importación transaccional y borrado

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Acciones de historial en el servicio de estadísticas

**Files:**
- Modify: `src/stats/service.rs`, `src/stats/store.rs`, `src/stats/transfer.rs` (quitar los `allow(dead_code)`)
- Test: `src/stats/service.rs`

**Interfaces:**
- Consumes: `Store::{all_events, import_events, clear}`, `transfer::{to_json, parse}`.
- Produces: `DataOutcome { Exported(usize), Imported { imported: usize, duplicates: usize, invalid: usize }, Cleared, Failed(String) }`; `StatsHandle::export_to(&self, path: PathBuf)`; `StatsHandle::import_from(&self, path: PathBuf)`; `StatsHandle::clear_history(&self)`; `StatsHandle::take_data_outcome(&self) -> Option<DataOutcome>` (lo devuelve una sola vez). Importar con éxito (`imported > 0`) y borrar suben `events_version`.

- [ ] **Step 1: Write the failing tests**

Añadir a `mod tests` de `src/stats/service.rs`:

```rust
    fn temp_file(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("sp_svc_{}_{name}", std::process::id()))
    }

    fn wait_outcome(h: &StatsHandle) -> DataOutcome {
        h.flush(Duration::from_secs(2));
        h.take_data_outcome().expect("hay resultado")
    }

    #[test]
    fn exportar_e_importar_en_otra_base_reproduce_el_historial() {
        let file = temp_file("export.json");
        let _ = std::fs::remove_file(&file);
        let a = StatsHandle::spawn(StatsLocation::Memory, noop());
        a.record(event("/a"));
        a.record(event("/b"));
        a.flush(Duration::from_secs(2));
        a.export_to(file.clone());
        assert!(matches!(wait_outcome(&a), DataOutcome::Exported(2)));

        let b = StatsHandle::spawn(StatsLocation::Memory, noop());
        b.import_from(file.clone());
        assert!(matches!(wait_outcome(&b), DataOutcome::Imported { imported: 2, duplicates: 0, invalid: 0 }));
        b.request_summary(StatsRange::All);
        b.flush(Duration::from_secs(2));
        assert_eq!(b.latest_summary().unwrap().1.totals.plays, 2);
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn importar_dos_veces_no_duplica_y_lo_dice() {
        let file = temp_file("twice.json");
        let a = StatsHandle::spawn(StatsLocation::Memory, noop());
        a.record(event("/a"));
        a.flush(Duration::from_secs(2));
        a.export_to(file.clone());
        let _ = wait_outcome(&a);
        a.import_from(file.clone());
        assert!(matches!(wait_outcome(&a), DataOutcome::Imported { imported: 0, duplicates: 1, invalid: 0 }));
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn un_archivo_corrupto_o_inexistente_da_un_error_y_no_inserta_nada() {
        let file = temp_file("bad.json");
        std::fs::write(&file, "{ corrupto").unwrap();
        let h = StatsHandle::spawn(StatsLocation::Memory, noop());
        h.import_from(file.clone());
        assert!(matches!(wait_outcome(&h), DataOutcome::Failed(_)));
        h.import_from(temp_file("no-existe.json"));
        assert!(matches!(wait_outcome(&h), DataOutcome::Failed(_)));
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_secs(2));
        assert_eq!(h.latest_summary().unwrap().1.totals.plays, 0);
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn exportar_a_una_ruta_imposible_da_un_error() {
        let h = StatsHandle::spawn(StatsLocation::Memory, noop());
        h.export_to(PathBuf::from("/proc/no-existe/historial.json"));
        assert!(matches!(wait_outcome(&h), DataOutcome::Failed(_)));
    }

    #[test]
    fn borrar_el_historial_lo_vacia_y_sube_la_version() {
        let h = StatsHandle::spawn(StatsLocation::Memory, noop());
        h.record(event("/a"));
        h.flush(Duration::from_secs(2));
        let before = h.events_version();
        h.clear_history();
        assert!(matches!(wait_outcome(&h), DataOutcome::Cleared));
        assert!(h.events_version() > before);
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_secs(2));
        assert_eq!(h.latest_summary().unwrap().1.totals.plays, 0);
    }

    #[test]
    fn un_servicio_deshabilitado_ignora_las_acciones_sin_entrar_en_panico() {
        let h = StatsHandle::disabled("prueba");
        h.export_to(PathBuf::from("/tmp/x.json"));
        h.import_from(PathBuf::from("/tmp/x.json"));
        h.clear_history();
        assert!(h.take_data_outcome().is_none());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test stats::service 2>&1 | grep -E "^error" -A3 | head -8`
Expected: error de compilación (`cannot find type DataOutcome`, `no method named export_to`) (RED).

- [ ] **Step 3: Write minimal implementation**

En `src/stats/service.rs`:

```rust
use super::transfer;
use std::path::PathBuf;

/// Resultado de la última acción de datos (exportar, importar o borrar), para mostrarlo en la pantalla.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DataOutcome {
    Exported(usize),
    Imported { imported: usize, duplicates: usize, invalid: usize },
    Cleared,
    Failed(String),
}
```
- `enum Message`: añadir `Export(PathBuf)`, `Import(PathBuf)`, `Clear`.
- `struct Shared`: añadir `data_outcome: Mutex<Option<DataOutcome>>`.
- En `impl StatsHandle`:
  ```rust
  pub fn export_to(&self, path: PathBuf) { self.send(Message::Export(path)); }
  pub fn import_from(&self, path: PathBuf) { self.send(Message::Import(path)); }
  pub fn clear_history(&self) { self.send(Message::Clear); }

  /// El resultado de la última acción de datos; se entrega una sola vez.
  pub fn take_data_outcome(&self) -> Option<DataOutcome> {
      self.shared.data_outcome.lock().unwrap_or_else(|e| e.into_inner()).take()
  }
  ```
- En `worker`, antes de `Message::Flush`:
  ```rust
            Message::Export(path) => {
                let outcome = match store.all_events() {
                    Ok(events) => match std::fs::write(&path, transfer::to_json(&events, chrono::Utc::now().timestamp_millis())) {
                        Ok(()) => DataOutcome::Exported(events.len()),
                        Err(e) => DataOutcome::Failed(format!("No se pudo guardar el archivo: {e}")),
                    },
                    Err(e) => DataOutcome::Failed(format!("No se pudo leer el historial: {}", e.0)),
                };
                publish(&shared, outcome, &repaint);
            }
            Message::Import(path) => {
                let outcome = match std::fs::read_to_string(&path) {
                    Err(e) => DataOutcome::Failed(format!("No se pudo leer el archivo: {e}")),
                    Ok(json) => match transfer::parse(&json) {
                        Err(e) => DataOutcome::Failed(e.to_string()),
                        Ok(parsed) => match store.import_events(&parsed.events) {
                            Ok(report) => {
                                if report.imported > 0 {
                                    shared.events_version.fetch_add(1, Ordering::SeqCst);
                                }
                                DataOutcome::Imported { imported: report.imported, duplicates: report.duplicates, invalid: parsed.invalid }
                            }
                            Err(e) => DataOutcome::Failed(format!("No se pudo importar: {}", e.0)),
                        },
                    },
                };
                publish(&shared, outcome, &repaint);
            }
            Message::Clear => {
                let outcome = match store.clear() {
                    Ok(()) => {
                        shared.events_version.fetch_add(1, Ordering::SeqCst);
                        DataOutcome::Cleared
                    }
                    Err(e) => DataOutcome::Failed(format!("No se pudo borrar el historial: {}", e.0)),
                };
                publish(&shared, outcome, &repaint);
            }
  ```
  y la función auxiliar:
  ```rust
  fn publish(shared: &Shared, outcome: DataOutcome, repaint: &Repaint) {
      *shared.data_outcome.lock().unwrap_or_else(|e| e.into_inner()) = Some(outcome);
      repaint();
  }
  ```
- Quitar los `#[allow(dead_code)]` de la Tarea 8 (`all_events`, `import_events`, `clear`, `ImportReport`, `transfer::*`).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test stats 2>&1 | tail -6; cargo test 2>&1 | grep -E "^test result|^(error|warning)" -A4`
Expected: 6 pruebas nuevas en verde, suite completa verde y sin advertencias.

- [ ] **Step 5: Commit**

```bash
git add -A src/stats
git commit -m "El servicio de estadísticas exporta, importa y borra el historial

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 10: Restablecer de fábrica y sección Datos

**Files:**
- Create: `src/reset.rs`
- Modify: `src/main.rs` (declarar `mod reset;`), `src/state.rs`, `src/ui/screens/settings.rs`
- Test: `src/reset.rs`, `src/ui/screens/settings.rs`

**Interfaces:**
- Consumes: `DataOutcome`, `StatsHandle::{export_to, import_from, clear_history, take_data_outcome, disabled_reason}`, `paths::{get_library_cache_path, get_playback_state_path, get_covers_dir}`, `window_state::window_state_path`, `Settings::default`.
- Produces: `reset::remove_all(files: &[PathBuf], dirs: &[PathBuf]) -> ResetReport { removed: usize, errors: Vec<String> }`; `reset::factory_paths(settings_file: &Path, window_file: &Path) -> (Vec<PathBuf>, Vec<PathBuf>)`; `AppState::factory_reset(&mut self) -> Result<usize, String>` (solo si hay `settings_path`); `AppState::can_factory_reset(&self) -> bool`; ayudantes de la pantalla `outcome_message(&DataOutcome) -> (String, bool)` y `export_file_name(date: chrono::NaiveDate) -> String`.

- [ ] **Step 1: Write the failing tests**

`src/reset.rs` (con `mod reset;` en `src/main.rs`):

```rust
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
        let (settings, window, playback, cache) = (root.join("settings.json"), root.join("window.json"), root.join("playback_state.json"), root.join("library_cache.json"));
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
```

Añadir a `mod tests` de `src/ui/screens/settings.rs`:

```rust
    #[test]
    fn los_resultados_se_explican_y_marcan_los_errores() {
        use crate::stats::service::DataOutcome;
        assert_eq!(outcome_message(&DataOutcome::Exported(128)), ("128 eventos exportados".to_string(), false));
        assert_eq!(outcome_message(&DataOutcome::Exported(1)).0, "1 evento exportado");
        assert_eq!(outcome_message(&DataOutcome::Imported { imported: 12, duplicates: 3, invalid: 0 }).0, "12 importados, 3 repetidos");
        assert_eq!(outcome_message(&DataOutcome::Imported { imported: 1, duplicates: 0, invalid: 2 }).0, "1 importado, 0 repetidos, 2 inválidos");
        assert_eq!(outcome_message(&DataOutcome::Cleared), ("Historial borrado".to_string(), false));
        let (text, is_error) = outcome_message(&DataOutcome::Failed("sin permisos".into()));
        assert!(is_error && text.contains("sin permisos"));
    }

    #[test]
    fn el_nombre_sugerido_lleva_la_fecha() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 3).unwrap();
        assert_eq!(export_file_name(date), "simple-player-historial-2026-10-03.json");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test reset 2>&1 | grep -E "^error" -A3 | head -6; cargo test ui::screens::settings 2>&1 | grep -E "^error" -A3 | head -6`
Expected: errores de compilación (`cannot find function remove_all` / `outcome_message`) (RED).

- [ ] **Step 3: Write minimal implementation**

`src/reset.rs` (antes de `#[cfg(test)]`):

```rust
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
```
**Cuidado con la prueba `la_lista_de_fabrica...`:** `get_covers_dir()` crea la carpeta en `~/.cache` al llamarse; es el comportamiento existente de `paths`, no se cambia.

`src/state.rs`:

```rust
    /// El restablecimiento de fábrica toca archivos reales: solo existe en ejecuciones normales.
    pub fn can_factory_reset(&self) -> bool {
        self.settings_path.is_some()
    }

    /// Borra los datos de la app (ver `reset`), detiene la reproducción y vuelve a los valores por defecto
    /// en memoria. Conserva el historial de estadísticas. Devuelve cuántas cosas se borraron.
    pub fn factory_reset(&mut self) -> Result<usize, String> {
        let Some(settings_file) = self.settings_path.clone() else {
            return Err("No disponible en ejecuciones de desarrollo".to_string());
        };
        let window_file = crate::window_state::window_state_path(
            std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        );
        let (files, dirs) = crate::reset::factory_paths(&settings_file, &window_file);
        let report = crate::reset::remove_all(&files, &dirs);

        let _ = self.audio.pause();
        self.is_playing = false;
        self.current_song_index = None;
        self.active_queue.clear();
        self.settings = Settings::default();
        self.set_volume(0.8);
        self.is_shuffle = false;
        self.repeat_mode = RepeatMode::Off;
        self.set_songs(Vec::new());

        if report.errors.is_empty() { Ok(report.removed) } else { Err(report.errors.join("; ")) }
    }
```
(Leer `observe_stats_at`/`stats.song_*` en `state.rs` para confirmar que dejar `is_playing = false` y la cola vacía cierra la sesión de escucha en curso sin pánico; si hay un método explícito para cerrar la sesión al detener, llamarlo aquí. No persistir: `persist()` no se llama para no recrear `playback_state.json`.)

`src/ui/screens/settings.rs` — añadir:

```rust
use crate::stats::service::DataOutcome;

pub fn outcome_message(outcome: &DataOutcome) -> (String, bool) {
    match outcome {
        DataOutcome::Exported(n) => (format!("{n} {} exportado{}", if *n == 1 { "evento" } else { "eventos" }, if *n == 1 { "" } else { "s" }), false),
        DataOutcome::Imported { imported, duplicates, invalid } => {
            let mut text = format!("{imported} {}, {duplicates} {}", if *imported == 1 { "importado" } else { "importados" }, if *duplicates == 1 { "repetido" } else { "repetidos" });
            if *invalid > 0 {
                text.push_str(&format!(", {invalid} {}", if *invalid == 1 { "inválido" } else { "inválidos" }));
            }
            (text, false)
        }
        DataOutcome::Cleared => ("Historial borrado".to_string(), false),
        DataOutcome::Failed(reason) => (reason.clone(), true),
    }
}

pub fn export_file_name(date: chrono::NaiveDate) -> String {
    format!("simple-player-historial-{}.json", date.format("%Y-%m-%d"))
}

fn data_section(ui: &mut egui::Ui, state: &mut AppState, view: &mut SettingsUi) {
    let handle = state.stats.handle().clone();
    if let Some(outcome) = handle.take_data_outcome() {
        view.status = Some(outcome_message(&outcome));
    }
    let disabled_reason = handle.disabled_reason();
    section(ui, "DATOS", |ui| {
        ui.add_enabled_ui(disabled_reason.is_none(), |ui| {
            row(ui, "Exportar historial", "Guarda todas tus reproducciones en un archivo JSON.", |ui| {
                if PillButton::new("Exportar…", PillKind::Secondary).icon(icons::DOWNLOAD_SIMPLE).show(ui).clicked() {
                    let name = export_file_name(chrono::Local::now().date_naive());
                    if let Some(path) = rfd::FileDialog::new().set_file_name(&name).add_filter("Historial", &["json"]).save_file() {
                        handle.export_to(path);
                    }
                }
            });
            ui.add_space(space::SM);
            row(ui, "Importar historial", "Une un archivo exportado con el que ya tienes; los repetidos se ignoran.", |ui| {
                if PillButton::new("Importar…", PillKind::Secondary).icon(icons::UPLOAD_SIMPLE).show(ui).clicked() {
                    if let Some(path) = rfd::FileDialog::new().add_filter("Historial", &["json"]).pick_file() {
                        handle.import_from(path);
                    }
                }
            });
            ui.add_space(space::SM);
            confirm_row(ui, view, Confirm::ClearHistory, "Borrar historial", "Elimina todas las reproducciones registradas.", "Borrar", || handle.clear_history());
        })
        .response
        .on_hover_text(disabled_reason.clone().unwrap_or_default());
        ui.add_space(space::SM);
        let can_reset = state.can_factory_reset();
        let mut do_reset = false;
        ui.add_enabled_ui(can_reset, |ui| {
            confirm_row(ui, view, Confirm::FactoryReset, "Restablecer de fábrica", "Borra ajustes, tamaño de ventana, estado de reproducción y caché. Conserva tu historial y tu música.", "Restablecer", || do_reset = true);
        });
        if do_reset {
            view.status = Some(match state.factory_reset() {
                Ok(_) => ("Ajustes restablecidos. El tamaño de la ventana vuelve al valor por defecto la próxima vez que abras la app.".to_string(), false),
                Err(e) => (e, true),
            });
        }
        if let Some((text, is_error)) = &view.status {
            ui.add_space(space::MD);
            let color = if *is_error { egui::Color32::from_rgb(255, 120, 120) } else { theme::TEXT_MUTED };
            ui.label(RichText::new(text).size(text::SM).color(color));
        }
    });
}

/// Fila con una acción destructiva: el primer clic pide confirmación en la misma fila.
fn confirm_row(ui: &mut egui::Ui, view: &mut SettingsUi, which: Confirm, label: &str, description: &str, action_label: &str, on_confirm: impl FnOnce()) {
    row(ui, label, description, |ui| {
        if view.confirm == Some(which) {
            if PillButton::new("Cancelar", PillKind::Ghost).show(ui).clicked() {
                view.confirm = None;
            }
            if PillButton::new(&format!("Sí, {}", action_label.to_lowercase()), PillKind::Primary).icon(icons::WARNING).show(ui).clicked() {
                view.confirm = None;
                on_confirm();
            }
            ui.label(RichText::new("¿Seguro?").color(theme::ACCENT_SUNSET));
        } else if PillButton::new(action_label, PillKind::Secondary).icon(icons::TRASH).show(ui).clicked() {
            view.confirm = Some(which);
        }
    });
}
```
y en `show`, después de `appearance_section`: `ui.add_space(GAP); data_section(ui, state, &mut view);`. En `library_section` quitar el guion bajo de `_view` si ya no hace falta (o dejarlo). Quitar el `#![allow(dead_code)]` de `settings.rs` si se añadió en la Tarea 6.

- [ ] **Step 4: Run tests, build and look at it**

Run: `cargo test 2>&1 | grep -E "^test result|^(error|warning)" -A5; cargo build --release 2>&1 | grep -E "^(error|warning)" -A6`
Expected: pruebas nuevas en verde (3 de `reset`, 2 de la pantalla), suite completa verde y sin advertencias. Capturar `--tab settings` (`--stats-demo` para tener historial) a 1050×750 y 1000×650 y leer: la sección Datos con los cuatro botones, "Restablecer de fábrica" deshabilitado (corrida de desarrollo) y el texto no desborda. **Comprobación manual** (no automatizable sin ventana real; anotar en el commit que la hizo el implementador o que queda para el usuario): exportar → borrar → importar con la app normal y un `XDG_DATA_HOME` temporal para no tocar la base real.

- [ ] **Step 5: Commit**

```bash
git add -A src
git commit -m "Configuración: exportar, importar, borrar historial y restablecer de fábrica

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 11: Documentación y verificación final

**Files:**
- Modify: `README.md`, `docs/superpowers/specs/2026-10-03-configuracion-y-tipografia-design.md`

**Interfaces:**
- Consumes: todo lo anterior.
- Produces: documentación al día y los criterios de aceptación verificados.

- [ ] **Step 1: Actualizar el README**

- Característica nueva en la lista: `- **Configuración:** varias carpetas de música, visualizador por defecto, recordar el tamaño de la ventana y herramientas del historial (exportar, importar, borrar) y restablecer de fábrica.`
- Tabla «Datos que guarda la app»: añadir `| Ajustes (carpetas, preferencias) | ~/.config/simple-player/settings.json (respeta XDG_CONFIG_HOME) |` y mencionar que la ubicación de la base de estadísticas se puede abrir desde Configuración → Acerca de.
- Estructura: añadir `settings.rs`, `reset.rs` y `ui/screens/settings.rs` en el árbol del proyecto; en `assets/` mencionar «GTA Art Deco (toda la interfaz) y Noto Sans (respaldo)».
- Opciones de desarrollo: añadir `| --music-folder <ruta> | Siembra una carpeta en los ajustes en memoria (repetible) |` y `--tab settings` en la fila de `--tab`.

- [ ] **Step 2: Actualizar el spec**

Cambiar `> **Estado:**` a `implementado en nueva-version; falta la aceptación manual del usuario (criterios 1, 3, 5 y 6 con la app normal).`

- [ ] **Step 3: Verificación final**

```bash
cargo test 2>&1 | grep -E "^test result|^(error|warning)" -A4
cargo build --release 2>&1 | grep -E "^(error|warning)" -A4
SECS=8 scripts/bench-visualizers.sh; echo "exit=$?"
```
Expected: suite en verde, build sin advertencias y `exit=0` (el cambio de fuente no debe empeorar los ms por frame de forma notable: comparar con la última tabla, Barras ≈ 0.7 ms). Capturar las pantallas principales a 1050×750 y 1000×650 y revisar los criterios de aceptación del spec uno por uno con la evidencia; los criterios 1, 3, 5 y 6 con la app real los verifica el usuario.

- [ ] **Step 4: Commit**

```bash
git add README.md docs
git commit -m "Documenta la configuración y la tipografía unificada

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```
