# Simple Player — Estadísticas de escucha (SQLite)

> **Estado:** implementado en `nueva-version`; falta la aceptación manual del usuario (criterios 2 y 3 con la app normal).
> **Rama de trabajo:** `nueva-version` (sin push ni merge por ahora).
> **Fecha:** 2026-10-03
> **Referencia:** el sistema de estadísticas de `PixelPlayer-master/` (app Android del mismo autor).

---

## 1. Objetivo y alcance

**Qué se quiere:** que Simple Player registre lo que el usuario escucha y le muestre su propia actividad: tiempo total, reproducciones, tops de canciones, artistas y álbumes por rango, hábitos (días activos, rachas, sesiones, día pico) y una línea de tiempo en barras.

**Alcance de la versión 1 ("núcleo + hábitos")**

- Captura confiable del tiempo real escuchado.
- Almacenamiento local en SQLite.
- Pantalla "Estadísticas" con rangos Hoy, Semana (desde el lunes), Mes, Año y Todo.

**Fuera de la versión 1 (YAGNI)**

- Géneros (el modelo `Song` no los tiene).
- Distribución por hora del día y donut de concentración de pistas.
- Tarjeta resumen en la pantalla principal.
- Exportar, respaldar o importar el historial.
- Separar artistas por coma (rompería nombres como "Tyler, The Creator").
- "Tu año en música" y reproducir al hacer clic en un top.
- Checkpoints periódicos de la sesión en curso.
- Sincronización entre dispositivos y cualquier uso de red.

**Privacidad:** todo queda local; no hay red.

---

## 2. Referencia: PixelPlayer y qué se adopta

PixelPlayer organiza el sistema en cuatro capas: captura (`ListeningStatsTracker`), almacenamiento (`playback_history.json`), agregación al leer (`PlaybackStatsRepository.buildSummaryFromEvents`) y presentación (`StatsViewModel` + `StatsScreen`).

| Aspecto | PixelPlayer | Simple Player |
|---|---|---|
| Medición | Tiempo real acumulado mientras suena (deltas de reloj monotónico), no la posición | **Igual** |
| Umbral | 5 s | **Igual** |
| Cierre de sesión | Cambio de pista, parada, cierre de la app | **Igual** |
| Almacén | JSON único reescrito en cada evento | **SQLite** (decisión del usuario) |
| Fusión de solapes | Sí (varios dispositivos y cast) | **No**: hay un solo reproductor; el total es la suma de lo escuchado |
| Rangos | Día, semana, mes, año, todo (por calendario) | **Igual** |
| Sesiones | Hueco de más de 30 min | **Igual** |
| Artistas | Separa multi-artista | **No** en v1 |
| Poda | 730 días | **Sin poda** (el volumen es pequeño) |
| Inicio de un evento | `fin − duración` | Inicio y fin **reales** más `listened_ms` aparte (las pausas no desplazan el inicio) |

---

## 3. Decisiones tomadas

1. **SQLite con `rusqlite` y la feature `bundled`**: se compila dentro del binario; no hay dependencia de sistema para el empaquetado.
2. **`chrono`** para convertir marcas de tiempo a fecha y hora local (días, semanas, meses).
3. **Ubicación:** `$XDG_DATA_HOME/simple-player/stats.db`, por defecto `~/.local/share/simple-player/stats.db`. Los datos del usuario no van en `~/.cache`.
4. **Cada evento guarda un snapshot** de la canción (ruta, título, artista, álbum, duración) para que el historial sobreviva a archivos movidos o borrados.
5. **Reparto del cálculo:** SQL para filtrar por rango y para los tops (`GROUP BY`); Rust para lo que es lógica de fechas locales e intervalos (días activos, rachas, sesiones, día pico, buckets).
6. **Un evento pertenece al día en que terminó** (`ended_at`). Un evento dura como máximo una canción, así que cruzar medianoche es despreciable.
7. **La racha actual se calcula sobre todo el historial**, sin importar el rango elegido; la racha más larga, dentro del rango.
8. **El resto de la app solo habla con un hilo de fondo** (`StatsService`): la UI nunca espera al disco.

---

## 4. Arquitectura

Módulo nuevo `native/src/stats/`, con lógica pura separada de la UI (como `viz/`):

```
native/src/stats/
├─ mod.rs        # reexporta la API pública
├─ model.rs      # PlayEvent, SongSnapshot, StatsRange, StatsSummary y sus partes
├─ session.rs    # SessionTracker: máquina de estados pura (el tiempo se inyecta)
├─ store.rs      # Store: abrir, migrar, insertar, consultar por rango y tops
├─ aggregate.rs  # cálculos de hábitos y línea de tiempo sobre filas (puro)
├─ format.rs     # duraciones, días y meses en español (puro)
└─ service.rs    # StatsService: hilo + canal; StatsHandle para el resto de la app
```

