# Simple Player — Configuración y tipografía unificada

> **Estado:** implementado en `nueva-version`; falta la aceptación manual del usuario (criterios 1, 3, 5 y 6 con la app normal).
> **Rama de trabajo:** `nueva-version`.
> **Fecha:** 2026-10-03

---

## 1. Objetivo y alcance

Dos cambios independientes:

1. **Tipografía unificada.** Toda la interfaz usa la familia **GTA Art Deco** (ya en `assets/fonts/`). Noto Sans queda solo como respaldo de glifos que Art Deco no tiene.
2. **Pantalla de Configuración.** Una opción nueva del sidebar, con su pantalla, donde el usuario administra las carpetas de música (varias), algunas preferencias y las acciones de datos: exportar, importar y borrar el historial, y restablecer de fábrica.

**Fuera de alcance (YAGNI)**

- Exportar a CSV, temas o colores configurables, atajos configurables.
- Mover el escaneo a un hilo con barra de progreso (sigue siendo síncrono; con varias carpetas muy grandes puede congelar la ventana un momento).
- Sincronización, copias de seguridad automáticas y cualquier uso de red.
- Importar historial de otras apps; solo se importa el formato que exporta esta app.

---

## 2. Tipografía

### 2.1 Mapeo

| Rol | Hoy | Después |
|---|---|---|
| Texto normal (familia `Proportional`) | Noto Sans Regular | `GTAArtDeco_Regular.ttf` |
| Negritas (`theme::bold`) | Noto Sans Bold | `GTAArtDeco_Bold.ttf` |
| Títulos y logo (`theme::deco`) | `GTAArtDeco_CondensedBold.ttf` | sin cambios |
| Respaldo de glifos | Noto Sans (ya en la cadena) | `NotoSans-Regular.ttf`, solo como respaldo |
| Íconos | Phosphor | sin cambios |

- Cada familia con nombre (`bold`, `deco`) y la familia `Proportional` tienen la cadena: Art Deco del peso correspondiente → Noto Sans (respaldo) → Phosphor → fuentes por defecto de egui (emoji).
- `NotoSans-Bold.ttf` deja de embeberse y se elimina del repositorio; `NotoSans-Regular.ttf` y su licencia se conservan. Los pesos Medium y Condensed Heavy de Art Deco siguen sin usarse.
- Verificado: Art Deco (los cinco pesos) cubre todo el español (`áéíóúüñ`, mayúsculas, `¿¡`, dígitos y la puntuación de uso común).

### 2.2 Estructura

- Las constantes y el armado de familias pasan a una función pura `build_font_definitions() -> egui::FontDefinitions` en `src/theme/`, para poder probarla sin ventana.
- `FONT_BOLD` pasa a nombrarse `GTAArtDecoBold` (su valor deja de decir `NotoSansBold`). Las funciones `theme::bold()` y `theme::deco()` conservan su nombre y firma.

### 2.3 Efecto en el diseño

Art Deco es algo más ancha que Noto Sans. Se revisa con capturas a 1050×750 y 1000×650 (lista de canciones, barra inferior, hero, estadísticas, cola, pantalla completa y la nueva Configuración). Donde un texto deje de caber se ajusta el tamaño de la escala (`theme::text`) o el truncado, no el diseño.

---

## 3. Ajustes (`settings.json`)

### 3.1 Modelo y archivo

```rust
pub struct Settings {
    pub music_folders: Vec<String>,   // rutas absolutas, sin repetidos ni vacías
    pub default_visualizer: String,   // etiqueta de VisualizerMode ("Barras" por defecto)
    pub remember_window_size: bool,   // true por defecto
}
```

- Archivo: `$XDG_CONFIG_HOME/simple-player/settings.json` (por defecto `~/.config/simple-player/settings.json`), junto a `window.json`. JSON con campo `"version": 1`.
- Módulo `src/settings.rs`, puro salvo `load`/`save`: `Settings::default()`, `parse`, `to_json`, `sanitized()`, `add_folder`, `remove_folder`, `settings_path(xdg, home)`.
- `sanitized()`: quita rutas vacías y repetidas (conserva el orden), pone `default_visualizer` en "Barras" si la etiqueta ya no existe (por ejemplo "Radial" o "Resplandor") y no falla con campos ausentes (`#[serde(default)]`).
- Un archivo corrupto o de una versión mayor se ignora con un aviso en la consola y se usan los valores por defecto (no se sobrescribe hasta el siguiente guardado normal).

### 3.2 Migración

Si `settings.json` no existe y `playback_state.json` trae `folder_path`, los ajustes arrancan con `music_folders = [folder_path]`. Desde entonces `playback_state.json` **deja de escribir** `folder_path` (el campo se sigue leyendo, con `#[serde(default)]`, solo para esta migración).

### 3.3 Corridas de desarrollo

