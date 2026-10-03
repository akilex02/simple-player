# Estadísticas de escucha — Plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Que Simple Player registre el tiempo real escuchado en SQLite y muestre una pantalla "Estadísticas" con totales, tops, hábitos y línea de tiempo por rango.

**Architecture:** Módulo nuevo `native/src/stats/` con lógica pura (modelo, tracker de sesión, agregación, formato) y un hilo de fondo (`StatsService`) dueño de la conexión SQLite. `AppState` abre una sesión en `play_index` y la observa cada frame; la UI nunca espera al disco. La pantalla pide resúmenes al servicio y se refresca cuando llega un evento nuevo.

**Tech Stack:** Rust, `rusqlite` 0.40 (feature `bundled`), `chrono` 0.4, egui/eframe 0.29 (ya en el proyecto).

**Spec:** `docs/superpowers/specs/2026-10-03-estadisticas-de-escucha-design.md`

## Global Constraints

- SQLite con `rusqlite` y la feature `bundled`; `chrono` con `default-features = false, features = ["clock", "std"]`.
- Archivo: `$XDG_DATA_HOME/simple-player/stats.db`, por defecto `~/.local/share/simple-player/stats.db`. Nunca en `~/.cache`.
- Umbral de reproducción: **5 s** de tiempo real escuchado (`MIN_LISTEN_MS = 5_000`). Hueco de sesión: **30 min** (inclusive).
- Rangos por calendario en hora local: Hoy, Semana (desde el **lunes**), Mes, Año, Todo; el evento pertenece al día en que **terminó** (`ended_at`); bordes inclusivos.
- Tops de **5** por tiempo, desempate por reproducciones y luego por nombre; álbum = (álbum, artista).
- Todo el texto de interfaz en español; artista/álbum vacío se muestra como "Artista desconocido" / "Álbum desconocido".
- La pantalla no pide repintado continuo (reposo ≤ ~0.5 % de CPU). Resumen de 100 000 eventos en menos de ~200 ms (release).
- Las corridas de desarrollo (`--shot`, `--bench`, `--gallery`, `--stats-demo`) **nunca** escriben en la base real.
- Si la base no abre, las estadísticas se desactivan con aviso; la reproducción no se ve afectada.
- Sin push ni merge: todo se commitea en la rama `nueva-version`.
- Comandos de prueba: desde `native/`, `cargo test <filtro>`; suite completa `cargo test`.

**Aclaración sobre el spec (§6.3):** el servicio expone **dos** contadores. `events_version()` sube solo con eventos guardados y es lo que la pantalla observa para volver a pedir el resumen; los resúmenes listos se publican en `latest_summary()` y despiertan la UI, pero **no** cambian `events_version()` (si lo cambiaran, cada resumen provocaría otra petición en bucle).

## Review Focus

Entradas o condiciones que el spec implica pero que ninguna tarea cubriría por sí sola; cada una tiene su prueba en la tarea indicada:

1. **Reloj del sistema raro** (`ended_at < started_at`) o evento sin tiempo: el almacén corrige o descarta, no falla. → Tarea 3.
2. **Canciones sin etiquetas** (artista/álbum vacíos): se agrupan bajo la cadena vacía y se muestran como "desconocido". → Tareas 1 y 3.
3. **Rango "Todo" sin ningún evento** (biblioteca nueva): resumen vacío, sin pánico ni división por cero. → Tarea 5.
4. **Ruta de base imposible de abrir** (directorio inexistente/sin permisos): servicio deshabilitado, `record` no entra en pánico. → Tarea 6.
5. **App cerrada mucho después de pausar:** `ended_at` es el último instante en que sonó, no el momento del cierre. → Tarea 2.

---

## Mapa de archivos

| Archivo | Acción | Responsabilidad |
|---|---|---|
| `native/Cargo.toml` | Modificar | Agregar `chrono` (Tarea 1) y `rusqlite` (Tarea 3) |
| `native/src/main.rs` | Modificar | `mod stats;`, servicio, `on_exit`, `observe`, flags de desarrollo |
| `native/src/stats/mod.rs` | Crear | Declara los submódulos |
| `native/src/stats/model.rs` | Crear | Tipos de datos compartidos |
| `native/src/stats/format.rs` | Crear | Duraciones y nombres en español (puro) |
| `native/src/stats/session.rs` | Crear | `SessionTracker` (puro, tiempo inyectado) |
| `native/src/stats/store.rs` | Crear | SQLite: esquema, migración, inserción, consultas |
| `native/src/stats/aggregate.rs` | Crear | Rangos, hábitos y línea de tiempo (puro) |
| `native/src/stats/summary.rs` | Crear | Une `Store` + `aggregate` en un `StatsSummary` |
| `native/src/stats/location.rs` | Crear | Ruta de la base según entorno y flags |
| `native/src/stats/service.rs` | Crear | Hilo de fondo, `StatsHandle` |
| `native/src/stats/recorder.rs` | Crear | `StatsRecorder`: tracker + handle |
| `native/src/stats/demo.rs` | Crear | Eventos sintéticos para `--stats-demo` |
| `native/src/state.rs` | Modificar | Campos y enganche en `play_index` |
| `native/src/theme/icons.rs` | Modificar | Exportar `CHART_BAR` |
| `native/src/ui/widgets/bar_chart.rs` | Crear | Geometría y dibujo del gráfico de barras |
| `native/src/ui/widgets/mod.rs` | Modificar | Registrar `bar_chart` |
| `native/src/ui/screens/stats.rs` | Crear | Pantalla Estadísticas |
| `native/src/ui/screens/mod.rs` | Modificar | Despachar `ActiveTab::Stats` |
| `native/src/ui/shell/sidebar.rs` | Modificar | Entrada "Estadísticas" |
| `native/src/ui/shell/topbar.rs` | Modificar | Ocultar buscador en Estadísticas |

Mientras no esté integrado (hasta la Tarea 8), `mod stats;` en `main.rs` lleva `#[allow(dead_code)]` para no ensuciar la compilación con avisos; la Tarea 8 lo quita.

---

### Task 1: Modelo y formato en español

**Files:**
- Create: `native/src/stats/mod.rs`, `native/src/stats/model.rs`, `native/src/stats/format.rs`
- Modify: `native/Cargo.toml`, `native/src/main.rs`

**Interfaces:**
- Consumes: `crate::library::Song` (campos `path, title, artist, album: String`, `duration_secs: u64`, `cover_art: Option<String>`).
- Produces (usados por todas las tareas siguientes):
  - `model::{SongSnapshot, PlayEvent, EventRow, StatsRange, TopEntry, TimelineBucket, Habits, Totals, StatsSummary}` con los campos exactos del bloque de código del Paso 3.
  - `StatsRange::ALL_RANGES: [StatsRange; 5]`, `StatsRange::label(self) -> &'static str`.
  - `format::format_duration(ms: u64) -> String`, `format::weekday_short(chrono::Weekday) -> &'static str`, `format::weekday_full(chrono::Weekday) -> &'static str`, `format::month_short(month: u32) -> &'static str` (1..=12), `format::unknown_if_empty(name: &str, fallback: &str) -> String`.

- [ ] **Step 1: Agregar `chrono` y registrar el módulo**

En `native/Cargo.toml`, bajo `rand = "0.8"`, agregar:

```toml

# Estadísticas de escucha
chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }
```

En `native/src/main.rs`, junto a las demás declaraciones `mod`, agregar (orden alfabético, después de `mod state;`):

```rust
#[allow(dead_code)]
mod stats;
```

Crear `native/src/stats/mod.rs`:

```rust
pub mod format;
pub mod model;
```

- [ ] **Step 2: Escribir las pruebas que fallan**

Crear `native/src/stats/format.rs` con funciones vacías y las pruebas:

```rust
use chrono::Weekday;

/// "41 h 12 min", "1 h", "45 min", "3 min 20 s", "45 s", "0 min".
pub fn format_duration(_ms: u64) -> String {
    String::new()
}

pub fn weekday_short(_day: Weekday) -> &'static str {
    ""
}

pub fn weekday_full(_day: Weekday) -> &'static str {
    ""
}

/// `month` de 1 a 12.
pub fn month_short(_month: u32) -> &'static str {
    ""
}

/// El nombre tal cual, o `fallback` si está vacío o solo tiene espacios.
pub fn unknown_if_empty(_name: &str, _fallback: &str) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duraciones_largas_y_cortas() {
        assert_eq!(format_duration(0), "0 min");
        assert_eq!(format_duration(45_000), "45 s");
        assert_eq!(format_duration(180_000), "3 min");
        assert_eq!(format_duration(200_000), "3 min 20 s");
        assert_eq!(format_duration(600_000), "10 min");
        assert_eq!(format_duration(2_700_000), "45 min");
        assert_eq!(format_duration(3_600_000), "1 h");
        assert_eq!(format_duration(5_400_000), "1 h 30 min");
        assert_eq!(format_duration(148_320_000), "41 h 12 min");
    }

    #[test]
    fn por_encima_de_10_min_se_ignoran_los_segundos() {
        assert_eq!(format_duration(605_000), "10 min");
        assert_eq!(format_duration(3_659_000), "1 h");
    }

    #[test]
    fn nombres_de_dias_y_meses_en_espanol() {
        assert_eq!(weekday_short(Weekday::Mon), "Lun");
        assert_eq!(weekday_short(Weekday::Wed), "Mié");
        assert_eq!(weekday_short(Weekday::Sat), "Sáb");
        assert_eq!(weekday_full(Weekday::Sat), "sábado");
        assert_eq!(weekday_full(Weekday::Wed), "miércoles");
        assert_eq!(month_short(1), "ene");
        assert_eq!(month_short(9), "sep");
        assert_eq!(month_short(12), "dic");
    }

    #[test]
    fn un_nombre_vacio_se_muestra_como_desconocido() {
        assert_eq!(unknown_if_empty("", "Artista desconocido"), "Artista desconocido");
        assert_eq!(unknown_if_empty("   ", "Artista desconocido"), "Artista desconocido");
        assert_eq!(unknown_if_empty("Toto", "Artista desconocido"), "Toto");
    }
}
```

Crear `native/src/stats/model.rs` con las pruebas del modelo (los tipos se completan en el Paso 4, así que aquí solo la prueba y un esqueleto que no compila sin ellos):

```rust
use crate::library::Song;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_snapshot_convierte_segundos_a_milisegundos() {
        let song = Song {
            path: "/m/a.mp3".into(),
            title: "A".into(),
            artist: "B".into(),
            album: "C".into(),
            duration_secs: 200,
            cover_art: None,
        };
        let snap = SongSnapshot::from(&song);
        assert_eq!(snap.duration_ms, 200_000);
        assert_eq!((snap.path.as_str(), snap.title.as_str()), ("/m/a.mp3", "A"));
    }

    #[test]
    fn los_rangos_tienen_etiqueta_y_orden_fijos() {
        let labels: Vec<&str> = StatsRange::ALL_RANGES.iter().map(|r| r.label()).collect();
        assert_eq!(labels, ["Hoy", "Semana", "Mes", "Año", "Todo"]);
    }
}
```

- [ ] **Step 3: Ejecutar para verificar que falla**

Run: `cd native && cargo test stats::`
Expected: error de compilación `cannot find type SongSnapshot` / `StatsRange` en `model.rs` (RED por tipos inexistentes) y, tras el Paso 4, las pruebas de `format` fallarán por aserción.

- [ ] **Step 4: Implementar el modelo**

Reemplazar `native/src/stats/model.rs` (conservando el bloque `tests` del Paso 2 al final) por:

```rust
use crate::library::Song;

/// Datos de la canción congelados al empezar a sonar: el historial sobrevive
/// aunque el archivo se mueva o se borre.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SongSnapshot {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
}

impl From<&Song> for SongSnapshot {
    fn from(song: &Song) -> Self {
        Self {
            path: song.path.clone(),
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            duration_ms: song.duration_secs * 1000,
        }
    }
}

/// Una reproducción terminada. Tiempos en milisegundos desde epoch UTC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayEvent {
    pub started_at: i64,
    pub ended_at: i64,
    pub listened_ms: u64,
    pub song: SongSnapshot,
}

/// Fila compacta de un evento, suficiente para hábitos y línea de tiempo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventRow {
    pub started_at: i64,
    pub ended_at: i64,
    pub listened_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatsRange {
    Today,
    Week,
    Month,
    Year,
    All,
}

impl StatsRange {
    pub const ALL_RANGES: [StatsRange; 5] =
        [StatsRange::Today, StatsRange::Week, StatsRange::Month, StatsRange::Year, StatsRange::All];

    pub fn label(self) -> &'static str {
        match self {
            StatsRange::Today => "Hoy",
            StatsRange::Week => "Semana",
            StatsRange::Month => "Mes",
            StatsRange::Year => "Año",
            StatsRange::All => "Todo",
        }
    }
}

/// Una fila de los rankings. `name` queda vacío si la etiqueta falta; la UI lo muestra como "desconocido".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopEntry {
    pub name: String,
    /// Artista (para canciones y álbumes); vacío en el ranking de artistas.
    pub detail: String,
    /// Ruta de una canción de referencia, para resolver la carátula en la UI.
    pub reference_path: String,
    pub listened_ms: u64,
    pub plays: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineBucket {
    pub label: String,
    pub listened_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Habits {
    pub active_days: u32,
    pub current_streak_days: u32,
    pub longest_streak_days: u32,
    pub sessions: u32,
    pub avg_session_ms: u64,
    pub longest_session_ms: u64,
    pub sessions_per_day: f64,
    /// (nombre completo del día, tiempo total); `None` si el rango abarca un solo día.
    pub peak_weekday: Option<(String, u64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Totals {
    pub listened_ms: u64,
    pub plays: u32,
    pub unique_songs: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StatsSummary {
    pub range: StatsRange,
    pub totals: Totals,
    pub avg_daily_ms: u64,
    pub days_in_span: u32,
    pub top_songs: Vec<TopEntry>,
    pub top_artists: Vec<TopEntry>,
    pub top_albums: Vec<TopEntry>,
    pub habits: Habits,
    pub timeline: Vec<TimelineBucket>,
    pub peak_bucket: Option<usize>,
}
```

Implementar `native/src/stats/format.rs` (reemplazar las cuatro funciones vacías, dejando el bloque `tests`):

```rust
pub fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    let minutes = secs / 60;
    let (hours, rest_minutes) = (minutes / 60, minutes % 60);
    if hours > 0 {
        return if rest_minutes == 0 { format!("{hours} h") } else { format!("{hours} h {rest_minutes} min") };
    }
    if secs == 0 {
        return "0 min".to_string();
    }
    if secs < 60 {
        return format!("{secs} s");
    }
    // Bajo 10 min se muestran los segundos; desde ahí solo minutos.
    if secs < 600 && secs % 60 != 0 {
        return format!("{minutes} min {} s", secs % 60);
    }
    format!("{minutes} min")
}

pub fn weekday_short(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Lun",
        Weekday::Tue => "Mar",
        Weekday::Wed => "Mié",
        Weekday::Thu => "Jue",
        Weekday::Fri => "Vie",
        Weekday::Sat => "Sáb",
        Weekday::Sun => "Dom",
    }
}

pub fn weekday_full(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "lunes",
        Weekday::Tue => "martes",
        Weekday::Wed => "miércoles",
        Weekday::Thu => "jueves",
        Weekday::Fri => "viernes",
        Weekday::Sat => "sábado",
        Weekday::Sun => "domingo",
    }
}

pub fn month_short(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
    MONTHS[(month.clamp(1, 12) - 1) as usize]
}

pub fn unknown_if_empty(name: &str, fallback: &str) -> String {
    if name.trim().is_empty() { fallback.to_string() } else { name.to_string() }
}
```

