//! Pantalla de Configuración: carpetas de música, preferencias, datos y acerca de.
use crate::paths::get_covers_dir;
use crate::state::AppState;
use crate::stats::service::DataOutcome;
use crate::ui::visualizers::VisualizerMode;
use crate::ui::widgets::chip::chip;
use crate::ui::widgets::toggle::toggle;
use crate::theme::{self, icons, radius, space, text};
use crate::ui::widgets::glass::{glass_panel, GlassKind};
use crate::ui::widgets::icon_button::IconButton;
use crate::ui::widgets::pill_button::{PillButton, PillKind};
use eframe::egui::{self, RichText};
use std::path::{Path, PathBuf};

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
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
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
    view.begin_frame(ui.ctx().cumulative_pass_nr());

    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.label(RichText::new("CONFIGURACIÓN").font(theme::deco(text::XL + 4.0)).color(theme::accent(ui.ctx())));
        ui.add_space(space::LG);
        library_section(ui, state);
        ui.add_space(GAP);
        appearance_section(ui, state);
        ui.add_space(GAP);
        data_section(ui, state, &mut view);
        ui.add_space(GAP);
        about_section(ui);
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
    let total = ui.available_width();
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            // El texto se ajusta en varias líneas para no pisar el control de la derecha.
            ui.set_max_width((total - 200.0).max(160.0));
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
        ui.label(
            RichText::new("Con el que abre la pantalla completa. Cambiar de modo allí no modifica este valor.")
                .size(text::SM)
                .color(theme::TEXT_MUTED),
        );
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

pub fn outcome_message(outcome: &DataOutcome) -> (String, bool) {
    match outcome {
        DataOutcome::Exported(n) => (format!("{n} {} exportado{}", if *n == 1 { "evento" } else { "eventos" }, if *n == 1 { "" } else { "s" }), false),
        DataOutcome::Imported { imported, duplicates, invalid } => {
            let mut text = format!(
                "{imported} {}, {duplicates} {}",
                if *imported == 1 { "importado" } else { "importados" },
                if *duplicates == 1 { "repetido" } else { "repetidos" }
            );
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
            confirm_row(ui, view, Confirm::ClearHistory, "Borrar historial", "Elimina todas las reproducciones registradas.", "Borrar", || {
                handle.clear_history()
            });
        })
        .response
        .on_hover_text(disabled_reason.clone().unwrap_or_default());
        ui.add_space(space::SM);

        let can_reset = state.can_factory_reset();
        let mut do_reset = false;
        ui.add_enabled_ui(can_reset, |ui| {
            confirm_row(
                ui,
                view,
                Confirm::FactoryReset,
                "Restablecer de fábrica",
                "Borra ajustes, tamaño de ventana, estado de reproducción y caché. Conserva tu historial y tu música.",
                "Restablecer",
                || do_reset = true,
            );
        });
        if do_reset {
            view.status = Some(match state.factory_reset() {
                Ok(_) => (
                    "Ajustes restablecidos. El tamaño de la ventana vuelve al valor por defecto la próxima vez que abras la app.".to_string(),
                    false,
                ),
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
fn confirm_row(
    ui: &mut egui::Ui,
    view: &mut SettingsUi,
    which: Confirm,
    label: &str,
    description: &str,
    action_label: &str,
    on_confirm: impl FnOnce(),
) {
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

fn library_section(ui: &mut egui::Ui, state: &mut AppState) {
    section(ui, "BIBLIOTECA", |ui| {
        ui.label(RichText::new(library_summary(state.songs.len(), state.settings.music_folders.len())).color(theme::TEXT_MUTED));
        ui.add_space(space::SM);

        let mut remove: Option<String> = None;
        for folder in &state.settings.music_folders {
            let exists = Path::new(folder).is_dir();
            let total = ui.available_width();
            ui.horizontal(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2((total - 190.0).max(80.0), 28.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(RichText::new(icons::FOLDER_OPEN).size(text::LG).color(theme::TEXT_MUTED));
                        ui.add(egui::Label::new(RichText::new(folder)).truncate()).on_hover_text(folder);
                    },
                );
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
}