`--shot`, `--bench` y `--gallery` no leen ni escriben `settings.json` (ajustes en memoria con los valores por defecto), igual que con la base de estadísticas y `window.json`. Nuevo flag de desarrollo, repetible: `--music-folder <ruta>` siembra carpetas en esos ajustes en memoria (para capturas de la pantalla de Configuración).

### 3.4 Efecto en el resto de la app

- `AppState` guarda `settings: Settings`; `current_folder_path` desaparece. El nombre de carpeta del sidebar y el texto de `songs.rs` leen `settings.music_folders`.
- `default_visualizer` fija el modo inicial de la pantalla completa (salvo que se pase `--viz`). Cambiar de modo desde la pantalla completa **no** cambia el valor guardado; solo se cambia en Configuración.
- `remember_window_size = false`: `window.json` ni se lee ni se escribe y la app abre en 1050×750.

---

## 4. Biblioteca con varias carpetas

- `library::scan_music_folders(folders: &[String]) -> Vec<Song>`: recorre cada carpeta en orden con `walkdir`, une los resultados y **quita duplicados por ruta** (una carpeta dentro de otra no repite canciones). Una carpeta que no existe o no se puede leer se omite con un aviso; sigue en los ajustes.
- Sin carpetas configuradas y sin caché, la biblioteca queda **vacía** (se elimina el valor implícito `~/Música`). El estado vacío de `songs.rs` ofrece "Elegir carpeta", que abre el diálogo y agrega la carpeta directamente.
- Caché (`library_cache.json`): se sigue sirviendo al arrancar si existe. Agregar una carpeta, quitarla, "Re-escanear" y limpiar la caché de carátulas vuelven a escanear y reescriben la caché.
- Quitar una carpeta quita sus canciones de la biblioteca. La cola activa no se toca (igual que hoy al cambiar de carpeta); lo que suena sigue sonando.
- Se conserva el módulo de extracción de metadatos y carátulas tal cual.

---

## 5. Pantalla de Configuración

### 5.1 Navegación

- `ActiveTab::Settings` (nueva). El sidebar muestra "Configuración" (ícono de engrane de Phosphor) en una sección propia "APLICACIÓN", debajo de "BIBLIOTECA". Se elimina la sección "CARPETA" del sidebar.
- `--tab settings` abre la pantalla (flag de desarrollo, como los demás `--tab`).

### 5.2 Secciones

Cada sección es una tarjeta de vidrio (`paint_glass`) con título en Art Deco condensado y filas con etiqueta, descripción corta y control. La pantalla hace scroll si no cabe.

**Biblioteca**
- Lista de carpetas: ruta (con truncado y la ruta completa en el tooltip), un aviso "No se encuentra" si no existe y un botón para quitarla.
- "Agregar carpeta" (diálogo de selección de carpeta; ignora la que ya esté).
- Resumen: "N canciones en M carpetas".
- "Re-escanear".
- "Limpiar caché de carátulas": muestra cuánto ocupa (suma de `covers/`), borra los archivos y re-escanea para regenerarlos.

**Apariencia y ventana**
- "Visualizador por defecto": selector que recorre los modos como el chip de la pantalla completa.
- "Recordar el tamaño de la ventana": interruptor.

**Datos**
- "Exportar historial…" (§6.1), "Importar historial…" (§6.2), "Borrar historial" (§6.3, con confirmación) y "Restablecer de fábrica" (§6.4, con confirmación).
- Resultado de la última acción en una línea de estado bajo los botones ("128 eventos exportados", "12 importados, 3 repetidos", errores en rojo).

**Acerca de**
- Versión (`env!("CARGO_PKG_VERSION")`) y las rutas de datos (ajustes, base de estadísticas, caché), cada una con "Abrir carpeta" (`xdg-open`; si falla no pasa nada).

### 5.3 Confirmaciones

"Borrar historial" y "Restablecer de fábrica" no actúan al primer clic: el botón se reemplaza por "¿Seguro? **Sí, borrar** · Cancelar" en la misma fila. Cambiar de pantalla cancela la confirmación pendiente.

### 5.4 Reglas de diseño

Sigue el sistema visual vigente (tarjetas de vidrio, acento dinámico, escala `theme::text`, `PillButton`, `nav_item`). La pantalla no pide repintado continuo.

---

## 6. Acciones de datos

El historial vive en SQLite y lo maneja el hilo del `StatsService`; las acciones se piden por mensajes y el resultado se publica para que la pantalla lo muestre, como los resúmenes (la UI nunca espera al disco).

### 6.1 Exportar

- Diálogo "Guardar como" (`rfd`), nombre sugerido `simple-player-historial-AAAA-MM-DD.json`.
- Formato versionado:

```json
{
  "app": "simple-player",
  "version": 1,
  "exported_at": 1760000000000,
  "events": [
    { "started_at": 0, "ended_at": 0, "listened_ms": 0,
      "path": "", "title": "", "artist": "", "album": "", "duration_ms": 0 }
  ]
}
```