- [ ] **Step 5: Ejecutar para verificar que pasa**

Run: `cd native && cargo test stats::`
Expected: PASS (6 pruebas).

- [ ] **Step 6: Commit**

```bash
git add native/Cargo.toml native/Cargo.lock native/src/main.rs native/src/stats
git commit -m "Estadísticas: modelo de datos y formato en español"
```

---

### Task 2: `SessionTracker` (captura del tiempo real escuchado)

**Files:**
- Create: `native/src/stats/session.rs`
- Modify: `native/src/stats/mod.rs`

**Interfaces:**
- Consumes: `model::{SongSnapshot, PlayEvent}`.
- Produces:
  - `session::MIN_LISTEN_MS: u64` (= 5_000).
  - `session::SessionTracker::new() -> Self`.
  - `SessionTracker::start(&mut self, song: SongSnapshot, now: std::time::Instant, epoch_ms: i64, playing: bool) -> Option<PlayEvent>` — cierra la sesión anterior (devuelve su evento si cuenta) y abre otra.
  - `SessionTracker::observe(&mut self, playing: bool, now: Instant, epoch_ms: i64)`.
  - `SessionTracker::finish(&mut self, now: Instant, epoch_ms: i64) -> Option<PlayEvent>`.

- [ ] **Step 1: Registrar el módulo y escribir las pruebas que fallan**

En `native/src/stats/mod.rs` agregar `pub mod session;` (después de `pub mod model;`).

Crear `native/src/stats/session.rs`:

```rust
use super::model::{PlayEvent, SongSnapshot};
use std::time::Instant;

/// Mínimo de tiempo real escuchado para que una sesión cuente.
pub const MIN_LISTEN_MS: u64 = 5_000;

struct Active {
    song: SongSnapshot,
    started_at: i64,
    listened_ms: u64,
    last_instant: Instant,
    playing: bool,
    last_playing_epoch: i64,
}

/// Mide el tiempo real escuchado de la canción en curso. No tiene reloj
/// propio: el tiempo llega por parámetro, así se prueba sin esperar.
pub struct SessionTracker {
    active: Option<Active>,
}

impl SessionTracker {
    pub fn new() -> Self {
        Self { active: None }
    }

    pub fn start(&mut self, _song: SongSnapshot, _now: Instant, _epoch_ms: i64, _playing: bool) -> Option<PlayEvent> {
        None
    }

    pub fn observe(&mut self, _playing: bool, _now: Instant, _epoch_ms: i64) {}

    pub fn finish(&mut self, _now: Instant, _epoch_ms: i64) -> Option<PlayEvent> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const E0: i64 = 1_700_000_000_000;

    fn song(path: &str) -> SongSnapshot {
        SongSnapshot {
            path: path.into(),
            title: format!("T {path}"),
            artist: "A".into(),
            album: "B".into(),
            duration_ms: 200_000,
        }
    }

    fn at(t0: Instant, ms: u64) -> Instant {
        t0 + Duration::from_millis(ms)
    }

    #[test]
    fn acumula_solo_mientras_suena() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        assert!(t.start(song("a"), t0, E0, true).is_none());
        t.observe(true, at(t0, 4_000), E0 + 4_000);
        t.observe(false, at(t0, 6_000), E0 + 6_000);
        t.observe(false, at(t0, 60_000), E0 + 60_000);
        let e = t.finish(at(t0, 60_000), E0 + 60_000).expect("cuenta: 6 s escuchados");
        assert_eq!(e.listened_ms, 6_000);
        assert_eq!(e.started_at, E0);
    }

    #[test]
    fn ended_at_es_el_ultimo_instante_sonando_si_se_cierra_en_pausa() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        t.observe(false, at(t0, 8_000), E0 + 8_000);
        // la app queda horas en pausa y se cierra mucho después
        let e = t.finish(at(t0, 3 * 3_600_000), E0 + 3 * 3_600_000).unwrap();
        assert_eq!(e.ended_at, E0 + 8_000);
        assert_eq!(e.listened_ms, 8_000);
    }

    #[test]
    fn menos_de_cinco_segundos_no_genera_evento() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        t.observe(true, at(t0, 4_999), E0 + 4_999);
        assert!(t.finish(at(t0, 4_999), E0 + 4_999).is_none());
    }

    #[test]
    fn exactamente_cinco_segundos_si_cuenta() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        let e = t.finish(at(t0, 5_000), E0 + 5_000).expect("5 s justos cuentan");
        assert_eq!(e.listened_ms, 5_000);
    }

    #[test]
    fn pausa_y_reanuda_suma_solo_los_tramos_sonando() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        t.observe(false, at(t0, 4_000), E0 + 4_000); // sonó 4 s
        t.observe(true, at(t0, 10_000), E0 + 10_000); // 6 s en pausa: no suman
        t.observe(true, at(t0, 13_000), E0 + 13_000); // sonó 3 s más
        let e = t.finish(at(t0, 13_000), E0 + 13_000).unwrap();
        assert_eq!(e.listened_ms, 7_000);
        assert_eq!(e.ended_at, E0 + 13_000);
    }

    #[test]
    fn empezar_otra_cancion_devuelve_el_evento_de_la_anterior() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        let prev = t.start(song("b"), at(t0, 8_000), E0 + 8_000, true).expect("la anterior cuenta");
        assert_eq!(prev.song.path, "a");
        assert_eq!(prev.listened_ms, 8_000);
        let next = t.finish(at(t0, 20_000), E0 + 20_000).unwrap();
        assert_eq!(next.song.path, "b");
        assert_eq!(next.started_at, E0 + 8_000);
        assert_eq!(next.listened_ms, 12_000);
    }

    #[test]
    fn repetir_la_misma_cancion_cuenta_como_otra_reproduccion() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        let first = t.start(song("a"), at(t0, 200_000), E0 + 200_000, true).expect("primera vuelta");
        assert_eq!(first.listened_ms, 200_000);
        assert!(t.finish(at(t0, 205_000), E0 + 205_000).is_some());
    }

    #[test]
    fn una_sesion_que_empieza_en_pausa_no_suma_hasta_que_suena() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, false);
        t.observe(false, at(t0, 30_000), E0 + 30_000);
        t.observe(true, at(t0, 31_000), E0 + 31_000);
        t.observe(true, at(t0, 37_000), E0 + 37_000);
        let e = t.finish(at(t0, 37_000), E0 + 37_000).unwrap();
        assert_eq!(e.listened_ms, 6_000);
    }

    #[test]
    fn finish_sin_sesion_no_devuelve_nada_y_finish_es_idempotente() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        assert!(t.finish(t0, E0).is_none());
        t.start(song("a"), t0, E0, true);
        assert!(t.finish(at(t0, 6_000), E0 + 6_000).is_some());
        assert!(t.finish(at(t0, 7_000), E0 + 7_000).is_none());
    }

    #[test]
    fn ended_at_nunca_es_anterior_a_started_at() {
        let t0 = Instant::now();
        let mut t = SessionTracker::new();
        t.start(song("a"), t0, E0, true);
        // el reloj del sistema retrocede durante la sesión
        let e = t.finish(at(t0, 6_000), E0 - 10_000).unwrap();
        assert!(e.ended_at >= e.started_at);
    }
}
```

- [ ] **Step 2: Ejecutar para verificar que falla**

Run: `cd native && cargo test stats::session`
Expected: FAIL — varias pruebas por aserción (los métodos devuelven `None`/no hacen nada); pasan solo las que esperan `None`.

- [ ] **Step 3: Implementar**

Reemplazar las tres funciones del `impl SessionTracker` por:

```rust
    pub fn start(&mut self, song: SongSnapshot, now: Instant, epoch_ms: i64, playing: bool) -> Option<PlayEvent> {
        let previous = self.finish(now, epoch_ms);
        self.active = Some(Active {
            song,
            started_at: epoch_ms,
            listened_ms: 0,
            last_instant: now,
            playing,
            last_playing_epoch: epoch_ms,
        });
        previous
    }

    pub fn observe(&mut self, playing: bool, now: Instant, epoch_ms: i64) {
        let Some(a) = &mut self.active else { return };
        if a.playing {
            a.listened_ms += elapsed_ms(a.last_instant, now);
            a.last_playing_epoch = epoch_ms;
        }
        a.last_instant = now;
        a.playing = playing;
    }

    pub fn finish(&mut self, now: Instant, epoch_ms: i64) -> Option<PlayEvent> {
        let mut a = self.active.take()?;
        if a.playing {
            a.listened_ms += elapsed_ms(a.last_instant, now);
            a.last_playing_epoch = epoch_ms;
        }
        if a.listened_ms < MIN_LISTEN_MS {
            return None;
        }
        Some(PlayEvent {
            started_at: a.started_at,
            ended_at: a.last_playing_epoch.max(a.started_at),
            listened_ms: a.listened_ms,
            song: a.song,
        })
    }
```

Y agregar, fuera del `impl` (antes del bloque `#[cfg(test)]`):

```rust
fn elapsed_ms(from: Instant, to: Instant) -> u64 {
    to.saturating_duration_since(from).as_millis() as u64
}
```

- [ ] **Step 4: Ejecutar para verificar que pasa**

Run: `cd native && cargo test stats::session`
Expected: PASS (10 pruebas).

- [ ] **Step 5: Commit**

```bash
git add native/src/stats/session.rs native/src/stats/mod.rs
git commit -m "Estadísticas: SessionTracker mide el tiempo real escuchado"
```

---

### Task 3: Almacén SQLite (`Store`)

**Files:**
- Create: `native/src/stats/store.rs`
- Modify: `native/Cargo.toml`, `native/src/stats/mod.rs`

**Interfaces:**
- Consumes: `model::{PlayEvent, SongSnapshot, EventRow, TopEntry, Totals}`.
- Produces:
  - `store::StoreError(pub String)` (impl `Debug`, `From<rusqlite::Error>`) y `store::StoreResult<T> = Result<T, StoreError>`.
  - `Store::open(path: &std::path::Path) -> StoreResult<Store>`, `Store::open_in_memory() -> StoreResult<Store>`.
  - `Store::insert(&self, event: &PlayEvent) -> StoreResult<()>`; `Store::insert_batch(&mut self, events: &[PlayEvent]) -> StoreResult<()>`.
  - `Store::totals(&self, start: Option<i64>, end: i64) -> StoreResult<Totals>`.
  - `Store::rows_in_range(&self, start: Option<i64>, end: i64) -> StoreResult<Vec<EventRow>>` (ordenadas por `started_at`).
  - `Store::top_songs / top_artists / top_albums(&self, start: Option<i64>, end: i64, limit: u32) -> StoreResult<Vec<TopEntry>>`.
  - `Store::first_event_end(&self) -> StoreResult<Option<i64>>` (mínimo `ended_at`); `Store::all_ended_at(&self) -> StoreResult<Vec<i64>>` (ascendente); `Store::count(&self) -> StoreResult<u64>`.

- [ ] **Step 1: Agregar la dependencia y registrar el módulo**

En `native/Cargo.toml`, bajo `chrono`:

```toml
rusqlite = { version = "0.40", features = ["bundled"] }
```

En `native/src/stats/mod.rs` agregar `pub mod store;`.

- [ ] **Step 2: Escribir las pruebas que fallan**

Crear `native/src/stats/store.rs` con tipos y funciones vacías más las pruebas:

```rust
use super::model::{EventRow, PlayEvent, TopEntry, Totals};
use rusqlite::{params, Connection, Statement};
use std::path::Path;

#[derive(Debug)]
pub struct StoreError(pub String);

impl From<rusqlite::Error> for StoreError {
    fn from(e: rusqlite::Error) -> Self {
        StoreError(e.to_string())
    }
}

pub type StoreResult<T> = Result<T, StoreError>;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(_path: &Path) -> StoreResult<Store> {
        Err(StoreError("sin implementar".into()))
    }

    pub fn open_in_memory() -> StoreResult<Store> {
        Err(StoreError("sin implementar".into()))
    }

    pub fn insert(&self, _event: &PlayEvent) -> StoreResult<()> {
        Ok(())
    }

    pub fn insert_batch(&mut self, _events: &[PlayEvent]) -> StoreResult<()> {
        Ok(())
    }

    pub fn count(&self) -> StoreResult<u64> {
        Ok(0)
    }

    pub fn totals(&self, _start: Option<i64>, _end: i64) -> StoreResult<Totals> {
        Ok(Totals { listened_ms: 0, plays: 0, unique_songs: 0 })
    }

    pub fn rows_in_range(&self, _start: Option<i64>, _end: i64) -> StoreResult<Vec<EventRow>> {
        Ok(Vec::new())
    }

    pub fn top_songs(&self, _start: Option<i64>, _end: i64, _limit: u32) -> StoreResult<Vec<TopEntry>> {
        Ok(Vec::new())
    }

    pub fn top_artists(&self, _start: Option<i64>, _end: i64, _limit: u32) -> StoreResult<Vec<TopEntry>> {
        Ok(Vec::new())
    }

    pub fn top_albums(&self, _start: Option<i64>, _end: i64, _limit: u32) -> StoreResult<Vec<TopEntry>> {
        Ok(Vec::new())
    }

    pub fn first_event_end(&self) -> StoreResult<Option<i64>> {
        Ok(None)
    }

    pub fn all_ended_at(&self) -> StoreResult<Vec<i64>> {
        Ok(Vec::new())
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
}
```

- [ ] **Step 3: Ejecutar para verificar que falla**

Run: `cd native && cargo test stats::store`
Expected: error de compilación `no function or associated item named init` (la prueba de versión usa `Store::init`) y, al añadirla, fallos por aserción.

- [ ] **Step 4: Implementar**

Reemplazar el `impl Store` completo por:

```rust
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
             WHERE {RANGE_FILTER} ORDER BY started_at ASC, id ASC"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map(params![start, end], |r| {
            Ok(EventRow { started_at: r.get(0)?, ended_at: r.get(1)?, listened_ms: r.get::<_, i64>(2)? as u64 })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn top_songs(&self, start: Option<i64>, end: i64, limit: u32) -> StoreResult<Vec<TopEntry>> {
        self.run_top(
            "SELECT MAX(title), MAX(artist), song_path, SUM(listened_ms) AS t, COUNT(*) AS n
             FROM play_events WHERE {FILTER}
             GROUP BY song_path ORDER BY t DESC, n DESC, MAX(title) ASC LIMIT ?3",
            start,
            end,
            limit,
        )
    }

    pub fn top_artists(&self, start: Option<i64>, end: i64, limit: u32) -> StoreResult<Vec<TopEntry>> {
        self.run_top(
            "SELECT artist, '', MIN(song_path), SUM(listened_ms) AS t, COUNT(*) AS n
             FROM play_events WHERE {FILTER}
             GROUP BY artist ORDER BY t DESC, n DESC, artist ASC LIMIT ?3",
            start,
            end,
            limit,
        )
    }

    pub fn top_albums(&self, start: Option<i64>, end: i64, limit: u32) -> StoreResult<Vec<TopEntry>> {
        self.run_top(
            "SELECT album, artist, MIN(song_path), SUM(listened_ms) AS t, COUNT(*) AS n
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
```