UI:

```
native/src/ui/screens/stats.rs     # pantalla
native/src/ui/widgets/bar_chart.rs # gráfico de barras con tooltip
```

Cambios en código existente: `ActiveTab::Stats`, entrada en `shell/sidebar.rs`, `screens/mod.rs`, `shell/topbar.rs` (ocultar buscador), `state.rs` (enganches y rango elegido), `main.rs` (crear el servicio, `on_exit`, flags de desarrollo), `theme/icons.rs` (ícono de barras), `Cargo.toml` (`rusqlite`, `chrono`).

### Dependencias

- `rusqlite = { version = "0.32", features = ["bundled"] }`
- `chrono = { version = "0.4", default-features = false, features = ["clock", "std"] }`

(Las versiones exactas se fijan en el plan, comprobando compatibilidad con el resto del árbol.)

---

## 5. Modelo de datos

```sql
CREATE TABLE play_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at  INTEGER NOT NULL,                 -- ms desde epoch UTC
    ended_at    INTEGER NOT NULL,                 -- ms desde epoch UTC
    listened_ms INTEGER NOT NULL CHECK (listened_ms > 0),
    song_path   TEXT    NOT NULL,
    title       TEXT    NOT NULL,
    artist      TEXT    NOT NULL,                 -- '' si no hay etiqueta
    album       TEXT    NOT NULL,                 -- '' si no hay etiqueta
    duration_ms INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_play_events_ended_at ON play_events (ended_at);
```

- `PRAGMA journal_mode = WAL;` y `PRAGMA user_version = 1;` (versión del esquema; migraciones futuras por comparación).
- Invariantes al insertar: `ended_at >= started_at`; `listened_ms > 0`; si `ended_at < started_at` se corrige `started_at = ended_at - listened_ms`.
- Un artista o álbum vacío se **muestra** como "Artista desconocido" / "Álbum desconocido" y se agrupa bajo la cadena vacía.

### Tipos Rust (esbozo)

```rust
pub struct SongSnapshot { pub path: String, pub title: String, pub artist: String, pub album: String, pub duration_ms: u64 }
pub struct PlayEvent   { pub started_at: i64, pub ended_at: i64, pub listened_ms: u64, pub song: SongSnapshot }
pub enum StatsRange    { Today, Week, Month, Year, All }
```

---

## 6. Captura

### 6.1 `SessionTracker` (puro)

Estado: sesión activa opcional con `song`, `started_at_ms`, `listened_ms`, `last_instant`, `playing`, `last_playing_epoch_ms`.

API (todo el tiempo llega por parámetro, sin reloj interno):

- `start(song, now: Instant, epoch_ms: i64, playing: bool) -> Option<PlayEvent>`: cierra la sesión anterior (si cuenta) y abre otra.
- `observe(playing: bool, now: Instant, epoch_ms: i64)`: suma `now − last_instant` **solo si la observación anterior estaba reproduciendo**; actualiza `playing` y `last_playing_epoch_ms`.
- `finish(now: Instant, epoch_ms: i64) -> Option<PlayEvent>`: cierra la sesión.

Reglas:

- Un evento solo se emite si `listened_ms >= 5000`.
- `ended_at` = `epoch_ms` si estaba reproduciendo al cerrar; si estaba en pausa, el **último instante en que sonó** (`last_playing_epoch_ms`).
- Los seeks no suman tiempo (se mide reloj, no posición). Repetir la misma canción (repetir uno) abre una sesión nueva: cuenta como otra reproducción.

### 6.2 Enganches en `AppState`

- `play_index` llama a `start(...)` con el snapshot de la canción (cierra la anterior y envía el evento resultante al servicio).
- `tick()` (ya se ejecuta cada frame) llama a `observe(is_playing, ...)`. Esto cubre pausas desde la UI, MPRIS y teclas multimedia, porque todas terminan en `is_playing`.
- `eframe::App::on_exit` llama a `finish(...)` y hace **flush síncrono** al servicio (espera como máximo 1 s).
- Si la app muere de golpe se pierde solo la canción en curso.

### 6.3 `StatsService` y `StatsHandle`

