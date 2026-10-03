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
            ImportError::UnsupportedVersion(v) => {
                write!(f, "El archivo es de una versión más nueva ({v}) que la que entiende esta app.")
            }
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

/// Una reproducción no puede durar más de un día; un tope realista evita que unas pocas filas enormes
/// desborden `SUM(listened_ms)` en SQLite y dejen las estadísticas inutilizables.
const MAX_LISTENED_MS: u64 = 24 * 60 * 60 * 1000;

fn is_valid(e: &ExportEvent) -> bool {
    e.listened_ms > 0
        && e.listened_ms <= MAX_LISTENED_MS
        && e.duration_ms <= i64::MAX as u64
        && e.started_at >= 0
        && e.ended_at >= 0
        && e.ended_at >= e.started_at
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

    #[test]
    fn un_tiempo_escuchado_absurdo_se_considera_invalido_para_no_desbordar_las_sumas() {
        let day = 24 * 60 * 60 * 1000;
        let json = to_json(
            &[
                event(0, day, day as u64, "/limite.mp3"),
                event(0, day, day as u64 + 1, "/pasa-del-limite.mp3"),
                event(0, i64::MAX, i64::MAX as u64, "/enorme-1.mp3"),
                event(0, i64::MAX, i64::MAX as u64, "/enorme-2.mp3"),
            ],
            1,
        );
        let parsed = parse(&json).unwrap();
        assert_eq!(parsed.events.len(), 1, "solo el que está en el límite entra");
        assert_eq!(parsed.invalid, 3);
    }
}
