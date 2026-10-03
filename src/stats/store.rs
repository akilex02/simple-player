use super::model::{EventRow, PlayEvent, SongSnapshot, TopEntry, Totals};
use rusqlite::{params, Connection, Statement};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug)]
pub struct StoreError(pub String);

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError(e.to_string())
    }
}

pub type StoreResult<T> = Result<T, StoreError>;

/// Cuántos eventos entraron y cuántos ya estaban al importar.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub duplicates: usize,
}

pub struct Store {
    conn: Connection,
}

const SCHEMA_VERSION: i64 = 1;

const SCHEMA_SQL: &str = "
CREATE TABLE play_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at  INTEGER NOT NULL,
    ended_at    INTEGER NOT NULL,
    listened_ms INTEGER NOT NULL CHECK (listened_ms > 0),
    song_path   TEXT    NOT NULL,
    title       TEXT    NOT NULL,
    artist      TEXT    NOT NULL,
    album       TEXT    NOT NULL,
    duration_ms INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_play_events_ended_at ON play_events (ended_at);
";

const INSERT_SQL: &str = "INSERT INTO play_events
    (started_at, ended_at, listened_ms, song_path, title, artist, album, duration_ms)
    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)";

const RANGE_FILTER: &str = "ended_at <= ?2 AND (?1 IS NULL OR ended_at >= ?1)";

impl Store {
    pub fn open(path: &Path) -> StoreResult<Store> {
        let conn = Connection::open(path)?;
        // WAL: más robusto ante cierres bruscos. Devuelve una fila, por eso query_row.
        let _: String = conn.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
        Self::init(conn)
    }

    pub fn open_in_memory() -> StoreResult<Store> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> StoreResult<Store> {
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        match version {
            0 => conn.execute_batch(&format!("BEGIN; {SCHEMA_SQL} PRAGMA user_version = {SCHEMA_VERSION}; COMMIT;"))?,
            SCHEMA_VERSION => {}
            newer => {
                return Err(StoreError(format!(
                    "la base de estadísticas es de una versión más nueva ({newer}) que la que entiende esta app"
                )))
            }
        }
        Ok(Store { conn })
    }

    /// Un evento sin tiempo se descarta; si el fin quedó antes del inicio
    /// (el reloj retrocedió) se corrige el inicio.
    fn normalized(event: &PlayEvent) -> Option<(i64, i64)> {
        if event.listened_ms == 0 {
            return None;
        }
        let ended = event.ended_at;
        let started = if event.started_at > ended {
            ended.saturating_sub(event.listened_ms as i64)
        } else {
            event.started_at
        };
        Some((started, ended))
    }

    fn execute_insert(stmt: &mut Statement, event: &PlayEvent, started: i64, ended: i64) -> rusqlite::Result<()> {
        stmt.execute(params![
            started,
            ended,
            event.listened_ms as i64,
            event.song.path,
            event.song.title,
            event.song.artist,
            event.song.album,
            event.song.duration_ms as i64
        ])?;
        Ok(())
    }

    pub fn insert(&self, event: &PlayEvent) -> StoreResult<()> {
        if let Some((started, ended)) = Self::normalized(event) {
            let mut stmt = self.conn.prepare_cached(INSERT_SQL)?;
            Self::execute_insert(&mut stmt, event, started, ended)?;
        }
        Ok(())
    }