- `StatsService::spawn(db_path, repaint: egui::Context) -> StatsHandle` abre el almacén en un hilo dedicado y atiende mensajes de un canal.
- Mensajes: `Record(PlayEvent)`, `Summary(StatsRange)`, `Flush(Sender<()>)`, `Shutdown`.
- `StatsHandle::record(event)` envía y retorna sin bloquear.
- `StatsHandle::request_summary(range)` pide el cálculo; el resultado queda disponible con `latest_summary() -> Option<(StatsRange, Arc<StatsSummary>)>` y una **versión** (contador) que sube con cada evento nuevo o resumen listo, además de despertar la UI con `ctx.request_repaint()`.
- Si la base no abre o falla, el handle queda **deshabilitado**: `record` no hace nada, `latest_summary` devuelve `None` y expone el motivo para mostrarlo en la pantalla. La reproducción nunca se ve afectada.
- Las corridas de desarrollo (`--shot`, `--bench`, `--gallery`) no escriben en la base real: usan una base temporal o ninguna. `--stats-db <ruta>` fuerza otra ruta y `--stats-demo` siembra datos sintéticos en una base temporal.

---

## 7. Cálculo

### 7.1 Rangos (hora local, por calendario)

| Rango | Inicio | Fin |
|---|---|---|
| Hoy | 00:00 de hoy | ahora |
| Semana | lunes 00:00 de esta semana | ahora |
| Mes | día 1 a las 00:00 | ahora |
| Año | 1 de enero a las 00:00 | ahora |
| Todo | sin límite | ahora |

El filtro es `ended_at BETWEEN inicio AND fin` y usa el índice.

### 7.2 Resumen (`StatsSummary`)

**Totales**

- `total_listened_ms = SUM(listened_ms)`; `plays = COUNT(*)`; `unique_songs = COUNT(DISTINCT song_path)`.
- `days_in_span`: días desde el inicio del rango (en Todo, desde el día del primer evento) hasta hoy, ambos inclusive, mínimo 1.
- `avg_daily_ms = total_listened_ms / days_in_span`.

**Tops de 5, por tiempo** (desempate por reproducciones y luego por nombre)

- Canciones: `GROUP BY song_path`.
- Artistas: `GROUP BY artist`.
- Álbumes: `GROUP BY album, artist` (un álbum se identifica por álbum y artista, igual que en la pestaña Álbumes).
- Cada fila: nombre, tiempo, reproducciones y, para canciones y álbumes, la ruta de una canción de referencia para resolver la carátula en la UI.

**Hábitos** (sobre los eventos del rango, salvo la racha actual)

- `active_days`: fechas locales distintas de `ended_at`.
- `longest_streak_days`: la corrida más larga de fechas consecutivas.
- `current_streak_days`: sobre **todo el historial**; cuenta hacia atrás desde hoy si hoy tiene actividad, o desde ayer si hoy no la tiene pero ayer sí; 0 si ninguno.
- `sessions`: eventos ordenados por `started_at`; dos eventos son de la misma sesión si `started_at(sig) − ended_at(ant) <= 30 min`. Por sesión: suma de `listened_ms`. Resultado: cantidad, promedio, la más larga y `sessions_per_day = sessions / days_in_span`.
- `peak_weekday`: día de la semana con más tiempo total, solo si el rango abarca más de un día.

**Línea de tiempo** (suma de `listened_ms` por bucket; se marca el pico si el máximo es mayor que cero)

| Rango | Buckets |
|---|---|
| Hoy | 24 horas |
| Semana | 7 días (Lun…Dom) |
| Mes | un bucket por día del mes actual |
| Año | 12 meses |
| Todo | por mes, o por año si el historial abarca más de 36 meses |

### 7.3 Formato (`format.rs`, español)

- Duración larga: `"41 h 12 min"`, `"1 h"`, `"45 min"`, `"3 min 20 s"` (bajo 10 min se muestran segundos), `"0 min"`.
- Días: Lun, Mar, Mié, Jue, Vie, Sáb, Dom (completos en el día pico). Meses: ene, feb, mar, abr, may, jun, jul, ago, sep, oct, nov, dic.

---

## 8. Interfaz

**Navegación:** entrada "Estadísticas" en el sidebar (ícono de barras) tras Artistas; `ActiveTab::Stats`. En esta pestaña la barra superior oculta el buscador.

**Pantalla** (columna con scroll, tarjetas de vidrio del sistema de diseño):

1. **Encabezado:** "ESTADÍSTICAS" (Art Deco) y chips de rango; el rango por defecto es **Semana**.
2. **Mosaicos:** tiempo total, reproducciones, canciones únicas, promedio diario.
3. **Línea de tiempo:** gráfico de barras con degradado (misma receta que el visualizador), pico resaltado con el acento y tooltip con la etiqueta y el valor del bucket.
4. **Hábitos:** días activos, racha actual, racha más larga, sesiones, sesión promedio, sesión más larga, día pico.
5. **Tops:** tres tarjetas (canciones, artistas, álbumes); cada fila con posición, carátula (si la canción sigue en la biblioteca; si no, un ícono), nombre, tiempo, reproducciones y una barra proporcional al primero.