- Tiempos en milisegundos desde epoch UTC (iguales a los de la base). Sin eventos exporta una lista vacía (no es un error).

### 6.2 Importar

- Diálogo "Abrir archivo". Valida `app == "simple-player"` y `version == 1` (una versión mayor da "versión no compatible"; un JSON inválido da un error y no se inserta nada).
- **Duplicado** = mismo `started_at`, `ended_at` y `path` que un evento ya guardado.
- Un evento no válido (`listened_ms == 0`, tiempos negativos o `ended_at < started_at`) se cuenta como "inválido" y se omite.
- Las inserciones van en **una sola transacción**: o entran todas las nuevas o ninguna.
- Resultado: "N importados, M repetidos" (y "K inválidos" si los hay). Sube `events_version` para que la pantalla de Estadísticas se refresque.

### 6.3 Borrar historial

`DELETE FROM play_events` y `VACUUM`; sube `events_version`. No toca ajustes ni biblioteca.

### 6.4 Restablecer de fábrica

- Borra: `settings.json`, `window.json`, `playback_state.json`, `library_cache.json` y la carpeta `covers/`.
- **Conserva** el historial de estadísticas (tiene su propio botón) y los archivos de música.
- Efecto inmediato en memoria: ajustes por defecto, biblioteca vacía, cola vaciada, reproducción detenida y volumen por defecto. El tamaño de la ventana vuelve al valor por defecto en el siguiente arranque (se avisa en la línea de estado).
- En corridas de desarrollo esta acción está deshabilitada (nunca toca archivos reales).

### 6.5 Errores

Sin permisos de escritura, disco lleno o archivo inexistente: la línea de estado muestra el error en español y la app sigue funcionando. Si las estadísticas están desactivadas (la base no abrió) los botones de historial se deshabilitan con la razón en el tooltip.

---

## 7. Pruebas

**Tipografía:** `build_font_definitions` — Art Deco Regular es la primera de `Proportional`; las familias `bold` y `deco` empiezan con su peso Art Deco; Noto Sans sigue en las tres cadenas como respaldo; Phosphor sigue presente.

**Ajustes:** valores por defecto; ida y vuelta JSON; archivo vacío, corrupto o con campos faltantes; `sanitized` (vacías, repetidos, etiqueta de visualizador obsoleta); `add_folder`/`remove_folder` (repetido, ausente); `settings_path` con y sin `XDG_CONFIG_HOME`; migración desde `playback_state.json`; guardar crea el directorio.

**Biblioteca:** `scan_music_folders` con carpetas temporales — une dos carpetas, no repite canciones de carpetas anidadas, omite una carpeta inexistente, lista vacía da biblioteca vacía.

**Datos:** exportar → importar en una base vacía reproduce los eventos; importar dos veces no duplica; archivo corrupto o de versión mayor no inserta nada; eventos inválidos se cuentan y omiten; transacción única (un fallo a mitad no deja eventos parciales); borrar historial deja la tabla vacía; restablecer de fábrica borra exactamente los archivos listados y no toca el historial (sobre un directorio temporal).

**Navegación y pantalla:** `select_tab(Settings)`; el estado de confirmación se cancela al salir de la pantalla (lógica pura). Capturas con `--tab settings --music-folder …` a 1050×750 y 1000×650.

---

## 8. Criterios de aceptación

1. Todo el texto de la interfaz usa Art Deco (Regular, Bold y Condensed Bold); Noto Sans solo aparece como respaldo y nada se ve cortado o desbordado a 1050×750 ni a 1000×650.
2. El sidebar muestra "Configuración" y ya no tiene la sección "Carpeta".
3. Se pueden agregar y quitar varias carpetas; la biblioteca junta sus canciones sin repetir; una carpeta inexistente se marca y se omite.
4. La carpeta que ya usaba el usuario sigue ahí tras actualizar (migración).
5. Exportar → importar conserva el historial y no duplica; "Borrar historial" y "Restablecer de fábrica" piden confirmación y hacen lo descrito.
6. "Visualizador por defecto" y "Recordar el tamaño de la ventana" se respetan al reiniciar.
7. Las corridas de desarrollo no tocan `settings.json`, `window.json` ni la base real.
8. `cargo test` en verde y sin advertencias del compilador.
9. README actualizado (pantalla de Configuración, ubicación de `settings.json`, `--music-folder`).

---

## 9. Orden de implementación

1. Tipografía (independiente).
2. Ajustes (`settings.rs`) y migración, conectados a `AppState`.
3. Biblioteca con varias carpetas.
4. Sidebar, `ActiveTab::Settings` y la pantalla con las secciones Biblioteca, Apariencia y Acerca de.
5. Acciones de datos (exportar, importar, borrar y restablecer).
6. README y verificación final.