- [ ] **Step 5: Ejecutar para verificar que pasa**

Run: `cd native && cargo test stats::store`
Expected: PASS (11 pruebas). La primera compilación descarga y compila SQLite (~40 s).

- [ ] **Step 6: Commit**

```bash
git add native/Cargo.toml native/Cargo.lock native/src/stats/store.rs native/src/stats/mod.rs
git commit -m "Estadísticas: almacén SQLite con migración, consultas por rango y tops"
```

---

### Task 4: Rangos, hábitos y línea de tiempo (`aggregate`)

**Files:**
- Create: `native/src/stats/aggregate.rs`
- Modify: `native/src/stats/mod.rs`

**Interfaces:**
- Consumes: `model::{EventRow, Habits, StatsRange, TimelineBucket}`, `format::{month_short, weekday_full, weekday_short}`, `chrono`.
- Produces:
  - `aggregate::RangeBounds { pub start_ms: Option<i64>, pub end_ms: i64 }`.
  - `aggregate::range_bounds<Tz: chrono::TimeZone>(range: StatsRange, now: &chrono::DateTime<Tz>) -> RangeBounds`.
  - `aggregate::days_in_span<Tz: TimeZone>(range: StatsRange, now: &DateTime<Tz>, first_event_end_ms: Option<i64>) -> u32`.
  - `aggregate::habits<Tz: TimeZone>(rows: &[EventRow], all_ended_at: &[i64], days_in_span: u32, now: &DateTime<Tz>) -> Habits`.
  - `aggregate::timeline<Tz: TimeZone>(rows: &[EventRow], range: StatsRange, now: &DateTime<Tz>, first_event_end_ms: Option<i64>) -> (Vec<TimelineBucket>, Option<usize>)`.

- [ ] **Step 1: Registrar el módulo y escribir las pruebas que fallan**

En `native/src/stats/mod.rs` agregar `pub mod aggregate;`.

Crear `native/src/stats/aggregate.rs` con el esqueleto y las pruebas. Fecha de referencia de las pruebas: **sábado 2026-10-03 15:30** en una zona de desfase fijo UTC−6.

```rust
use super::model::{EventRow, Habits, StatsRange, TimelineBucket};
use chrono::{DateTime, TimeZone};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeBounds {
    pub start_ms: Option<i64>,
    pub end_ms: i64,
}

/// Hueco máximo entre dos reproducciones para que sigan en la misma sesión.
pub const SESSION_GAP_MS: i64 = 30 * 60 * 1000;

pub fn range_bounds<Tz: TimeZone>(_range: StatsRange, now: &DateTime<Tz>) -> RangeBounds {
    RangeBounds { start_ms: None, end_ms: now.timestamp_millis() }
}

pub fn days_in_span<Tz: TimeZone>(_range: StatsRange, _now: &DateTime<Tz>, _first_event_end_ms: Option<i64>) -> u32 {
    0
}

pub fn habits<Tz: TimeZone>(
    _rows: &[EventRow],
    _all_ended_at: &[i64],
    _days_in_span: u32,
    _now: &DateTime<Tz>,
) -> Habits {
    Habits {
        active_days: 0,
        current_streak_days: 0,
        longest_streak_days: 0,
        sessions: 0,
        avg_session_ms: 0,
        longest_session_ms: 0,
        sessions_per_day: 0.0,
        peak_weekday: None,
    }
}

pub fn timeline<Tz: TimeZone>(
    _rows: &[EventRow],
    _range: StatsRange,
    _now: &DateTime<Tz>,
    _first_event_end_ms: Option<i64>,
) -> (Vec<TimelineBucket>, Option<usize>) {
    (Vec::new(), None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::west_opt(6 * 3600).unwrap()
    }

    /// Milisegundos de una fecha-hora local en la zona de prueba.
    fn ms(y: i32, m: u32, d: u32, h: u32, min: u32) -> i64 {
        tz().with_ymd_and_hms(y, m, d, h, min, 0).unwrap().timestamp_millis()
    }

    fn now() -> DateTime<FixedOffset> {
        tz().with_ymd_and_hms(2026, 10, 3, 15, 30, 0).unwrap() // sábado
    }

    fn row(start: i64, end: i64, listened: u64) -> EventRow {
        EventRow { started_at: start, ended_at: end, listened_ms: listened }
    }

    fn ended_on(y: i32, m: u32, d: u32, h: u32) -> i64 {
        ms(y, m, d, h, 0)
    }

    // ── rangos ─────────────────────────────────────────────────────────────
    #[test]
    fn los_inicios_de_rango_son_medianoche_local_y_la_semana_empieza_el_lunes() {
        let n = now();
        assert_eq!(range_bounds(StatsRange::Today, &n).start_ms, Some(ms(2026, 10, 3, 0, 0)));
        assert_eq!(range_bounds(StatsRange::Week, &n).start_ms, Some(ms(2026, 9, 28, 0, 0)));
        assert_eq!(range_bounds(StatsRange::Month, &n).start_ms, Some(ms(2026, 10, 1, 0, 0)));
        assert_eq!(range_bounds(StatsRange::Year, &n).start_ms, Some(ms(2026, 1, 1, 0, 0)));
        assert_eq!(range_bounds(StatsRange::All, &n).start_ms, None);
        assert_eq!(range_bounds(StatsRange::Week, &n).end_ms, n.timestamp_millis());
    }

    #[test]
    fn la_semana_de_un_lunes_empieza_ese_mismo_dia() {
        let monday = tz().with_ymd_and_hms(2026, 9, 28, 9, 0, 0).unwrap();
        assert_eq!(range_bounds(StatsRange::Week, &monday).start_ms, Some(ms(2026, 9, 28, 0, 0)));
    }

    #[test]
    fn dias_del_rango() {
        let n = now();
        assert_eq!(days_in_span(StatsRange::Today, &n, None), 1);
        assert_eq!(days_in_span(StatsRange::Week, &n, None), 6); // lun..sáb
        assert_eq!(days_in_span(StatsRange::Month, &n, None), 3);
        assert_eq!(days_in_span(StatsRange::Year, &n, None), 276);
        assert_eq!(days_in_span(StatsRange::All, &n, Some(ms(2026, 9, 20, 10, 0))), 14);
    }

    #[test]
    fn todo_sin_eventos_cuenta_un_dia_sin_dividir_por_cero() {
        assert_eq!(days_in_span(StatsRange::All, &now(), None), 1);
    }

    // ── hábitos ────────────────────────────────────────────────────────────
    #[test]
    fn dias_activos_y_racha_mas_larga() {
        let rows = [
            row(0, ended_on(2026, 9, 28, 10), 60_000),
            row(0, ended_on(2026, 10, 1, 10), 60_000),
            row(0, ended_on(2026, 10, 2, 22), 60_000),
            row(0, ended_on(2026, 10, 3, 9), 60_000),
        ];
        let h = habits(&rows, &[], 6, &now());
        assert_eq!(h.active_days, 4);
        assert_eq!(h.longest_streak_days, 3);
    }

    #[test]
    fn la_racha_cruza_el_fin_de_mes() {
        let rows = [
            row(0, ended_on(2026, 9, 29, 10), 60_000),
            row(0, ended_on(2026, 9, 30, 10), 60_000),
            row(0, ended_on(2026, 10, 1, 10), 60_000),
        ];
        assert_eq!(habits(&rows, &[], 30, &now()).longest_streak_days, 3);
    }

    #[test]
    fn la_racha_actual_cuenta_desde_hoy_o_desde_ayer() {
        let con_hoy = [ended_on(2026, 10, 3, 9), ended_on(2026, 10, 2, 9), ended_on(2026, 10, 1, 9), ended_on(2026, 9, 29, 9)];
        assert_eq!(habits(&[], &con_hoy, 1, &now()).current_streak_days, 3);

        let desde_ayer = [ended_on(2026, 10, 2, 9), ended_on(2026, 10, 1, 9)];
        assert_eq!(habits(&[], &desde_ayer, 1, &now()).current_streak_days, 2);

        let rota = [ended_on(2026, 9, 30, 9)];
        assert_eq!(habits(&[], &rota, 1, &now()).current_streak_days, 0);
        assert_eq!(habits(&[], &[], 1, &now()).current_streak_days, 0);
    }

    #[test]
    fn un_hueco_de_exactamente_30_min_une_la_sesion_y_30_min_y_1_ms_la_separa() {
        let a = row(ms(2026, 10, 3, 10, 0), ms(2026, 10, 3, 10, 10), 600_000);
        let b = row(ms(2026, 10, 3, 10, 40), ms(2026, 10, 3, 10, 50), 600_000); // hueco 30 min justos
        let c = row(ms(2026, 10, 3, 11, 20) + 1, ms(2026, 10, 3, 11, 30), 300_000); // hueco 30 min + 1 ms
        let h = habits(&[a, b, c], &[], 1, &now());
        assert_eq!(h.sessions, 2);
        assert_eq!(h.longest_session_ms, 1_200_000);
        assert_eq!(h.avg_session_ms, (1_200_000 + 300_000) / 2);
        assert!((h.sessions_per_day - 2.0).abs() < 1e-9);
    }

    #[test]
    fn el_dia_pico_solo_existe_si_el_rango_abarca_mas_de_un_dia() {
        let rows = [
            row(0, ended_on(2026, 10, 3, 9), 3_000_000),  // sábado
            row(0, ended_on(2026, 10, 1, 9), 1_000_000),  // jueves
        ];
        let h = habits(&rows, &[], 6, &now());
        assert_eq!(h.peak_weekday, Some(("sábado".to_string(), 3_000_000)));
        assert_eq!(habits(&rows, &[], 1, &now()).peak_weekday, None);
    }

    #[test]
    fn sin_eventos_los_habitos_son_cero() {
        let h = habits(&[], &[], 6, &now());
        assert_eq!((h.active_days, h.sessions, h.longest_streak_days, h.current_streak_days), (0, 0, 0, 0));
        assert_eq!((h.avg_session_ms, h.longest_session_ms), (0, 0));
        assert_eq!(h.peak_weekday, None);
    }

    // ── línea de tiempo ────────────────────────────────────────────────────
    #[test]
    fn hoy_tiene_24_horas_y_marca_el_pico() {
        let rows = [row(0, ms(2026, 10, 3, 15, 30), 60_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::Today, &now(), None);
        assert_eq!(buckets.len(), 24);
        assert_eq!((buckets[0].label.as_str(), buckets[23].label.as_str()), ("00", "23"));
        assert_eq!(buckets[15].listened_ms, 60_000);
        assert_eq!(peak, Some(15));
    }

    #[test]
    fn la_semana_tiene_7_dias_de_lunes_a_domingo() {
        let rows = [row(0, ms(2026, 9, 30, 12, 0), 90_000)]; // miércoles
        let (buckets, peak) = timeline(&rows, StatsRange::Week, &now(), None);
        let labels: Vec<&str> = buckets.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"]);
        assert_eq!((buckets[2].listened_ms, peak), (90_000, Some(2)));
    }

    #[test]
    fn el_mes_tiene_un_bucket_por_dia() {
        let rows = [row(0, ms(2026, 10, 3, 12, 0), 10_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::Month, &now(), None);
        assert_eq!(buckets.len(), 31); // octubre
        assert_eq!((buckets[0].label.as_str(), buckets[30].label.as_str()), ("1", "31"));
        assert_eq!((buckets[2].listened_ms, peak), (10_000, Some(2)));
    }

    #[test]
    fn el_ano_tiene_12_meses() {
        let rows = [row(0, ms(2026, 10, 3, 12, 0), 10_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::Year, &now(), None);
        assert_eq!(buckets.len(), 12);
        assert_eq!((buckets[0].label.as_str(), buckets[11].label.as_str()), ("ene", "dic"));
        assert_eq!(peak, Some(9));
    }

    #[test]
    fn todo_va_por_mes_si_el_historial_cabe_en_36_meses() {
        let first = ms(2026, 8, 15, 10, 0);
        let rows = [row(0, ms(2026, 9, 10, 10, 0), 5_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::All, &now(), Some(first));
        let labels: Vec<&str> = buckets.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["ago 26", "sep 26", "oct 26"]);
        assert_eq!(peak, Some(1));
    }

    #[test]
    fn todo_va_por_ano_si_el_historial_supera_36_meses() {
        let first = ms(2022, 5, 1, 10, 0);
        let rows = [row(0, ms(2024, 3, 1, 10, 0), 5_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::All, &now(), Some(first));
        let labels: Vec<&str> = buckets.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["2022", "2023", "2024", "2025", "2026"]);
        assert_eq!(peak, Some(2));
    }

    #[test]
    fn todo_sin_eventos_no_tiene_linea_de_tiempo_ni_pico() {
        let (buckets, peak) = timeline(&[], StatsRange::All, &now(), None);
        assert!(buckets.is_empty());
        assert_eq!(peak, None);
    }

    #[test]
    fn sin_tiempo_en_ningun_bucket_no_hay_pico() {
        let (buckets, peak) = timeline(&[], StatsRange::Week, &now(), None);
        assert_eq!(buckets.len(), 7);
        assert_eq!(peak, None);
    }

    #[test]
    fn un_evento_que_termina_justo_a_medianoche_pertenece_al_dia_nuevo() {
        let rows = [row(0, ms(2026, 10, 3, 0, 0), 60_000)];
        let (buckets, _) = timeline(&rows, StatsRange::Today, &now(), None);
        assert_eq!(buckets[0].listened_ms, 60_000);
        let (week, _) = timeline(&rows, StatsRange::Week, &now(), None);
        assert_eq!(week[5].listened_ms, 60_000); // sábado, no viernes
    }
}
```

- [ ] **Step 2: Ejecutar para verificar que falla**

Run: `cd native && cargo test stats::aggregate`
Expected: FAIL — fallan por aserción casi todas (pasan solo las que esperan vacíos/ceros).

- [ ] **Step 3: Implementar**

Reemplazar el cuerpo de las cuatro funciones y agregar los helpers (conservar `RangeBounds`, `SESSION_GAP_MS` y el bloque `tests`). Cambiar la primera línea de imports a:

```rust
use super::format::{month_short, weekday_full, weekday_short};
use super::model::{EventRow, Habits, StatsRange, TimelineBucket};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Timelike, Weekday};
use std::collections::{BTreeMap, BTreeSet};
```