**Estados**

- *Vacío* (sin eventos en el rango): ícono, "Aún no hay estadísticas" y "Reproduce una canción más de 5 segundos para empezar", con un botón que lleva a Canciones.
- *Cargando:* esqueletos mientras el hilo calcula.
- *Deshabilitado:* mensaje con el motivo ("No se pudo abrir la base de estadísticas: …").
- *Actualización:* la pantalla se refresca sola cuando sube la versión del servicio.

**Rendimiento y accesibilidad:** la pantalla no pide repintado continuo (el reposo se mantiene en ~0.2 % de CPU); los chips son enfocables con Tab y con aro de foco; el texto cumple el contraste AA ya verificado en el tema.

---

## 9. Errores y casos límite

- Base inexistente: se crea. Base corrupta o sin permisos: servicio deshabilitado con aviso; no se borra ni se renombra nada automáticamente.
- Reloj del sistema que retrocede: `ended_at < started_at` se corrige al insertar; eventos con `listened_ms == 0` se descartan.
- Duración desconocida (`duration_ms == 0`): se guarda 0 y no afecta a ningún cálculo.
- Canción borrada o movida: el evento conserva su snapshot; en la UI la carátula cae al ícono.
- Dos instancias: la instancia única ya lo impide; las corridas de desarrollo usan otra base.
- Cambio de horario (DST): las fechas se calculan con la zona local del sistema; **no hay tests automáticos de DST** (se prueba con desfases fijos). Límite conocido.

---

## 10. Pruebas y criterios de aceptación

**Pruebas automáticas** (sin audio ni GPU)

- `session.rs` con reloj inyectado: acumula solo mientras suena; pausa y reanudación; descarte bajo 5 s; `ended_at` cuando se cierra en pausa; repetir la misma canción abre otra sesión; `start` devuelve el evento de la sesión anterior.
- `aggregate.rs` con eventos fijos y zona de desfase fijo: semana que empieza el lunes; racha que cruza fin de mes; hueco de sesión exactamente de 30 min (se une) y de 30 min y 1 ms (se separa); buckets por rango; resumen vacío; racha actual desde hoy y desde ayer; día pico.
- `store.rs` con SQLite en memoria: inserción y consulta por rango (bordes inclusivos), orden y desempates de los tops, agrupación de vacíos, migración por `user_version`.
- `format.rs`: duraciones y nombres.
- `service.rs`: `record` no bloquea; un `Flush` deja los eventos consultables; un servicio con ruta inválida queda deshabilitado sin entrar en pánico.

**Criterios de aceptación**

1. `cargo test` pasa, con las pruebas nuevas incluidas.
2. Reproducir una canción más de 5 s y cambiar de pista crea un evento; menos de 5 s no crea nada; cerrar la app con una canción sonando guarda su evento.
3. Los datos sobreviven a reiniciar la app.
4. La pantalla muestra los cinco rangos con datos sembrados por `--stats-demo` (capturas revisadas) y los tres estados (vacío, cargando, deshabilitado).
5. Un resumen sobre **100 000 eventos** tarda menos de ~200 ms en release (benchmark marcado como ignorado en `cargo test`).
6. El reposo con la pantalla abierta no supera ~0.5 % de CPU; reproduciendo, el costo de registrar eventos es imperceptible.
7. Ninguna corrida de desarrollo modifica `~/.local/share/simple-player/stats.db`.

---

## 11. Riesgos

| Riesgo | Mitigación |
|---|---|
| La primera compilación con SQLite incluido añade ~40 s y ~1 MB | Se acepta; solo afecta al build |
| Cambio de horario sin cobertura automática | Documentado; se prueba con desfases fijos |
| `rusqlite` no es `Sync` | La conexión vive solo en el hilo del servicio |
| Perder la canción en curso si el proceso muere | Aceptado en v1 (sin checkpoints) |

---

## 12. Fases de implementación sugeridas (para el plan)

1. **Modelo y captura:** `model.rs`, `session.rs`, `format.rs` con sus pruebas.
2. **Almacén y servicio:** `store.rs`, `service.rs` con SQLite en memoria; dependencia nueva.
3. **Cálculo:** `aggregate.rs` y los tops en SQL.
4. **Integración:** enganches en `AppState` y `main.rs`, flags de desarrollo, `on_exit`.
5. **Pantalla:** pestaña, mosaicos, gráfico de barras, hábitos, tops y estados.
6. **Verificación:** datos de demostración, capturas de los cinco rangos, benchmark de 100 000 eventos y medición de reposo.

---

## 13. Decisiones abiertas

Ninguna. Los supuestos y alcance fueron confirmados por sección en la conversación de diseño.