    pub fn insert_batch(&mut self, events: &[PlayEvent]) -> StoreResult<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(INSERT_SQL)?;
            for event in events {
                if let Some((started, ended)) = Self::normalized(event) {
                    Self::execute_insert(&mut stmt, event, started, ended)?;
                }
            }
        }
        tx.commit()?;
        Ok(())
    }

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

    #[cfg(test)]
    pub fn count(&self) -> StoreResult<u64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM play_events", [], |r| r.get::<_, i64>(0))? as u64)
    }

    pub fn totals(&self, start: Option<i64>, end: i64) -> StoreResult<Totals> {
        let sql = format!(
            "SELECT COALESCE(SUM(listened_ms), 0), COUNT(*), COUNT(DISTINCT song_path)
             FROM play_events WHERE {RANGE_FILTER}"
        );
        Ok(self.conn.query_row(&sql, params![start, end], |r| {
            Ok(Totals {
                listened_ms: r.get::<_, i64>(0)? as u64,
                plays: r.get::<_, i64>(1)? as u32,
                unique_songs: r.get::<_, i64>(2)? as u32,
            })
        })?)
    }

    pub fn rows_in_range(&self, start: Option<i64>, end: i64) -> StoreResult<Vec<EventRow>> {
        let sql = format!(
            "SELECT started_at, ended_at, listened_ms FROM play_events
             WHERE {RANGE_FILTER}"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![start, end], |r| {
            Ok(EventRow { started_at: r.get(0)?, ended_at: r.get(1)?, listened_ms: r.get::<_, i64>(2)? as u64 })
        })?;
        let mut rows: Vec<EventRow> = rows.collect::<Result<_, _>>()?;
        // Ordenar en Rust: llegan casi ordenadas por `ended_at` y evita un sort en SQLite.
        rows.sort_by_key(|r| r.started_at);
        Ok(rows)
    }

    pub fn top_songs(&self, start: Option<i64>, end: i64, limit: u32) -> StoreResult<Vec<TopEntry>> {
        self.run_top(
            "SELECT title, artist, song_path, SUM(listened_ms) AS t, COUNT(*) AS n, MAX(ended_at)
             FROM play_events WHERE {FILTER}
             GROUP BY song_path ORDER BY t DESC, n DESC, title ASC LIMIT ?3",
            start,
            end,
            limit,
        )
    }

    pub fn top_artists(&self, start: Option<i64>, end: i64, limit: u32) -> StoreResult<Vec<TopEntry>> {
        self.run_top(
            "SELECT artist, '', song_path, SUM(listened_ms) AS t, COUNT(*) AS n, MAX(ended_at)
             FROM play_events WHERE {FILTER}
             GROUP BY artist ORDER BY t DESC, n DESC, artist ASC LIMIT ?3",
            start,
            end,
            limit,
        )
    }

    pub fn top_albums(&self, start: Option<i64>, end: i64, limit: u32) -> StoreResult<Vec<TopEntry>> {
        self.run_top(
            "SELECT album, artist, song_path, SUM(listened_ms) AS t, COUNT(*) AS n, MAX(ended_at)
             FROM play_events WHERE {FILTER}
             GROUP BY album, artist ORDER BY t DESC, n DESC, album ASC LIMIT ?3",
            start,
            end,
            limit,
        )
    }

    fn run_top(&self, template: &str, start: Option<i64>, end: i64, limit: u32) -> StoreResult<Vec<TopEntry>> {
        let sql = template.replace("{FILTER}", RANGE_FILTER);
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![start, end, limit], |r| {
            Ok(TopEntry {
                name: r.get(0)?,
                detail: r.get(1)?,
                reference_path: r.get(2)?,
                listened_ms: r.get::<_, i64>(3)? as u64,
                plays: r.get::<_, i64>(4)? as u32,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn first_event_end(&self) -> StoreResult<Option<i64>> {
        Ok(self.conn.query_row("SELECT MIN(ended_at) FROM play_events", [], |r| r.get(0))?)
    }

    pub fn all_ended_at(&self) -> StoreResult<Vec<i64>> {
        let mut stmt = self.conn.prepare("SELECT ended_at FROM play_events ORDER BY ended_at ASC")?;
        let rows = stmt.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::model::SongSnapshot;

    fn ev(path: &str, title: &str, artist: &str, album: &str, started: i64, ended: i64, listened: u64) -> PlayEvent {
        PlayEvent {
            started_at: started,
            ended_at: ended,
            listened_ms: listened,
            song: SongSnapshot {
                path: path.into(),
                title: title.into(),
                artist: artist.into(),
                album: album.into(),
                duration_ms: 200_000,
            },
        }
    }

    fn store_with(events: &[PlayEvent]) -> Store {
        let mut s = Store::open_in_memory().expect("abre en memoria");
        s.insert_batch(events).unwrap();
        s
    }

    #[test]
    fn inserta_y_cuenta() {
        let s = store_with(&[ev("a", "A", "X", "Z", 1_000, 2_000, 1_000)]);
        assert_eq!(s.count().unwrap(), 1);
    }

    #[test]
    fn el_rango_incluye_ambos_bordes() {
        let s = store_with(&[
            ev("a", "A", "X", "Z", 0, 1_000, 1_000),
            ev("b", "B", "X", "Z", 0, 2_000, 1_000),
            ev("c", "C", "X", "Z", 0, 3_000, 1_000),
            ev("d", "D", "X", "Z", 0, 4_000, 1_000),
        ]);
        let rows = s.rows_in_range(Some(2_000), 3_000).unwrap();
        let ends: Vec<i64> = rows.iter().map(|r| r.ended_at).collect();
        assert_eq!(ends, vec![2_000, 3_000]);
        assert_eq!(s.rows_in_range(None, 2_000).unwrap().len(), 2);
    }

    #[test]
    fn las_filas_salen_ordenadas_por_inicio() {
        let s = store_with(&[
            ev("a", "A", "X", "Z", 5_000, 6_000, 1_000),
            ev("b", "B", "X", "Z", 1_000, 2_000, 1_000),
        ]);
        let starts: Vec<i64> = s.rows_in_range(None, 10_000).unwrap().iter().map(|r| r.started_at).collect();
        assert_eq!(starts, vec![1_000, 5_000]);
    }

    #[test]
    fn los_totales_cuentan_tiempo_reproducciones_y_canciones_unicas() {
        let s = store_with(&[
            ev("a", "A", "X", "Z", 0, 1_000, 30_000),
            ev("a", "A", "X", "Z", 0, 2_000, 20_000),
            ev("b", "B", "X", "Z", 0, 3_000, 10_000),
        ]);
        let t = s.totals(None, 10_000).unwrap();
        assert_eq!((t.listened_ms, t.plays, t.unique_songs), (60_000, 3, 2));
    }

    #[test]
    fn un_evento_con_fin_anterior_al_inicio_se_corrige_y_uno_sin_tiempo_se_descarta() {
        let s = store_with(&[
            ev("a", "A", "X", "Z", 9_000, 5_000, 4_000), // reloj que retrocede
            ev("b", "B", "X", "Z", 1_000, 2_000, 0),     // sin tiempo
        ]);
        assert_eq!(s.count().unwrap(), 1);
        let row = s.rows_in_range(None, 10_000).unwrap()[0];
        assert!(row.started_at <= row.ended_at, "{row:?}");
    }

    #[test]
    fn el_top_de_canciones_ordena_por_tiempo_y_desempata_por_reproducciones() {
        let s = store_with(&[
            ev("x", "X", "Ar", "Al", 0, 1_000, 200_000),
            ev("x", "X", "Ar", "Al", 0, 2_000, 200_000),
            ev("x", "X", "Ar", "Al", 0, 3_000, 200_000), // X: 600 s en 3 reproducciones
            ev("y", "Y", "Ar", "Al", 0, 4_000, 600_000), // Y: 600 s en 1 reproducción
            ev("z", "Z", "Ar", "Al", 0, 5_000, 100_000),
        ]);
        let top = s.top_songs(None, 10_000, 2).unwrap();
        let names: Vec<&str> = top.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["X", "Y"]);
        assert_eq!((top[0].listened_ms, top[0].plays), (600_000, 3));
        assert_eq!(top[0].detail, "Ar");
        assert_eq!(top[0].reference_path, "x");
    }

    #[test]
    fn los_artistas_vacios_se_agrupan_bajo_la_cadena_vacia() {
        let s = store_with(&[
            ev("a", "A", "", "", 0, 1_000, 10_000),
            ev("b", "B", "", "", 0, 2_000, 10_000),
            ev("c", "C", "Toto", "IV", 0, 3_000, 5_000),
        ]);
        let artists = s.top_artists(None, 10_000, 5).unwrap();
        assert_eq!(artists[0].name, "");
        assert_eq!((artists[0].listened_ms, artists[0].plays), (20_000, 2));
        assert_eq!(artists[1].name, "Toto");
    }

    #[test]
    fn un_album_se_identifica_por_album_y_artista() {
        let s = store_with(&[
            ev("a", "A", "Uno", "OG", 0, 1_000, 10_000),
            ev("b", "B", "Dos", "OG", 0, 2_000, 10_000),
        ]);
        let albums = s.top_albums(None, 10_000, 5).unwrap();
        assert_eq!(albums.len(), 2, "el mismo nombre de álbum en artistas distintos son dos álbumes");
        assert!(albums.iter().all(|a| a.name == "OG"));
    }

    #[test]
    fn primer_evento_y_fechas_de_fin_en_orden() {
        let s = store_with(&[
            ev("a", "A", "X", "Z", 0, 9_000, 1_000),
            ev("b", "B", "X", "Z", 0, 3_000, 1_000),
        ]);
        assert_eq!(s.first_event_end().unwrap(), Some(3_000));
        assert_eq!(s.all_ended_at().unwrap(), vec![3_000, 9_000]);
        assert_eq!(store_with(&[]).first_event_end().unwrap(), None);
    }

    #[test]
    fn una_base_de_version_mas_nueva_se_rechaza() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA user_version = 99;").unwrap();
        let err = Store::init(conn).err().expect("debe fallar");
        assert!(err.0.contains("99"), "{}", err.0);
    }

    #[test]
    fn abrir_dos_veces_el_mismo_archivo_conserva_los_datos() {
        let path = std::env::temp_dir().join(format!("sp_stats_{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let s = Store::open(&path).unwrap();
            s.insert(&ev("a", "A", "X", "Z", 0, 1_000, 6_000)).unwrap();
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.count().unwrap(), 1);
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", path.display()));
        }
    }

    fn ev_simple(started: i64, ended: i64, listened: u64, path: &str) -> PlayEvent {
        ev(path, "T", "A", "B", started, ended, listened)
    }

    #[test]
    fn all_events_devuelve_lo_guardado_en_orden() {
        let mut store = Store::open_in_memory().unwrap();
        store.insert_batch(&[ev_simple(100, 200, 100, "/b"), ev_simple(10, 20, 10, "/a")]).unwrap();
        let all = store.all_events().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].song.path, "/a", "ordenados por fin");
    }

    #[test]
    fn importar_en_una_base_vacia_inserta_todo() {
        let mut store = Store::open_in_memory().unwrap();
        let report = store.import_events(&[ev_simple(10, 20, 10, "/a"), ev_simple(30, 40, 10, "/b")]).unwrap();
        assert_eq!((report.imported, report.duplicates), (2, 0));
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn importar_dos_veces_no_duplica() {
        let mut store = Store::open_in_memory().unwrap();
        let events = [ev_simple(10, 20, 10, "/a"), ev_simple(30, 40, 10, "/b")];
        store.import_events(&events).unwrap();
        let again = store.import_events(&events).unwrap();
        assert_eq!((again.imported, again.duplicates), (0, 2));
        assert_eq!(store.count().unwrap(), 2);
    }

    #[test]
    fn los_repetidos_dentro_del_mismo_archivo_se_cuentan_como_duplicados() {
        let mut store = Store::open_in_memory().unwrap();
        let report = store.import_events(&[ev_simple(10, 20, 10, "/a"), ev_simple(10, 20, 10, "/a")]).unwrap();
        assert_eq!((report.imported, report.duplicates), (1, 1));
    }

    #[test]
    fn un_fallo_a_mitad_del_lote_no_deja_eventos_parciales() {
        let mut store = Store::open_in_memory().unwrap();
        // `u64::MAX` como i64 es -1 y viola CHECK (listened_ms > 0) en pleno lote.
        let result = store.import_events(&[ev_simple(10, 20, 10, "/a"), ev_simple(30, 40, u64::MAX, "/mala"), ev_simple(50, 60, 10, "/c")]);
        assert!(result.is_err());
        assert_eq!(store.count().unwrap(), 0, "la transacción debe revertirse entera");
    }

    #[test]
    fn clear_deja_la_tabla_vacia() {
        let mut store = Store::open_in_memory().unwrap();
        store.insert_batch(&[ev_simple(10, 20, 10, "/a")]).unwrap();
        store.clear().unwrap();
        assert_eq!(store.count().unwrap(), 0);
        store.insert(&ev_simple(10, 20, 10, "/a")).unwrap();
        assert_eq!(store.count().unwrap(), 1, "sigue funcionando después de limpiar");
    }
}