```rust
fn day_start_ms<Tz: TimeZone>(tz: &Tz, date: NaiveDate) -> Option<i64> {
    let naive = date.and_hms_opt(0, 0, 0)?;
    tz.from_local_datetime(&naive).earliest().map(|d| d.timestamp_millis())
}

fn date_of<Tz: TimeZone>(tz: &Tz, ms: i64) -> Option<NaiveDate> {
    tz.timestamp_millis_opt(ms).single().map(|d| d.date_naive())
}

fn local<Tz: TimeZone>(tz: &Tz, ms: i64) -> Option<DateTime<Tz>> {
    tz.timestamp_millis_opt(ms).single()
}

pub fn range_bounds<Tz: TimeZone>(range: StatsRange, now: &DateTime<Tz>) -> RangeBounds {
    let tz = now.timezone();
    let today = now.date_naive();
    let start_date = match range {
        StatsRange::Today => Some(today),
        StatsRange::Week => Some(today - Duration::days(today.weekday().num_days_from_monday() as i64)),
        StatsRange::Month => today.with_day(1),
        StatsRange::Year => today.with_ordinal(1),
        StatsRange::All => None,
    };
    RangeBounds {
        start_ms: start_date.and_then(|d| day_start_ms(&tz, d)),
        end_ms: now.timestamp_millis(),
    }
}

pub fn days_in_span<Tz: TimeZone>(range: StatsRange, now: &DateTime<Tz>, first_event_end_ms: Option<i64>) -> u32 {
    let today = now.date_naive();
    let days = match range {
        StatsRange::Today => 1,
        StatsRange::Week => today.weekday().num_days_from_monday() as i64 + 1,
        StatsRange::Month => today.day() as i64,
        StatsRange::Year => today.ordinal() as i64,
        StatsRange::All => first_event_end_ms
            .and_then(|ms| date_of(&now.timezone(), ms))
            .map(|first| (today - first).num_days() + 1)
            .unwrap_or(1),
    };
    days.max(1) as u32
}

fn longest_run(days: &BTreeSet<NaiveDate>) -> u32 {
    let (mut best, mut current, mut previous) = (0u32, 0u32, None::<NaiveDate>);
    for day in days {
        current = if previous.map_or(false, |p| p + Duration::days(1) == *day) { current + 1 } else { 1 };
        best = best.max(current);
        previous = Some(*day);
    }
    best
}

fn current_run(days: &BTreeSet<NaiveDate>, today: NaiveDate) -> u32 {
    let yesterday = today - Duration::days(1);
    let mut cursor = if days.contains(&today) {
        today
    } else if days.contains(&yesterday) {
        yesterday
    } else {
        return 0;
    };
    let mut run = 0;
    while days.contains(&cursor) {
        run += 1;
        cursor -= Duration::days(1);
    }
    run
}

pub fn habits<Tz: TimeZone>(
    rows: &[EventRow],
    all_ended_at: &[i64],
    days_in_span: u32,
    now: &DateTime<Tz>,
) -> Habits {
    let tz = now.timezone();
    let active: BTreeSet<NaiveDate> = rows.iter().filter_map(|r| date_of(&tz, r.ended_at)).collect();
    let history: BTreeSet<NaiveDate> = all_ended_at.iter().filter_map(|ms| date_of(&tz, *ms)).collect();

    // Sesiones: se une la reproducción siguiente si empieza a lo sumo 30 min después de que terminó la sesión.
    let mut sorted: Vec<&EventRow> = rows.iter().collect();
    sorted.sort_by_key(|r| r.started_at);
    let mut sessions: Vec<u64> = Vec::new();
    let mut session_end = i64::MIN;
    for r in sorted {
        if !sessions.is_empty() && r.started_at - session_end <= SESSION_GAP_MS {
            *sessions.last_mut().unwrap() += r.listened_ms;
            session_end = session_end.max(r.ended_at);
        } else {
            sessions.push(r.listened_ms);
            session_end = r.ended_at;
        }
    }
    let total: u64 = sessions.iter().sum();
    let count = sessions.len() as u32;

    let peak_weekday = if days_in_span > 1 {
        let mut by_day: BTreeMap<u32, (Weekday, u64)> = BTreeMap::new();
        for r in rows {
            if let Some(date) = date_of(&tz, r.ended_at) {
                let entry = by_day.entry(date.weekday().num_days_from_monday()).or_insert((date.weekday(), 0));
                entry.1 += r.listened_ms;
            }
        }
        by_day
            .values()
            .fold(None::<(Weekday, u64)>, |best, cur| match best {
                Some(b) if b.1 >= cur.1 => Some(b),
                _ => Some(*cur),
            })
            .filter(|(_, ms)| *ms > 0)
            .map(|(day, ms)| (weekday_full(day).to_string(), ms))
    } else {
        None
    };

    Habits {
        active_days: active.len() as u32,
        current_streak_days: current_run(&history, now.date_naive()),
        longest_streak_days: longest_run(&active),
        sessions: count,
        avg_session_ms: if count > 0 { total / count as u64 } else { 0 },
        longest_session_ms: sessions.iter().copied().max().unwrap_or(0),
        sessions_per_day: count as f64 / days_in_span.max(1) as f64,
        peak_weekday,
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let next = if month == 12 { NaiveDate::from_ymd_opt(year + 1, 1, 1) } else { NaiveDate::from_ymd_opt(year, month + 1, 1) };
    (next.unwrap() - first).num_days() as u32
}

pub fn timeline<Tz: TimeZone>(
    rows: &[EventRow],
    range: StatsRange,
    now: &DateTime<Tz>,
    first_event_end_ms: Option<i64>,
) -> (Vec<TimelineBucket>, Option<usize>) {
    let tz = now.timezone();
    let bucket = |label: String| TimelineBucket { label, listened_ms: 0 };

    // (etiquetas, función que lleva un instante local a su índice de bucket)
    let (mut buckets, index_of): (Vec<TimelineBucket>, Box<dyn Fn(&DateTime<Tz>) -> Option<usize>>) = match range {
        StatsRange::Today => ((0..24).map(|h| bucket(format!("{h:02}"))).collect(), Box::new(|d| Some(d.hour() as usize))),
        StatsRange::Week => (
            [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun]
                .iter()
                .map(|d| bucket(weekday_short(*d).to_string()))
                .collect(),
            Box::new(|d| Some(d.weekday().num_days_from_monday() as usize)),
        ),
        StatsRange::Month => (
            (1..=days_in_month(now.year(), now.month())).map(|d| bucket(d.to_string())).collect(),
            Box::new(|d| Some(d.day0() as usize)),
        ),
        StatsRange::Year => (
            (1..=12).map(|m| bucket(month_short(m).to_string())).collect(),
            Box::new(|d| Some(d.month0() as usize)),
        ),
        StatsRange::All => {
            let Some(first) = first_event_end_ms.and_then(|ms| local(&tz, ms)) else {
                return (Vec::new(), None);
            };
            let months = (now.year() - first.year()) * 12 + now.month0() as i32 - first.month0() as i32 + 1;
            if months <= 36 {
                let (y0, m0) = (first.year(), first.month0() as i32);
                let labels = (0..months)
                    .map(|i| {
                        let total = m0 + i;
                        let (year, month) = (y0 + total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
                        bucket(format!("{} {:02}", month_short(month), year.rem_euclid(100)))
                    })
                    .collect();
                (labels, Box::new(move |d| {
                    let i = (d.year() - y0) * 12 + d.month0() as i32 - m0;
                    usize::try_from(i).ok()
                }))
            } else {
                let y0 = first.year();
                let labels = (y0..=now.year()).map(|y| bucket(y.to_string())).collect();
                (labels, Box::new(move |d| usize::try_from(d.year() - y0).ok()))
            }
        }
    };

    for row in rows {
        if let Some(i) = local(&tz, row.ended_at).as_ref().and_then(|d| index_of(d)) {
            if let Some(b) = buckets.get_mut(i) {
                b.listened_ms += row.listened_ms;
            }
        }
    }
    let peak = buckets
        .iter()
        .enumerate()
        .filter(|(_, b)| b.listened_ms > 0)
        .fold(None::<(usize, u64)>, |best, (i, b)| match best {
            Some((_, v)) if v >= b.listened_ms => best,
            _ => Some((i, b.listened_ms)),
        })
        .map(|(i, _)| i);
    (buckets, peak)
}
```

- [ ] **Step 4: Ejecutar para verificar que pasa**

Run: `cd native && cargo test stats::aggregate`
Expected: PASS (17 pruebas). Si el cierre `Box<dyn Fn(&DateTime<Tz>) ...>` da errores de tiempo de vida con `Tz`, añadir `+ '_`/`'static` al `Box` o convertirlo en una función `fn bucket_index(range, first, now, dt) -> Option<usize>` con un `match`; el comportamiento no cambia.

- [ ] **Step 5: Commit**

```bash
git add native/src/stats/aggregate.rs native/src/stats/mod.rs
git commit -m "Estadísticas: rangos, hábitos y línea de tiempo (cálculo puro)"
```

---

### Task 5: Resumen completo (`summary`)

**Files:**
- Create: `native/src/stats/summary.rs`
- Modify: `native/src/stats/mod.rs`

**Interfaces:**
- Consumes: `store::{Store, StoreResult}`, `aggregate::{range_bounds, days_in_span, habits, timeline}`, `model::{StatsRange, StatsSummary}`.
- Produces: `summary::build_summary<Tz: chrono::TimeZone>(store: &Store, range: StatsRange, now: &chrono::DateTime<Tz>) -> StoreResult<StatsSummary>` y `summary::TOP_LIMIT: u32 = 5`.

- [ ] **Step 1: Registrar el módulo y escribir las pruebas que fallan**

En `native/src/stats/mod.rs` agregar `pub mod summary;`.

Crear `native/src/stats/summary.rs`:

```rust
use super::model::{Habits, StatsRange, StatsSummary, Totals};
use super::store::{Store, StoreResult};
use chrono::{DateTime, TimeZone};

pub const TOP_LIMIT: u32 = 5;

pub fn build_summary<Tz: TimeZone>(_store: &Store, range: StatsRange, _now: &DateTime<Tz>) -> StoreResult<StatsSummary> {
    Ok(StatsSummary {
        range,
        totals: Totals { listened_ms: 0, plays: 0, unique_songs: 0 },
        avg_daily_ms: 0,
        days_in_span: 0,
        top_songs: Vec::new(),
        top_artists: Vec::new(),
        top_albums: Vec::new(),
        habits: Habits {
            active_days: 0,
            current_streak_days: 0,
            longest_streak_days: 0,
            sessions: 0,
            avg_session_ms: 0,
            longest_session_ms: 0,
            sessions_per_day: 0.0,
            peak_weekday: None,
        },
        timeline: Vec::new(),
        peak_bucket: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::model::{PlayEvent, SongSnapshot};
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::west_opt(6 * 3600).unwrap()
    }

    fn now() -> DateTime<FixedOffset> {
        tz().with_ymd_and_hms(2026, 10, 3, 15, 30, 0).unwrap()
    }

    fn at(d: u32, h: u32) -> i64 {
        tz().with_ymd_and_hms(2026, 10, d, h, 0, 0).unwrap().timestamp_millis()
    }

    fn ev(path: &str, artist: &str, day: u32, hour: u32, listened: u64) -> PlayEvent {
        PlayEvent {
            started_at: at(day, hour),
            ended_at: at(day, hour) + listened as i64,
            listened_ms: listened,
            song: SongSnapshot {
                path: path.into(),
                title: path.to_uppercase(),
                artist: artist.into(),
                album: "Disco".into(),
                duration_ms: 200_000,
            },
        }
    }

    fn seeded() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        s.insert_batch(&[
            ev("a", "Uno", 1, 10, 200_000),
            ev("a", "Uno", 2, 10, 200_000),
            ev("b", "Dos", 3, 9, 100_000),
            ev("a", "Uno", 3, 9, 200_000), // hoy, mismo bloque horario que b
        ])
        .unwrap();
        s
    }

    #[test]
    fn el_resumen_de_todo_combina_totales_tops_habitos_y_linea_de_tiempo() {
        let s = build_summary(&seeded(), StatsRange::All, &now()).unwrap();
        assert_eq!((s.totals.listened_ms, s.totals.plays, s.totals.unique_songs), (700_000, 4, 2));
        assert_eq!(s.days_in_span, 3); // 1..3 de octubre
        assert_eq!(s.avg_daily_ms, 700_000 / 3);
        assert_eq!(s.top_songs[0].name, "A");
        assert_eq!(s.top_artists[0].name, "Uno");
        assert_eq!(s.habits.active_days, 3);
        assert_eq!(s.habits.current_streak_days, 3);
        assert_eq!(s.timeline.len(), 1);
        assert_eq!(s.peak_bucket, Some(0));
    }

    #[test]
    fn el_resumen_de_hoy_solo_incluye_lo_de_hoy() {
        let s = build_summary(&seeded(), StatsRange::Today, &now()).unwrap();
        assert_eq!((s.totals.plays, s.totals.listened_ms), (2, 300_000));
        assert_eq!(s.timeline.len(), 24);
        assert_eq!(s.timeline[9].listened_ms, 300_000);
        // la racha actual mira todo el historial aunque el rango sea Hoy
        assert_eq!(s.habits.current_streak_days, 3);
    }

    #[test]
    fn un_rango_sin_eventos_da_un_resumen_vacio_sin_fallar() {
        let empty = Store::open_in_memory().unwrap();
        for range in StatsRange::ALL_RANGES {
            let s = build_summary(&empty, range, &now()).unwrap();
            assert_eq!((s.totals.plays, s.totals.listened_ms, s.avg_daily_ms), (0, 0, 0), "{range:?}");
            assert!(s.top_songs.is_empty() && s.top_artists.is_empty() && s.top_albums.is_empty());
            assert_eq!((s.habits.sessions, s.habits.current_streak_days), (0, 0));
            assert_eq!(s.peak_bucket, None);
            assert!(s.days_in_span >= 1);
        }
    }

    /// Medición, no regresión: `cargo test --release summary::tests::resumen_de_100k -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn resumen_de_100k_eventos_en_menos_de_200_ms() {
        let mut store = Store::open_in_memory().unwrap();
        let base = at(1, 0);
        let events: Vec<PlayEvent> = (0..100_000u32)
            .map(|i| {
                let ended = base - (i as i64) * 600_000; // un evento cada 10 min hacia atrás
                PlayEvent {
                    started_at: ended - 180_000,
                    ended_at: ended,
                    listened_ms: 180_000,
                    song: SongSnapshot {
                        path: format!("/m/{}.mp3", i % 600),
                        title: format!("T{}", i % 600),
                        artist: format!("A{}", i % 150),
                        album: format!("Al{}", i % 200),
                        duration_ms: 200_000,
                    },
                }
            })
            .collect();
        store.insert_batch(&events).unwrap();

        let t0 = std::time::Instant::now();
        let s = build_summary(&store, StatsRange::All, &now()).unwrap();
        let elapsed = t0.elapsed();
        println!("[bench] resumen de {} eventos: {elapsed:?}", s.totals.plays);
        assert!(elapsed.as_millis() < 200, "tardó {elapsed:?}");
    }
}
```

- [ ] **Step 2: Ejecutar para verificar que falla**

Run: `cd native && cargo test stats::summary`
Expected: FAIL por aserción (el esqueleto devuelve ceros).

- [ ] **Step 3: Implementar**

Reemplazar `build_summary` por (y ajustar imports a `use super::aggregate; use super::model::{StatsRange, StatsSummary};`):

```rust
pub fn build_summary<Tz: TimeZone>(store: &Store, range: StatsRange, now: &DateTime<Tz>) -> StoreResult<StatsSummary> {
    let bounds = aggregate::range_bounds(range, now);
    let first = store.first_event_end()?;
    let span = aggregate::days_in_span(range, now, first);

    let totals = store.totals(bounds.start_ms, bounds.end_ms)?;
    let rows = store.rows_in_range(bounds.start_ms, bounds.end_ms)?;
    let all_ended_at = store.all_ended_at()?;
    let (timeline, peak_bucket) = aggregate::timeline(&rows, range, now, first);

    Ok(StatsSummary {
        range,
        totals,
        avg_daily_ms: totals.listened_ms / span as u64,
        days_in_span: span,
        top_songs: store.top_songs(bounds.start_ms, bounds.end_ms, TOP_LIMIT)?,
        top_artists: store.top_artists(bounds.start_ms, bounds.end_ms, TOP_LIMIT)?,
        top_albums: store.top_albums(bounds.start_ms, bounds.end_ms, TOP_LIMIT)?,
        habits: aggregate::habits(&rows, &all_ended_at, span, now),
        timeline,
        peak_bucket,
    })
}
```

- [ ] **Step 4: Ejecutar para verificar que pasa, y medir el benchmark**

Run: `cd native && cargo test stats::summary`
Expected: PASS (3 pruebas; el benchmark queda `ignored`).

Run: `cd native && cargo test --release stats::summary::tests::resumen_de_100k -- --ignored --nocapture`
Expected: PASS e imprime `[bench] resumen de 100000 eventos: …` con un tiempo < 200 ms. Si no cumple, optimizar `rows_in_range`/`all_ended_at` (p. ej. `prepare_cached`) antes de seguir y anotar el tiempo medido en el mensaje del commit.

- [ ] **Step 5: Commit**

```bash
git add native/src/stats/summary.rs native/src/stats/mod.rs
git commit -m "Estadísticas: resumen que une el almacén con los cálculos"
```

---

### Task 6: Servicio de fondo y ubicación de la base

**Files:**
- Create: `native/src/stats/location.rs`, `native/src/stats/service.rs`
- Modify: `native/src/stats/mod.rs`

**Interfaces:**
- Consumes: `store::Store`, `summary::build_summary`, `model::{PlayEvent, StatsRange, StatsSummary}`.
- Produces:
  - `location::StatsLocation { File(PathBuf), Memory }`; `location::default_db_path(xdg_data_home: Option<&str>, home: Option<&str>) -> PathBuf`; `location::location_from_args(args: &[String], xdg_data_home: Option<&str>, home: Option<&str>) -> StatsLocation`.
  - `service::Repaint = std::sync::Arc<dyn Fn() + Send + Sync>`.
  - `service::StatsHandle` (`Clone`) con: `StatsHandle::disabled(reason: &str) -> Self`, `StatsHandle::spawn(location: StatsLocation, repaint: Repaint) -> Self`, `record(&self, PlayEvent)`, `record_batch(&self, Vec<PlayEvent>)`, `request_summary(&self, StatsRange)`, `latest_summary(&self) -> Option<(StatsRange, Arc<StatsSummary>)>`, `events_version(&self) -> u64`, `disabled_reason(&self) -> Option<String>`, `is_enabled(&self) -> bool`, `flush(&self, timeout: Duration)`.

- [ ] **Step 1: Registrar y escribir las pruebas de `location` (fallan)**

En `native/src/stats/mod.rs` agregar `pub mod location;` y `pub mod service;`.

Crear `native/src/stats/location.rs`:

```rust
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatsLocation {
    File(PathBuf),
    Memory,
}

pub fn default_db_path(_xdg_data_home: Option<&str>, _home: Option<&str>) -> PathBuf {
    PathBuf::new()
}

/// Dónde guardar según los argumentos: las corridas de desarrollo nunca tocan la base real.
pub fn location_from_args(_args: &[String], _xdg_data_home: Option<&str>, _home: Option<&str>) -> StatsLocation {
    StatsLocation::Memory
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn la_ruta_por_defecto_respeta_xdg_data_home() {
        assert_eq!(
            default_db_path(Some("/datos"), Some("/home/u")),
            PathBuf::from("/datos/simple-player/stats.db")
        );
    }

    #[test]
    fn sin_xdg_se_usa_local_share_del_home_y_un_xdg_vacio_se_ignora() {
        let esperado = PathBuf::from("/home/u/.local/share/simple-player/stats.db");
        assert_eq!(default_db_path(None, Some("/home/u")), esperado);
        assert_eq!(default_db_path(Some(""), Some("/home/u")), esperado);
    }

    #[test]
    fn una_corrida_normal_usa_la_base_real() {
        assert_eq!(
            location_from_args(&args(&["simple-player"]), None, Some("/home/u")),
            StatsLocation::File(PathBuf::from("/home/u/.local/share/simple-player/stats.db"))
        );
    }

    #[test]
    fn las_corridas_de_desarrollo_usan_memoria() {
        for flag in ["--shot", "--bench", "--gallery", "--stats-demo"] {
            assert_eq!(
                location_from_args(&args(&["simple-player", flag]), None, Some("/home/u")),
                StatsLocation::Memory,
                "{flag}"
            );
        }
    }

    #[test]
    fn stats_db_fuerza_una_ruta_incluso_en_desarrollo() {
        assert_eq!(
            location_from_args(&args(&["simple-player", "--shot", "--stats-db", "/tmp/x.db"]), None, None),
            StatsLocation::File(PathBuf::from("/tmp/x.db"))
        );
    }
}
```

- [ ] **Step 2: Ejecutar para verificar que falla**

Run: `cd native && cargo test stats::location`
Expected: FAIL — la ruta por defecto vacía y la ubicación siempre `Memory` hacen fallar 3 de 5 pruebas.

- [ ] **Step 3: Implementar `location`**

Reemplazar las dos funciones por:

```rust
pub fn default_db_path(xdg_data_home: Option<&str>, home: Option<&str>) -> PathBuf {
    let base = match xdg_data_home.filter(|v| !v.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => PathBuf::from(home.unwrap_or("/tmp")).join(".local").join("share"),
    };
    base.join("simple-player").join("stats.db")
}

pub fn location_from_args(args: &[String], xdg_data_home: Option<&str>, home: Option<&str>) -> StatsLocation {
    if let Some(i) = args.iter().position(|a| a == "--stats-db") {
        if let Some(path) = args.get(i + 1) {
            return StatsLocation::File(PathBuf::from(path));
        }
    }
    let dev = ["--stats-demo", "--shot", "--bench", "--gallery"].iter().any(|f| args.iter().any(|a| a == f));
    if dev {
        StatsLocation::Memory
    } else {
        StatsLocation::File(default_db_path(xdg_data_home, home))
    }
}
```

Run: `cd native && cargo test stats::location` — Expected: PASS (5).

- [ ] **Step 4: Escribir las pruebas del servicio (fallan)**

Crear `native/src/stats/service.rs` con el esqueleto y las pruebas:

```rust
use super::location::StatsLocation;
use super::model::{PlayEvent, StatsRange, StatsSummary};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type Repaint = Arc<dyn Fn() + Send + Sync>;

#[derive(Clone)]
pub struct StatsHandle {
    _placeholder: Arc<Mutex<()>>,
}

impl StatsHandle {
    pub fn disabled(_reason: &str) -> Self {
        Self { _placeholder: Arc::new(Mutex::new(())) }
    }

    pub fn spawn(_location: StatsLocation, _repaint: Repaint) -> Self {
        Self::disabled("sin implementar")
    }

    pub fn record(&self, _event: PlayEvent) {}
    pub fn record_batch(&self, _events: Vec<PlayEvent>) {}
    pub fn request_summary(&self, _range: StatsRange) {}

    pub fn latest_summary(&self) -> Option<(StatsRange, Arc<StatsSummary>)> {
        None
    }

    pub fn events_version(&self) -> u64 {
        0
    }

    pub fn disabled_reason(&self) -> Option<String> {
        None
    }

    pub fn is_enabled(&self) -> bool {
        false
    }

    pub fn flush(&self, _timeout: Duration) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::model::SongSnapshot;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn event(path: &str) -> PlayEvent {
        let now = chrono::Local::now().timestamp_millis();
        PlayEvent {
            started_at: now - 70_000,
            ended_at: now - 10_000,
            listened_ms: 60_000,
            song: SongSnapshot { path: path.into(), title: "T".into(), artist: "A".into(), album: "B".into(), duration_ms: 200_000 },
        }
    }

    fn noop() -> Repaint {
        Arc::new(|| {})
    }

    #[test]
    fn un_evento_registrado_aparece_en_el_resumen_tras_el_flush() {
        let h = StatsHandle::spawn(StatsLocation::Memory, noop());
        assert!(h.is_enabled());
        h.record(event("a"));
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_secs(2));
        let (range, summary) = h.latest_summary().expect("hay resumen");
        assert_eq!(range, StatsRange::All);
        assert_eq!(summary.totals.plays, 1);
    }

    #[test]
    fn la_version_de_eventos_sube_con_cada_evento_y_no_con_los_resumenes() {
        let h = StatsHandle::spawn(StatsLocation::Memory, noop());
        assert_eq!(h.events_version(), 0);
        h.record(event("a"));
        h.record_batch(vec![event("b"), event("c")]);
        h.flush(Duration::from_secs(2));
        let after_events = h.events_version();
        assert!(after_events >= 2, "{after_events}");
        h.request_summary(StatsRange::Week);
        h.flush(Duration::from_secs(2));
        assert_eq!(h.events_version(), after_events, "un resumen no debe disparar otra petición");
    }

    #[test]
    fn el_servicio_despierta_la_ui_al_guardar_y_al_calcular() {
        let wakes = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&wakes);
        let h = StatsHandle::spawn(StatsLocation::Memory, Arc::new(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        }));
        h.record(event("a"));
        h.request_summary(StatsRange::Today);
        h.flush(Duration::from_secs(2));
        assert!(wakes.load(Ordering::SeqCst) >= 2);
    }

    #[test]
    fn una_ruta_imposible_deja_el_servicio_deshabilitado_sin_entrar_en_panico() {
        let h = StatsHandle::spawn(StatsLocation::File(PathBuf::from("/proc/no-existe/stats.db")), noop());
        assert!(!h.is_enabled());
        assert!(h.disabled_reason().is_some());
        h.record(event("a"));
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_millis(100));
        assert!(h.latest_summary().is_none());
    }

    #[test]
    fn un_handle_deshabilitado_expone_el_motivo() {
        let h = StatsHandle::disabled("prueba");
        assert_eq!(h.disabled_reason().as_deref(), Some("prueba"));
        assert!(!h.is_enabled());
    }

    #[test]
    fn los_datos_en_archivo_sobreviven_a_reabrir() {
        let path = std::env::temp_dir().join(format!("sp_service_{}.db", std::process::id()));
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", path.display()));
        }
        {
            let h = StatsHandle::spawn(StatsLocation::File(path.clone()), noop());
            h.record(event("a"));
            h.flush(Duration::from_secs(2));
        }
        let h = StatsHandle::spawn(StatsLocation::File(path.clone()), noop());
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_secs(2));
        assert_eq!(h.latest_summary().unwrap().1.totals.plays, 1);
        for ext in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{ext}", path.display()));
        }
    }
}
```

Run: `cd native && cargo test stats::service`
Expected: FAIL por aserción (el esqueleto no hace nada).

- [ ] **Step 5: Implementar el servicio**

Reemplazar el contenido de `native/src/stats/service.rs` por (conservando el bloque `tests` del Paso 4 al final):

```rust
use super::location::StatsLocation;
use super::model::{PlayEvent, StatsRange, StatsSummary};
use super::store::Store;
use super::summary::build_summary;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type Repaint = Arc<dyn Fn() + Send + Sync>;

enum Message {
    Record(PlayEvent),
    RecordBatch(Vec<PlayEvent>),
    Summary(StatsRange),
    Flush(Sender<()>),
}

#[derive(Default)]
struct Shared {
    latest: Mutex<Option<(StatsRange, Arc<StatsSummary>)>>,
    events_version: AtomicU64,
    disabled: Mutex<Option<String>>,
}

/// Puerta de entrada al servicio de estadísticas. Se clona barato; todas las
/// operaciones retornan sin esperar al disco.
#[derive(Clone)]
pub struct StatsHandle {
    tx: Option<Sender<Message>>,
    shared: Arc<Shared>,
}

impl StatsHandle {
    pub fn disabled(reason: &str) -> Self {
        let shared = Arc::new(Shared::default());
        *shared.disabled.lock().unwrap_or_else(|e| e.into_inner()) = Some(reason.to_string());
        Self { tx: None, shared }
    }

    pub fn spawn(location: StatsLocation, repaint: Repaint) -> Self {
        let opened = match &location {
            StatsLocation::File(path) => {
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                Store::open(path)
            }
            StatsLocation::Memory => Store::open_in_memory(),
        };
        let store = match opened {
            Ok(store) => store,
            Err(e) => {
                eprintln!("[aviso] Estadísticas desactivadas: {}", e.0);
                return Self::disabled(&format!("No se pudo abrir la base de estadísticas: {}", e.0));
            }
        };

        let shared = Arc::new(Shared::default());
        let (tx, rx) = mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        let spawned = std::thread::Builder::new()
            .name("stats".into())
            .spawn(move || worker(store, rx, worker_shared, repaint));
        if let Err(e) = spawned {
            return Self::disabled(&format!("No se pudo iniciar el hilo de estadísticas: {e}"));
        }
        Self { tx: Some(tx), shared }
    }

    fn send(&self, message: Message) {
        if let Some(tx) = &self.tx {
            let _ = tx.send(message);
        }
    }

    pub fn record(&self, event: PlayEvent) {
        self.send(Message::Record(event));
    }

    pub fn record_batch(&self, events: Vec<PlayEvent>) {
        self.send(Message::RecordBatch(events));
    }

    pub fn request_summary(&self, range: StatsRange) {
        self.send(Message::Summary(range));
    }

    pub fn latest_summary(&self) -> Option<(StatsRange, Arc<StatsSummary>)> {
        self.shared.latest.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Sube solo cuando se guarda un evento; la pantalla lo observa para volver a pedir el resumen.
    pub fn events_version(&self) -> u64 {
        self.shared.events_version.load(Ordering::SeqCst)
    }

    pub fn disabled_reason(&self) -> Option<String> {
        self.shared.disabled.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn is_enabled(&self) -> bool {
        self.tx.is_some()
    }

    /// Espera (hasta `timeout`) a que el hilo procese todo lo enviado antes.
    pub fn flush(&self, timeout: Duration) {
        let Some(tx) = &self.tx else { return };
        let (done_tx, done_rx) = mpsc::channel();
        if tx.send(Message::Flush(done_tx)).is_ok() {
            let _ = done_rx.recv_timeout(timeout);
        }
    }
}

fn worker(mut store: Store, rx: Receiver<Message>, shared: Arc<Shared>, repaint: Repaint) {
    while let Ok(message) = rx.recv() {
        match message {
            Message::Record(event) => match store.insert(&event) {
                Ok(()) => {
                    shared.events_version.fetch_add(1, Ordering::SeqCst);
                    repaint();
                }
                Err(e) => eprintln!("[stats] no se pudo guardar un evento: {}", e.0),
            },
            Message::RecordBatch(events) => match store.insert_batch(&events) {
                Ok(()) => {
                    shared.events_version.fetch_add(1, Ordering::SeqCst);
                    repaint();
                }
                Err(e) => eprintln!("[stats] no se pudo guardar el lote: {}", e.0),
            },
            Message::Summary(range) => match build_summary(&store, range, &chrono::Local::now()) {
                Ok(summary) => {
                    *shared.latest.lock().unwrap_or_else(|e| e.into_inner()) = Some((range, Arc::new(summary)));
                    repaint();
                }
                Err(e) => eprintln!("[stats] no se pudo calcular el resumen: {}", e.0),
            },
            Message::Flush(done) => {
                let _ = done.send(());
            }
        }
    }
}
```

- [ ] **Step 6: Ejecutar para verificar que pasa**

Run: `cd native && cargo test stats::service stats::location`
Expected: PASS (6 + 5 pruebas).

- [ ] **Step 7: Commit**

```bash
git add native/src/stats/location.rs native/src/stats/service.rs native/src/stats/mod.rs
git commit -m "Estadísticas: servicio de fondo (StatsHandle) y ubicación de la base"
```

---

### Task 7: `StatsRecorder` y datos de demostración

**Files:**
- Create: `native/src/stats/recorder.rs`, `native/src/stats/demo.rs`
- Modify: `native/src/stats/mod.rs`

**Interfaces:**
- Consumes: `session::SessionTracker`, `service::StatsHandle`, `model::{SongSnapshot, PlayEvent}`.
- Produces:
  - `recorder::StatsRecorder::new(handle: StatsHandle) -> Self`; `handle(&self) -> &StatsHandle`.
  - `StatsRecorder::song_started(&mut self, song: SongSnapshot, playing: bool)`, `observe(&mut self, playing: bool)`, `finish(&mut self)` (reloj real).
  - Variantes con reloj inyectado, usadas por las pruebas: `song_started_at(&mut self, song, playing, now: Instant, epoch_ms: i64)`, `observe_at(&mut self, playing, now, epoch_ms)`, `finish_at(&mut self, now, epoch_ms)`.
  - `recorder::epoch_ms_now() -> i64`.
  - `demo::demo_events(now_ms: i64, songs: &[SongSnapshot]) -> Vec<PlayEvent>` (ordenado por `ended_at`; con `songs` vacío usa 12 canciones sintéticas).

- [ ] **Step 1: Registrar y escribir las pruebas (fallan)**

En `native/src/stats/mod.rs` agregar `pub mod demo;` y `pub mod recorder;`.

Crear `native/src/stats/recorder.rs`:

```rust
use super::model::SongSnapshot;
use super::service::StatsHandle;
use super::session::SessionTracker;
use std::time::Instant;

pub fn epoch_ms_now() -> i64 {
    0
}

/// Une el tracker de sesión con el servicio: lo que `AppState` llama.
pub struct StatsRecorder {
    tracker: SessionTracker,
    handle: StatsHandle,
}

impl StatsRecorder {
    pub fn new(handle: StatsHandle) -> Self {
        Self { tracker: SessionTracker::new(), handle }
    }

    pub fn handle(&self) -> &StatsHandle {
        &self.handle
    }

    pub fn song_started(&mut self, _song: SongSnapshot, _playing: bool) {}
    pub fn observe(&mut self, _playing: bool) {}
    pub fn finish(&mut self) {}

    pub fn song_started_at(&mut self, _song: SongSnapshot, _playing: bool, _now: Instant, _epoch_ms: i64) {}
    pub fn observe_at(&mut self, _playing: bool, _now: Instant, _epoch_ms: i64) {}
    pub fn finish_at(&mut self, _now: Instant, _epoch_ms: i64) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::location::StatsLocation;
    use crate::stats::model::StatsRange;
    use std::sync::Arc;
    use std::time::Duration;

    fn song(path: &str) -> SongSnapshot {
        SongSnapshot { path: path.into(), title: path.into(), artist: "A".into(), album: "B".into(), duration_ms: 200_000 }
    }

    fn recorder() -> StatsRecorder {
        StatsRecorder::new(StatsHandle::spawn(StatsLocation::Memory, Arc::new(|| {})))
    }

    fn plays(r: &StatsRecorder) -> u32 {
        r.handle().request_summary(StatsRange::All);
        r.handle().flush(Duration::from_secs(2));
        r.handle().latest_summary().map(|(_, s)| s.totals.plays).unwrap_or(0)
    }

    const E0: i64 = 1_700_000_000_000;

    #[test]
    fn cambiar_de_cancion_guarda_la_anterior_si_paso_de_5_segundos() {
        let t0 = Instant::now();
        let mut r = recorder();
        r.song_started_at(song("a"), true, t0, E0);
        r.song_started_at(song("b"), true, t0 + Duration::from_secs(30), E0 + 30_000);
        assert_eq!(plays(&r), 1);
    }

    #[test]
    fn una_cancion_saltada_antes_de_5_segundos_no_se_guarda() {
        let t0 = Instant::now();
        let mut r = recorder();
        r.song_started_at(song("a"), true, t0, E0);
        r.song_started_at(song("b"), true, t0 + Duration::from_secs(3), E0 + 3_000);
        assert_eq!(plays(&r), 0);
    }

    #[test]
    fn cerrar_la_app_guarda_la_cancion_en_curso() {
        let t0 = Instant::now();
        let mut r = recorder();
        r.song_started_at(song("a"), true, t0, E0);
        r.observe_at(true, t0 + Duration::from_secs(20), E0 + 20_000);
        r.finish_at(t0 + Duration::from_secs(20), E0 + 20_000);
        assert_eq!(plays(&r), 1);
    }

    #[test]
    fn un_recorder_con_el_servicio_deshabilitado_no_falla() {
        let t0 = Instant::now();
        let mut r = StatsRecorder::new(StatsHandle::disabled("prueba"));
        r.song_started_at(song("a"), true, t0, E0);
        r.finish_at(t0 + Duration::from_secs(30), E0 + 30_000);
        assert_eq!(plays(&r), 0);
    }

    #[test]
    fn el_reloj_real_devuelve_una_fecha_razonable() {
        assert!(epoch_ms_now() > 1_700_000_000_000);
    }
}
```

Crear `native/src/stats/demo.rs`:

```rust
use super::model::{PlayEvent, SongSnapshot};

pub fn demo_events(_now_ms: i64, _songs: &[SongSnapshot]) -> Vec<PlayEvent> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_790_000_000_000;
    const DAY: i64 = 86_400_000;

    #[test]
    fn genera_un_mes_de_escucha_valida_y_ordenada() {
        let events = demo_events(NOW, &[]);
        assert!(events.len() > 100, "{}", events.len());
        assert!(events.iter().all(|e| e.ended_at <= NOW && e.listened_ms >= 5_000 && e.started_at <= e.ended_at));
        assert!(events.windows(2).all(|w| w[0].ended_at <= w[1].ended_at));
        let days: std::collections::BTreeSet<i64> = events.iter().map(|e| e.ended_at / DAY).collect();
        assert!(days.len() >= 20, "{}", days.len());
    }

    #[test]
    fn es_determinista_y_usa_las_canciones_dadas() {
        let songs = vec![SongSnapshot {
            path: "/m/x.mp3".into(),
            title: "X".into(),
            artist: "Y".into(),
            album: "Z".into(),
            duration_ms: 240_000,
        }];
        let a = demo_events(NOW, &songs);
        let b = demo_events(NOW, &songs);
        assert_eq!(a, b);
        assert!(a.iter().all(|e| e.song.path == "/m/x.mp3"));
    }
}
```

Run: `cd native && cargo test stats::recorder stats::demo`
Expected: FAIL por aserción (esqueletos vacíos; `epoch_ms_now` devuelve 0).

- [ ] **Step 2: Implementar `recorder`**

Reemplazar `epoch_ms_now` y los métodos del `impl StatsRecorder` por:

```rust
pub fn epoch_ms_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
```

```rust
    pub fn song_started(&mut self, song: SongSnapshot, playing: bool) {
        self.song_started_at(song, playing, Instant::now(), epoch_ms_now());
    }

    pub fn observe(&mut self, playing: bool) {
        self.observe_at(playing, Instant::now(), epoch_ms_now());
    }

    pub fn finish(&mut self) {
        self.finish_at(Instant::now(), epoch_ms_now());
    }

    pub fn song_started_at(&mut self, song: SongSnapshot, playing: bool, now: Instant, epoch_ms: i64) {
        if let Some(event) = self.tracker.start(song, now, epoch_ms, playing) {
            self.handle.record(event);
        }
    }

    pub fn observe_at(&mut self, playing: bool, now: Instant, epoch_ms: i64) {
        self.tracker.observe(playing, now, epoch_ms);
    }

    pub fn finish_at(&mut self, now: Instant, epoch_ms: i64) {
        if let Some(event) = self.tracker.finish(now, epoch_ms) {
            self.handle.record(event);
        }
    }
```

- [ ] **Step 3: Implementar `demo`**

Reemplazar `demo_events` por:

```rust
pub fn demo_events(now_ms: i64, songs: &[SongSnapshot]) -> Vec<PlayEvent> {
    const DAY: i64 = 86_400_000;
    const HOUR: i64 = 3_600_000;

    let fallback: Vec<SongSnapshot>;
    let pool: &[SongSnapshot] = if songs.is_empty() {
        fallback = (0..12u64)
            .map(|i| SongSnapshot {
                path: format!("/demo/{i}.mp3"),
                title: format!("Canción {}", i + 1),
                artist: format!("Artista {}", i % 4 + 1),
                album: format!("Álbum {}", i % 3 + 1),
                duration_ms: 180_000 + i * 7_000,
            })
            .collect();
        &fallback
    } else {
        songs
    };

    // xorshift con semilla fija: la demo es idéntica en cada corrida.
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut events = Vec::new();
    for day in 0..30i64 {
        if day != 0 && day % 6 == 5 {
            continue; // algunos días sin escuchar, para que haya rachas rotas
        }
        let sessions = 1 + (next() % 3) as i64;
        for s in 0..sessions {
            let back = (s + 1) * 4 * HOUR + (next() % 1_800_000) as i64;
            let mut cursor = now_ms - day * DAY - back;
            for _ in 0..(3 + next() % 7) {
                let r = (next() % 1000) as f64 / 1000.0;
                let song = &pool[(((r * r) * pool.len() as f64) as usize).min(pool.len() - 1)]; // sesga hacia las primeras
                let full = if song.duration_ms > 0 { song.duration_ms } else { 200_000 };
                let listened = full * (60 + next() % 41) / 100;
                let ended = cursor + listened as i64;
                if ended > now_ms {
                    break;
                }
                events.push(PlayEvent { started_at: cursor, ended_at: ended, listened_ms: listened, song: song.clone() });
                cursor = ended + (next() % 20_000) as i64;
            }
        }
    }
    events.sort_by_key(|e| e.ended_at);
    events
}
```

- [ ] **Step 4: Ejecutar para verificar que pasa**

Run: `cd native && cargo test stats::`
Expected: PASS (todas las de `stats`, incluidas 5 de `recorder` y 2 de `demo`).

- [ ] **Step 5: Commit**

```bash
git add native/src/stats/recorder.rs native/src/stats/demo.rs native/src/stats/mod.rs
git commit -m "Estadísticas: StatsRecorder y eventos de demostración"
```

---

### Task 8: Integración en `AppState` y `main.rs`

**Files:**
- Modify: `native/src/state.rs`, `native/src/main.rs`

**Interfaces:**
- Consumes: `stats::recorder::StatsRecorder`, `stats::service::{StatsHandle, Repaint}`, `stats::location::location_from_args`, `stats::demo::demo_events`, `stats::model::{SongSnapshot, StatsRange}`.
- Produces (usados por la Tarea 10): campos públicos `AppState.stats: StatsRecorder`, `AppState.stats_range: StatsRange` (por defecto `Week`), `AppState.stats_requested: Option<(StatsRange, u64)>` (por defecto `None`).

- [ ] **Step 1: Campos y enganche en `AppState`**

En `native/src/state.rs`, agregar a los `use` del inicio:

```rust
use crate::stats::model::{SongSnapshot, StatsRange};
use crate::stats::recorder::StatsRecorder;
use crate::stats::service::StatsHandle;
```

En `pub struct AppState`, junto a `pub focus_search: bool,`, agregar:

```rust
    /// Registro de estadísticas de escucha (deshabilitado hasta que `main` lo inicie).
    pub stats: StatsRecorder,
    pub stats_range: StatsRange,
    /// Última (rango, versión de eventos) para la que la pantalla pidió un resumen.
    pub stats_requested: Option<(StatsRange, u64)>,
```

En `AppState::new`, junto a `focus_search: false,`:

```rust
            stats: StatsRecorder::new(StatsHandle::disabled("Las estadísticas todavía no se iniciaron")),
            stats_range: StatsRange::Week,
            stats_requested: None,
```

En `play_index`, justo después de `let song = queue[valid_index as usize].clone();`:

```rust
        self.stats.song_started(SongSnapshot::from(&song), true);
```

- [ ] **Step 2: Verificar que compila y las pruebas siguen verdes**

Run: `cd native && cargo test`
Expected: PASS (todas las pruebas existentes más las de `stats`).

- [ ] **Step 3: Cableado en `main.rs`**

En `native/src/main.rs`:

1. Quitar el `#[allow(dead_code)]` de `mod stats;` (queda `mod stats;`).

2. En `App::new`, después de `state.init();` y de procesar los flags `--song`/`--time`/etc. (antes de construir `Self { ... }`), agregar:

```rust
        // Estadísticas: base real en uso normal; memoria en corridas de desarrollo.
        let args: Vec<String> = std::env::args().collect();
        let location = stats::location::location_from_args(
            &args,
            std::env::var("XDG_DATA_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        );
        let repaint_ctx = cc.egui_ctx.clone();
        let stats_handle = stats::service::StatsHandle::spawn(location, std::sync::Arc::new(move || repaint_ctx.request_repaint()));
        if args.iter().any(|a| a == "--stats-demo") {
            let snapshots: Vec<stats::model::SongSnapshot> = state.songs.iter().take(60).map(stats::model::SongSnapshot::from).collect();
            stats_handle.record_batch(stats::demo::demo_events(stats::recorder::epoch_ms_now(), &snapshots));
        }
        state.stats = stats::recorder::StatsRecorder::new(stats_handle);
```

3. En `draw()`, justo después de `self.state.tick();` y su medición (`self.perf.record_tick(...)`), agregar:

```rust
        self.state.stats.observe(self.state.is_playing);
```

4. En `impl eframe::App for App`, agregar el método (después de `update`):

```rust
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.state.stats.finish();
        self.state.stats.handle().flush(std::time::Duration::from_secs(1));
    }
```

- [ ] **Step 4: Verificar compilación y comportamiento**

Run: `cd native && cargo build --release 2>&1 | tail -3`
Expected: `Finished`, sin avisos nuevos de código muerto en `stats` (los métodos aún sin uso de UI, como `latest_summary`, pueden avisar hasta la Tarea 10; es aceptable y se resuelve allí).

Run (arranque de humo, sin tocar la base real): `cd native && timeout 8 ./target/release/simple-player --allow-multiple --no-hotkeys --stats-demo 2>&1 | tail -3`
Expected: sin pánicos.

Run: `ls ~/.local/share/simple-player/ 2>&1 | head -2`
Expected: `No such file or directory` (las corridas de desarrollo no crean la base real).

- [ ] **Step 5: Commit**

```bash
git add native/src/state.rs native/src/main.rs
git commit -m "Estadísticas: enganche en play_index, observe por frame y cierre ordenado"
```

---

### Task 9: Gráfico de barras (`bar_chart`)

**Files:**
- Create: `native/src/ui/widgets/bar_chart.rs`
- Modify: `native/src/ui/widgets/mod.rs`

**Interfaces:**
- Consumes: `stats::model::TimelineBucket`, `theme`, `stats::format::format_duration`.
- Produces:
  - `bar_chart::bar_rects(rect: egui::Rect, values: &[u64], gap: f32) -> Vec<egui::Rect>`.
  - `bar_chart::bucket_at(rects: &[egui::Rect], x: f32) -> Option<usize>`.
  - `bar_chart::label_step(count: usize) -> usize`.
  - `bar_chart::bar_chart(ui: &mut egui::Ui, buckets: &[TimelineBucket], peak: Option<usize>, height: f32)`.

- [ ] **Step 1: Registrar y escribir las pruebas de geometría (fallan)**

En `native/src/ui/widgets/mod.rs` agregar `pub mod bar_chart;` (orden alfabético: antes de `card`).

Crear `native/src/ui/widgets/bar_chart.rs`:

```rust
use crate::stats::model::TimelineBucket;
use eframe::egui;

/// Una barra mínima visible para valores pequeños pero mayores que cero.
const MIN_BAR_HEIGHT: f32 = 3.0;

/// Rectángulos de las barras dentro de `rect`; la barra más alta ocupa toda la altura.
pub fn bar_rects(_rect: egui::Rect, _values: &[u64], _gap: f32) -> Vec<egui::Rect> {
    Vec::new()
}

/// Barra bajo el puntero (por su columna, sin importar la altura), incluyendo medio hueco a cada lado.
pub fn bucket_at(_rects: &[egui::Rect], _x: f32) -> Option<usize> {
    None
}

/// Cada cuántas barras se dibuja una etiqueta para que no se encimen.
pub fn label_step(_count: usize) -> usize {
    1
}

pub fn bar_chart(_ui: &mut egui::Ui, _buckets: &[TimelineBucket], _peak: Option<usize>, _height: f32) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(300.0, 100.0))
    }

    #[test]
    fn la_barra_mas_alta_ocupa_toda_la_altura_y_las_demas_son_proporcionales() {
        let rects = bar_rects(area(), &[50, 100, 25], 4.0);
        assert_eq!(rects.len(), 3);
        assert!((rects[1].height() - 100.0).abs() < 1e-3);
        assert!((rects[0].height() - 50.0).abs() < 1e-3);
        assert!((rects[2].height() - 25.0).abs() < 1e-3);
        assert!(rects.iter().all(|r| (r.bottom() - 120.0).abs() < 1e-3), "todas apoyadas en la base");
    }

    #[test]
    fn un_valor_cero_no_tiene_altura_y_uno_diminuto_se_ve() {
        let rects = bar_rects(area(), &[0, 1, 1_000_000], 4.0);
        assert_eq!(rects[0].height(), 0.0);
        assert!(rects[1].height() >= 3.0, "{}", rects[1].height());
    }

    #[test]
    fn todo_en_cero_no_produce_nan_ni_alturas() {
        let rects = bar_rects(area(), &[0, 0, 0], 4.0);
        assert!(rects.iter().all(|r| r.height() == 0.0 && r.width().is_finite()));
    }

    #[test]
    fn las_barras_con_sus_huecos_caben_exactamente_en_el_ancho() {
        let rects = bar_rects(area(), &[1, 2, 3, 4], 6.0);
        assert!((rects.first().unwrap().left() - 10.0).abs() < 1e-3);
        assert!((rects.last().unwrap().right() - 310.0).abs() < 1e-3);
        assert!(rects.windows(2).all(|w| (w[1].left() - w[0].right() - 6.0).abs() < 1e-3));
    }

    #[test]
    fn sin_valores_no_hay_barras() {
        assert!(bar_rects(area(), &[], 4.0).is_empty());
    }

    #[test]
    fn el_puntero_encuentra_la_barra_de_su_columna() {
        let rects = bar_rects(area(), &[1, 2, 3], 6.0);
        assert_eq!(bucket_at(&rects, rects[0].center().x), Some(0));
        assert_eq!(bucket_at(&rects, rects[2].center().x), Some(2));
        // en el hueco entre dos barras se elige la más cercana
        let gap_x = (rects[0].right() + rects[1].left()) / 2.0;
        assert!(bucket_at(&rects, gap_x).is_some());
        assert_eq!(bucket_at(&rects, 0.0), None);
        assert_eq!(bucket_at(&rects, 999.0), None);
        assert_eq!(bucket_at(&[], 50.0), None);
    }

    #[test]
    fn las_etiquetas_se_espacian_segun_la_cantidad_de_barras() {
        assert_eq!(label_step(7), 1);
        assert_eq!(label_step(12), 1);
        assert_eq!(label_step(24), 3);
        assert_eq!(label_step(31), 5);
    }
}
```

Run: `cd native && cargo test bar_chart`
Expected: FAIL por aserción.

- [ ] **Step 2: Implementar la geometría**

Reemplazar las tres funciones puras:

```rust
pub fn bar_rects(rect: egui::Rect, values: &[u64], gap: f32) -> Vec<egui::Rect> {
    let n = values.len();
    if n == 0 {
        return Vec::new();
    }
    let max = values.iter().copied().max().unwrap_or(0);
    let slot = (rect.width() - gap * (n as f32 - 1.0)) / n as f32;
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let mut height = if max == 0 { 0.0 } else { *v as f32 / max as f32 * rect.height() };
            if *v > 0 && height < MIN_BAR_HEIGHT {
                height = MIN_BAR_HEIGHT;
            }
            let x = rect.left() + i as f32 * (slot + gap);
            egui::Rect::from_min_size(egui::pos2(x, rect.bottom() - height), egui::vec2(slot, height))
        })
        .collect()
}

pub fn bucket_at(rects: &[egui::Rect], x: f32) -> Option<usize> {
    let first = rects.first()?;
    let last = rects.last()?;
    let half_gap = if rects.len() > 1 { (rects[1].left() - rects[0].right()) / 2.0 } else { 0.0 };
    if x < first.left() - half_gap || x > last.right() + half_gap {
        return None;
    }
    rects
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (a.center().x - x).abs().total_cmp(&(b.center().x - x).abs()))
        .map(|(i, _)| i)
}

pub fn label_step(count: usize) -> usize {
    match count {
        0..=12 => 1,
        13..=24 => 3,
        _ => 5,
    }
}
```

Run: `cd native && cargo test bar_chart` — Expected: PASS (7).

- [ ] **Step 3: Implementar el dibujo**

Reemplazar `bar_chart` y agregar los imports (`use crate::stats::format::format_duration; use crate::theme; use super::gradient::...` no hace falta):

```rust
pub fn bar_chart(ui: &mut egui::Ui, buckets: &[TimelineBucket], peak: Option<usize>, height: f32) {
    const LABEL_H: f32 = 22.0;
    let accent = theme::accent(ui.ctx());
    let (outer, response) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height + LABEL_H), egui::Sense::hover());
    if !ui.is_rect_visible(outer) || buckets.is_empty() {
        return;
    }
    let plot = egui::Rect::from_min_size(outer.min, egui::vec2(outer.width(), height));
    let values: Vec<u64> = buckets.iter().map(|b| b.listened_ms).collect();
    let gap = if buckets.len() > 24 { 3.0 } else { 6.0 };
    let rects = bar_rects(plot, &values, gap);
    let hovered = response.hover_pos().and_then(|p| bucket_at(&rects, p.x));
    let painter = ui.painter();

    painter.hline(plot.x_range(), plot.bottom(), egui::Stroke::new(1.0_f32, theme::GLASS_BORDER));
    let step = label_step(buckets.len());
    for (i, (rect, bucket)) in rects.iter().zip(buckets).enumerate() {
        let is_peak = peak == Some(i);
        let lit = is_peak || hovered == Some(i);
        let bottom_color = if lit { theme::with_alpha(accent, 230) } else { theme::with_alpha(accent, 120) };
        let top_color = if lit { theme::lerp_color(accent, egui::Color32::WHITE, 0.25) } else { theme::with_alpha(accent, 170) };
        if rect.height() > 0.0 {
            let mut mesh = egui::Mesh::default();
            let v = |pos: egui::Pos2, color| egui::epaint::Vertex { pos, uv: egui::epaint::WHITE_UV, color };
            mesh.vertices.extend([
                v(rect.left_top(), top_color),
                v(rect.right_top(), top_color),
                v(rect.right_bottom(), bottom_color),
                v(rect.left_bottom(), bottom_color),
            ]);
            mesh.indices.extend([0, 1, 2, 0, 2, 3]);
            painter.add(egui::Shape::mesh(mesh));
        }
        if i % step == 0 {
            let color = if lit { theme::TEXT_MAIN } else { theme::TEXT_MUTED };
            painter.text(
                egui::pos2(rect.center().x, plot.bottom() + LABEL_H / 2.0 + 2.0),
                egui::Align2::CENTER_CENTER,
                &bucket.label,
                egui::FontId::proportional(theme::text::XS),
                color,
            );
        }
    }

    if let Some(i) = hovered {
        let text = format!("{} · {}", buckets[i].label, format_duration(buckets[i].listened_ms));
        response.on_hover_text_at_pointer(text);
    }
}
```

- [ ] **Step 4: Verificar compilación**

Run: `cd native && cargo test bar_chart && cargo build 2>&1 | grep -E "^error" -A8 | head -20`
Expected: PASS y sin errores.

- [ ] **Step 5: Commit**

```bash
git add native/src/ui/widgets/bar_chart.rs native/src/ui/widgets/mod.rs
git commit -m "Estadísticas: widget de gráfico de barras con tooltip"
```

---

### Task 10: Pantalla "Estadísticas"

**Files:**
- Create: `native/src/ui/screens/stats.rs`
- Modify: `native/src/state.rs` (variante `ActiveTab::Stats`), `native/src/theme/icons.rs`, `native/src/ui/screens/mod.rs`, `native/src/ui/shell/sidebar.rs`, `native/src/ui/shell/topbar.rs`, `native/src/main.rs` (flags `--tab stats`, `--stats-range`)

**Interfaces:**
- Consumes: `AppState::{stats, stats_range, stats_requested, songs, select_tab}`, `StatsHandle`, `StatsSummary`, `format::{format_duration, unknown_if_empty}`, widgets `chip`, `glass_panel`, `empty_state`, `bar_chart`, `paint_cover`, `skeleton`.
- Produces: `screens::stats::show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache)`.

- [ ] **Step 1: Pestaña, ícono y despacho**

En `native/src/state.rs`, en `pub enum ActiveTab`, agregar la variante `Stats` después de `Artists`:

```rust
pub enum ActiveTab {
    All,
    Albums,
    Artists,
    Stats,
}
```

En `native/src/theme/icons.rs`, agregar `CHART_BAR` a la lista del `pub use` (orden alfabético, antes de `DISC`):

```rust
pub use egui_phosphor::regular::{
    ARROWS_OUT_SIMPLE, CARET_DOWN, CARET_LEFT, CARET_RIGHT, CARET_UP, CHART_BAR, DISC, FOLDER_OPEN, GEAR, HEART,
    LIST_BULLETS, MAGNIFYING_GLASS, MICROPHONE_STAGE, MUSIC_NOTES, PAUSE, PLAY, QUEUE, REPEAT, REPEAT_ONCE, SHUFFLE,
    SKIP_BACK, SKIP_FORWARD, SPEAKER_HIGH, SPEAKER_LOW, SPEAKER_X, TEXT_ALIGN_LEFT, USER, WAVEFORM, X,
};
```

En `native/src/ui/screens/mod.rs`, agregar `mod stats;` junto a los demás y el brazo del `match`:

```rust
        ActiveTab::Stats => stats::show(ui, state, textures),
```

(colocarlo antes del `_ =>`).

En `native/src/ui/shell/sidebar.rs`, en el arreglo de navegación, agregar tras Artistas:

```rust
        (icons::CHART_BAR, "Estadísticas", ActiveTab::Stats),
```

En `native/src/ui/shell/topbar.rs`, al inicio de `pub fn show(ui, state)`, agregar:

```rust
    if state.active_tab == crate::state::ActiveTab::Stats {
        ui.add_space(8.0); // el buscador no aplica en Estadísticas
        return;
    }
```

En `native/src/main.rs`, en el `match tab.as_str()` de `--tab`, agregar `"stats" => state.select_tab(ActiveTab::Stats),`; y debajo, el flag de rango:

```rust
        if let Some(range) = ui::gallery::arg_value("--stats-range") {
            state.stats_range = match range.as_str() {
                "today" => stats::model::StatsRange::Today,
                "month" => stats::model::StatsRange::Month,
                "year" => stats::model::StatsRange::Year,
                "all" => stats::model::StatsRange::All,
                _ => stats::model::StatsRange::Week,
            };
        }
```

- [ ] **Step 2: Escribir la pantalla**

Crear `native/src/ui/screens/stats.rs`:

```rust
use crate::library::Song;
use crate::state::{ActiveTab, AppState};
use crate::stats::format::{format_duration, unknown_if_empty};
use crate::stats::model::{StatsRange, StatsSummary, TopEntry};
use crate::theme::{self, icons, radius, space, text};
use crate::ui::textures::TextureCache;
use crate::ui::widgets::bar_chart::bar_chart;
use crate::ui::widgets::chip::chip;
use crate::ui::widgets::cover::{paint_cover, skeleton};
use crate::ui::widgets::empty_state::empty_state;
use crate::ui::widgets::glass::{glass_panel, GlassKind};
use eframe::egui::{self, RichText};

const GAP: f32 = 16.0;
const ROW_H: f32 = 52.0;

pub fn show(ui: &mut egui::Ui, state: &mut AppState, textures: &mut TextureCache) {
    header(ui, state);

    let handle = state.stats.handle().clone();
    if let Some(reason) = handle.disabled_reason() {
        empty_state(ui, icons::CHART_BAR, "Estadísticas no disponibles", &reason, None);
        return;
    }

    // Se vuelve a pedir el resumen solo si cambió el rango o llegó un evento nuevo.
    let key = (state.stats_range, handle.events_version());
    if state.stats_requested != Some(key) {
        handle.request_summary(state.stats_range);
        state.stats_requested = Some(key);
    }

    let range = state.stats_range;
    let Some(summary) = handle.latest_summary().filter(|(r, _)| *r == range).map(|(_, s)| s) else {
        loading(ui);
        return;
    };

    if summary.totals.plays == 0 {
        let go = empty_state(
            ui,
            icons::CHART_BAR,
            "Sin reproducciones en este rango",
            "Reproduce una canción más de 5 segundos para empezar.",
            Some("Ir a Canciones"),
        );
        if go {
            state.select_tab(ActiveTab::All);
        }
        return;
    }

    let songs = &state.songs;
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        tiles(ui, &summary);
        ui.add_space(GAP);
        timeline_card(ui, &summary);
        ui.add_space(GAP);
        habits_card(ui, &summary);
        ui.add_space(GAP);
        tops(ui, &summary, songs, textures);
        ui.add_space(GAP);
    });
}

fn header(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("ESTADÍSTICAS").font(theme::deco(text::XL + 4.0)).color(theme::accent(ui.ctx())));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            for range in StatsRange::ALL_RANGES.iter().rev() {
                if chip(ui, range.label(), state.stats_range == *range).clicked() {
                    state.stats_range = *range;
                }
            }
        });
    });
    ui.add_space(space::LG);
}

fn loading(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        for _ in 0..4 {
            skeleton(ui, 96.0, radius::MD);
        }
    });
}

fn tiles(ui: &mut egui::Ui, summary: &StatsSummary) {
    let width = ((ui.available_width() - 3.0 * GAP) / 4.0).max(140.0);
    let items = [
        ("Tiempo total", format_duration(summary.totals.listened_ms)),
        ("Reproducciones", summary.totals.plays.to_string()),
        ("Canciones únicas", summary.totals.unique_songs.to_string()),
        ("Promedio diario", format_duration(summary.avg_daily_ms)),
    ];
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        for (label, value) in items {
            glass_panel(ui, GlassKind::Standard, radius::LG, space::LG, |ui| {
                ui.set_width(width - 2.0 * space::LG);
                ui.label(RichText::new(label).size(text::SM).color(theme::TEXT_MUTED));
                ui.label(RichText::new(value).font(theme::bold(text::XL + 4.0)));
            });
        }
    });
}

fn timeline_card(ui: &mut egui::Ui, summary: &StatsSummary) {
    glass_panel(ui, GlassKind::Standard, radius::LG, space::XL, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("LÍNEA DE TIEMPO").font(theme::deco(text::LG)).color(theme::accent(ui.ctx())));
        if let Some(peak) = summary.peak_bucket.and_then(|i| summary.timeline.get(i)) {
            ui.label(
                RichText::new(format!("Pico: {} · {}", peak.label, format_duration(peak.listened_ms)))
                    .size(text::SM)
                    .color(theme::TEXT_MUTED),
            );
        }
        ui.add_space(space::MD);
        bar_chart(ui, &summary.timeline, summary.peak_bucket, 180.0);
    });
}

fn habits_card(ui: &mut egui::Ui, summary: &StatsSummary) {
    let h = &summary.habits;
    let peak_day = h.peak_weekday.as_ref().map(|(day, ms)| format!("{day} ({})", format_duration(*ms)));
    let metrics = [
        ("Días activos", h.active_days.to_string()),
        ("Racha actual", format!("{} d", h.current_streak_days)),
        ("Racha más larga", format!("{} d", h.longest_streak_days)),
        ("Sesiones", h.sessions.to_string()),
        ("Sesión promedio", format_duration(h.avg_session_ms)),
        ("Sesión más larga", format_duration(h.longest_session_ms)),
        ("Sesiones por día", format!("{:.1}", h.sessions_per_day)),
        ("Día pico", peak_day.unwrap_or_else(|| "—".to_string())),
    ];
    glass_panel(ui, GlassKind::Standard, radius::LG, space::XL, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new("HÁBITOS").font(theme::deco(text::LG)).color(theme::accent(ui.ctx())));
        ui.add_space(space::MD);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(space::XXL, space::LG);
            for (label, value) in metrics {
                ui.vertical(|ui| {
                    ui.label(RichText::new(label).size(text::SM).color(theme::TEXT_MUTED));
                    ui.label(RichText::new(value).font(theme::bold(text::MD)));
                });
            }
        });
    });
}

fn tops(ui: &mut egui::Ui, summary: &StatsSummary, songs: &[Song], textures: &mut TextureCache) {
    ui.columns(3, |cols| {
        top_card(&mut cols[0], "CANCIONES", &summary.top_songs, songs, textures, icons::MUSIC_NOTES, "Sin título", false);
        top_card(&mut cols[1], "ARTISTAS", &summary.top_artists, songs, textures, icons::USER, "Artista desconocido", true);
        top_card(&mut cols[2], "ÁLBUMES", &summary.top_albums, songs, textures, icons::DISC, "Álbum desconocido", false);
    });
}

#[allow(clippy::too_many_arguments)]
fn top_card(
    ui: &mut egui::Ui,
    title: &str,
    entries: &[TopEntry],
    songs: &[Song],
    textures: &mut TextureCache,
    fallback_icon: &str,
    unknown: &str,
    round_cover: bool,
) {
    glass_panel(ui, GlassKind::Standard, radius::LG, space::LG, |ui| {
        ui.set_width(ui.available_width());
        ui.label(RichText::new(title).font(theme::deco(text::LG)).color(theme::accent(ui.ctx())));
        ui.add_space(space::SM);
        let max = entries.first().map(|e| e.listened_ms).unwrap_or(1).max(1);
        for (i, entry) in entries.iter().enumerate() {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), ROW_H), egui::Sense::hover());
            let cover_rect = egui::Rect::from_center_size(egui::pos2(rect.left() + 38.0, rect.center().y - 3.0), egui::vec2(36.0, 36.0));
            let cover = songs.iter().find(|s| s.path == entry.reference_path).and_then(|s| s.cover_art.clone());
            paint_cover(ui, cover_rect, textures, &cover, if round_cover { 18.0 } else { 6.0 }, fallback_icon, false);

            let painter = ui.painter();
            painter.text(
                egui::pos2(rect.left() + 8.0, rect.center().y - 3.0),
                egui::Align2::LEFT_CENTER,
                (i + 1).to_string(),
                theme::bold(text::SM),
                theme::TEXT_MUTED,
            );
            let text_left = cover_rect.right() + 10.0;
            let clip = painter.with_clip_rect(egui::Rect::from_min_max(
                egui::pos2(text_left, rect.top()),
                egui::pos2(rect.right() - 70.0, rect.bottom()),
            ));
            clip.text(
                egui::pos2(text_left, rect.center().y - 12.0),
                egui::Align2::LEFT_CENTER,
                unknown_if_empty(&entry.name, unknown),
                theme::bold(text::BASE),
                theme::TEXT_MAIN,
            );
            let detail = if entry.detail.is_empty() { format!("{} rep.", entry.plays) } else { format!("{} · {} rep.", unknown_if_empty(&entry.detail, "Artista desconocido"), entry.plays) };
            clip.text(
                egui::pos2(text_left, rect.center().y + 4.0),
                egui::Align2::LEFT_CENTER,
                detail,
                egui::FontId::proportional(text::XS),
                theme::TEXT_MUTED,
            );
            painter.text(
                egui::pos2(rect.right() - 6.0, rect.center().y - 12.0),
                egui::Align2::RIGHT_CENTER,
                format_duration(entry.listened_ms),
                egui::FontId::proportional(text::SM),
                theme::TEXT_MAIN,
            );
            let bar_y = rect.bottom() - 4.0;
            let full = rect.width() - 8.0;
            let filled = full * (entry.listened_ms as f32 / max as f32);
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(rect.left() + 4.0, bar_y), egui::vec2(full, 3.0)), 1.5, egui::Color32::from_white_alpha(22));
            painter.rect_filled(egui::Rect::from_min_size(egui::pos2(rect.left() + 4.0, bar_y), egui::vec2(filled, 3.0)), 1.5, theme::accent(ui.ctx()));
        }
    });
}
```

- [ ] **Step 3: Compilar**

Run: `cd native && cargo build 2>&1 | grep -E "^(error|warning: unused)" -A10 | head -50`
Expected: sin errores. Si aparecen avisos de variables sin uso (por ejemplo `StatsSummary`), corregirlos; si `ui.columns` o `paint_cover` exigen ajustes de préstamo (`textures` entre columnas), pasar `textures` por referencia mutable a cada `top_card` ya está contemplado; en caso de conflicto de préstamos con `songs`, clonar el `&[Song]` no es necesario porque es inmutable.

- [ ] **Step 4: Verificar visualmente con datos de demostración**

Run:

```bash
cd native && cargo build --release 2>&1 | tail -1
S=/tmp/claude-1000/-home-budja8-Documents-Proyectos-simple-player-master/5badf05a-e152-478c-aeb0-3922f4d01526/scratchpad
for r in week today month year all; do
  timeout 40 ./target/release/simple-player --stats-demo --tab stats --stats-range $r --shot $S/stats_$r.png >/dev/null 2>&1
done
ls $S | grep stats_
```

Expected: cinco PNG. Revisar cada uno con la herramienta de lectura de imágenes: los cuatro mosaicos, la línea de tiempo con el pico resaltado, los hábitos y las tres tarjetas de tops con carátulas; las etiquetas del eje legibles en Hoy (24), Mes (31) y Todo; sin texto cortado ni solapado. Corregir lo que falle y repetir.

Estados: `--tab stats` sin `--stats-demo` (usa base en memoria vacía) debe mostrar el estado vacío con el botón "Ir a Canciones":

```bash
timeout 40 ./target/release/simple-player --tab stats --shot $S/stats_empty.png >/dev/null 2>&1
```

- [ ] **Step 5: Commit**

```bash
git add native/src
git commit -m "Estadísticas: pantalla con mosaicos, línea de tiempo, hábitos y tops"
```

---

### Task 11: Verificación final

**Files:**
- Modify: `docs/superpowers/specs/2026-10-03-estadisticas-de-escucha-design.md` (solo el encabezado de estado)

**Interfaces:**
- Consumes: todo lo anterior.
- Produces: evidencia de los criterios de aceptación del spec (§10).

- [ ] **Step 1: Suite completa y benchmark**

Run: `cd native && cargo test 2>&1 | grep -E "^error|FAILED|test result"`
Expected: `test result: ok.` con todas las pruebas (las de `stats` incluidas) y 0 fallos.

Run: `cd native && cargo test --release stats::summary::tests::resumen_de_100k -- --ignored --nocapture 2>&1 | grep -E "bench|test result"`
Expected: `[bench] resumen de 100000 eventos: <200 ms` y `ok` (criterio 5).

- [ ] **Step 2: Reposo con la pantalla abierta (criterio 6)**

Run:

```bash
cd native && cargo build --release 2>&1 | tail -1
bash -c './target/release/simple-player --allow-multiple --no-hotkeys --stats-demo --tab stats --bench 12 2>&1 | grep bench'
```

Expected: `[bench] CPU` ≤ ~0.5 %. Si es mayor, buscar un repintado continuo en la pantalla (por ejemplo `skeleton` visible o una animación que no termina) antes de continuar.

- [ ] **Step 3: Ninguna corrida de desarrollo escribió la base real (criterio 7)**

Run: `ls ~/.local/share/simple-player/ 2>&1 | head -2`
Expected: `No such file or directory` si el usuario aún no ha usado la app normal con esta versión; si existe de usos reales, comprobar que su fecha de modificación no cambió durante esta sesión (`stat -c %y ~/.local/share/simple-player/stats.db`).

- [ ] **Step 4: Aceptación manual con el usuario (criterios 2 y 3)**

Pedir al usuario que, **con la app normal** (sin flags):
1. Reproduzca una canción ~10 s y cambie a otra: abrir Estadísticas → debe aparecer 1 reproducción.
2. Salte una canción antes de 5 s: no debe sumar.
3. Cierre la app con una canción sonando, la abra de nuevo: la reproducción en curso debe estar registrada y el historial intacto.
4. Revise los cinco rangos y los estados.

Anotar el resultado en la conversación; si algo falla, corregirlo con una prueba nueva antes de cerrar.

- [ ] **Step 5: Actualizar el estado del spec y commit**

En `docs/superpowers/specs/2026-10-03-estadisticas-de-escucha-design.md`, cambiar la línea de estado del encabezado a:

```markdown
> **Estado:** implementado en `nueva-version` (pendiente de aceptación manual del usuario).
```

```bash
git add docs/superpowers/specs/2026-10-03-estadisticas-de-escucha-design.md
git commit -m "Estadísticas: marca el spec como implementado"
```

---

## Autoevaluación del plan frente al spec

**Cobertura del spec**

| Spec | Tarea |
|---|---|
| §3 decisiones (SQLite bundled, chrono, XDG, snapshot, reparto SQL/Rust, `ended_at`, racha actual global, hilo de fondo) | 1, 3, 4, 5, 6 |
| §4 arquitectura y archivos | Mapa de archivos; 1–10 |
| §5 modelo y esquema SQL, invariantes | 1, 3 |
| §6.1 `SessionTracker` | 2 |
| §6.2 enganches (`play_index`, `tick`/`observe`, `on_exit`) | 8 |
| §6.3 servicio, handle, deshabilitado, flags de desarrollo | 6, 8 |
| §7.1–7.2 rangos, totales, tops, hábitos, línea de tiempo | 4, 5 |
| §7.3 formato en español | 1 |
| §8 pantalla, estados, accesibilidad, reposo | 9, 10, 11 |
| §9 errores y casos límite | Review Focus; 3, 6 |
| §10 pruebas y criterios 1–7 | 2–7 (pruebas), 11 (criterios) |

**Revisión de marcadores:** no quedan "TBD"/"TODO"; los pasos con código muestran el código. Las notas de ajuste condicionales (tiempos de vida de `Box<dyn Fn>` en `aggregate`, `bind` en `store`) describen una alternativa concreta de comportamiento idéntico.

**Consistencia de tipos:** `StatsSummary`, `Habits`, `Totals`, `TopEntry`, `TimelineBucket`, `EventRow`, `SongSnapshot`, `PlayEvent` y `StatsRange` se definen en la Tarea 1 y se usan con los mismos campos en 3–10. `StatsHandle::{events_version, latest_summary, request_summary, record, record_batch, flush, disabled, spawn}` (Tarea 6) coinciden con su uso en las Tareas 7, 8 y 10. `StatsRecorder::{song_started, observe, finish, *_at, handle}` (Tarea 7) coinciden con la Tarea 8. `AppState.stats`, `stats_range`, `stats_requested` (Tarea 8) coinciden con la Tarea 10. `bar_chart(ui, &buckets, peak, height)` (Tarea 9) coincide con la Tarea 10.

**Desviación respecto al spec, ya señalada arriba:** `events_version()` cuenta solo eventos guardados (el spec decía "eventos o resúmenes listos") para evitar un bucle de peticiones.
