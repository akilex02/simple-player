# Simple Player — Rediseño de UI y corrección del visualizador (egui)

> **Estado:** borrador para revisión. Sin código modificado y sin commits.
> **Rama de trabajo prevista:** `nueva-version` (hoy solo existe como `origin/nueva-version`; hay que crear la rama local antes de implementar).
> **Fecha:** 2026-10-02
> **Origen:** análisis de lectura de `origin/nueva-version @ 67a1ffc`. Las causas del tirón del visualizador son **hipótesis sin medir** (el análisis se hizo en Windows, la app corre en Linux). La Fase 0 las confirma o descarta antes de arreglar nada.

---

## 1. Objetivo y alcance

**Qué se quiere**

1. Una interfaz moderna con estética **GTA VI / liquid-glass**, combinada con la **funcionalidad y la estructura de Spotify / Apple Music**.
2. Un visualizador de espectro **fluido**, sin tirones de fps.
3. Buen rendimiento en Linux (Wayland y X11), con consumo bajo en reposo.
4. Conservar lo que `nueva-version` ya tiene y `master` no: letras sincronizadas, tres visualizadores, MPRIS2, atajos globales, pantalla completa, tabla virtualizada y carátulas asíncronas.

**Decisiones ya tomadas**

- Se queda **eframe/egui**. Los tirones del visualizador vienen del manejo de datos y del frame loop, no del framework. El rendimiento de egui ya está validado en la máquina del usuario, y la UI (`native/src/ui/`, ~30 KB) está separada de la lógica, así que migrar después seguiría siendo barato.
- Estética: GTA VI / liquid-glass (paleta y tipografía de la versión Tauri) con la estructura de navegación de Spotify y Apple Music.
- Plataforma: Linux primero. No se invierte en Windows ni macOS.

**Fuera de alcance (YAGNI)**

- Playlists propias, streaming, sincronización en la nube, ecualizador.
- Barra de título propia: se mantienen las decoraciones del sistema (en Wayland con egui es frágil).
- Blur real de fondo con shaders (queda como extra opcional en la Fase 6).

---

## 2. Diagnóstico del estado actual

### 2.1 Visualizador (hipótesis ordenadas por probabilidad)

| # | Hipótesis | Evidencia en el código |
|---|---|---|
| H1 | **Sin interpolación entre muestras.** El espectro llega a 20 Hz y se dibuja crudo en cada frame. | `audio/player.rs`: `interval=50000000` (50 ms). `visualizers/bars.rs`: usa `spectrum` tal cual cada frame. `main.rs`: `request_repaint()` incondicional (~180 fps). Cada valor se ve ~9 frames y salta de golpe. |
| H2 | **Mensajes sin alinear con el reloj de reproducción.** | `audio/spectrum.rs`: el `sync_handler` envía por `mpsc` al instante, ignorando `stream-time`/`running-time` del mensaje. `spectrum` está antes del sink, así que los mensajes llegan en ráfagas y adelantados respecto al audio audible. |
| H3 | **Trabajo repetido en cada frame** que mete jitter en el tiempo de frame. | `ui/song_table.rs`: `state.sorted_songs().into_iter().cloned().collect()` clona las 628 canciones en cada frame. `artists_grid.rs`: `artist_groups()` en cada frame. |
| H4 | **Contención de mutex.** | `state.tick()` llama a `audio.position_secs()` (lock sobre `Mutex<Player>`) cada frame; el hilo de MPRIS usa el mismo mutex. |
| H5 | **La métrica de FPS no sirve.** | `PLAN_SIGUIENTE_SESION.md` ya documenta que `unstable_dt` marcó 179 fps con tirones visibles. |

### 2.2 Otros problemas encontrados (no son el bug, pero hay que corregirlos)

- **Caché de texturas sin evicción y sobredimensionada.** `ui/textures.rs` guarda cada carátula hasta 256×256 RGBA (~256 KB) para miniaturas de 30 px. Con 628 canciones eso puede acercarse a ~160 MB de texturas. Hace falta evicción (LRU) y tamaños por uso (64 / 256 / 512 px).
- **Los atajos globales pueden matar la app.** `main.rs` hace `.expect(...)` al registrar hotkeys; en Wayland, o con otra instancia abierta, el proceso muere (`X_GrabKey BadAccess`). Debe degradar a "sin atajos globales" con un aviso.
- **Overlay "Grabar FPS" siempre visible.** Pasa a ser un HUD de rendimiento oculto (ver Fase 0).
- **Ícono cargado desde ruta relativa** (`assets/icons/icon.png`): falla si no se ejecuta desde `native/`. Hay que incrustarlo con `include_bytes!`.
- **Código Tauri duplicado** (`src/`, `src-tauri/`, `package.json`, ...): se elimina en la Fase 7.

### 2.3 Por qué la UI se ve mal

- Casi todo son widgets por defecto de egui con emojis como íconos.
- Anchos fijos (320/200/200), sin layout adaptable.
- Sin jerarquía visual, profundidad, animaciones ni estados de hover.
- GTAArtDeco solo se usa en el logo.
- Faltan vistas y funciones que esperan de un reproductor moderno: álbumes, cola visible, "Reproduciendo ahora" con presencia.

---

## 3. Dirección de diseño

### 3.1 Concepto

> **"Atardecer en Vice City, en cristal."** Fondo oscuro índigo con luz cálida de atardecer (rosa → naranja → púrpura), superficies de vidrio translúcido y tipografía Art Deco solo donde aporta marca. Sobre eso, la ergonomía de Spotify (sidebar + lista densa + barra inferior) y la presencia de Apple Music (pantalla completa con portada grande, fondo desenfocado y letras protagonistas).

### 3.2 Tokens (propuesta; se afinan en la Fase 3)

Parten de la paleta actual de `theme.rs`.

| Token | Valor | Uso |
|---|---|---|
| `bg_base` | `#0E0E1B` | Fondo base |
| `bg_dark` / `bg_sidebar` / `bg_card` | `#131322` / `#18182D` / `#222238` | Heredados |
| `accent_pink` | `#FF9EBD` | Acento principal |
| `accent_pink_hover` | `#FF7BA4` | Hover |
| `accent_lime` | `#FEF8C9` | CTA principal (Reproducir) |
| `accent_purple` | `#9D8EC4` | Secundario |
| `accent_sunset` (nuevo) | `#FFB36B` | Extremo cálido del degradado |
| `gradient_sunset` | rosa → naranja → púrpura | Hero, barras, barra de progreso |
| `glass_fill` | blanco @ 6–8 % | Superficie de vidrio |
| `glass_fill_strong` | `#131322` @ 55 % | Sidebar, barra inferior |
| `glass_border` | blanco @ 8 % + línea superior blanco @ 18 % | Borde con reflejo |
| `text_main` / `text_muted` | `#FFFFFF` / `#A4A4C4` | Texto |
| Radios | 8 / 12 / 16 / 24 / pill | Controles → paneles |
| Espaciado | escala de 4 px | Márgenes |
| Movimiento | hover 120 ms, transición de pantalla 250 ms | Curva ease-out |

**Acento dinámico:** se extrae el color dominante de la carátula actual y se mezcla con `accent_pink` (como hacen Apple Music y Spotify). Va en el fondo, en la barra de progreso y en el brillo de los botones.

### 3.3 Tipografía

- **GTAArtDeco (Condensed Bold / Regular):** logo, títulos de sección, "REPRODUCIENDO AHORA", encabezados de pantalla. No para texto corrido: a tamaños pequeños pierde legibilidad.
- **Fuente de interfaz sans (Inter o Noto Sans, a elegir en la Fase 3):** listas, metadatos, tooltips, letras. Incrustada con `include_bytes!` y con buena cobertura latina.
- Escala: 11 / 12 / 13 / 15 / 18 / 24 / 32 / 48.

### 3.4 Íconos

Íconos vectoriales en lugar de emojis (`egui-phosphor` o SVG con `egui_extras`; **verificar compatibilidad con la versión de egui en la Fase 3**). Todos los íconos de transporte, navegación y acciones se pasan a esa fuente. El emoji de cobertura queda solo como placeholder del arte.

### 3.5 Receta "liquid-glass" en egui

egui no tiene blur de fondo ni degradados nativos, así que el efecto se construye por capas:

1. **Fondo vivo:** la carátula actual, reducida a ~64 px y desenfocada en CPU (en un hilo de fondo), se estira a pantalla completa con filtrado lineal, saturada y con un scrim oscuro. Cambia con crossfade de 400 ms. Sin carátula: degradado de atardecer con manchas de color que se mueven despacio.
2. **Panel de vidrio:** relleno translúcido + trazo de 1 px + reflejo superior + sombra suave (`egui::Shadow`). Se implementa una sola vez en `ui/widgets/glass.rs`.
3. **Degradados** con `egui::Mesh` (colores por vértice): barras del visualizador, barra de progreso, botón principal.
4. **Brillo (glow)** en hover y en el elemento activo, con sombra tintada del color del acento.
5. **Grano** sutil opcional (textura repetida a ~3 % de opacidad) para quitar el banding de los degradados.

---

## 4. Estructura de la aplicación (inspirada en Spotify / Apple Music)

```
┌──────────────┬────────────────────────────────────────────────┬────────────┐
│ ◉ SIMPLE     │  ‹ ›   [ 🔍 Buscar canciones, artistas…  ]     │            │
│   PLAYER     ├────────────────────────────────────────────────┤  COLA /    │
│              │  ┌ hero: portada + degradado ────────────────┐ │  AHORA     │
│ BIBLIOTECA   │  │ Toda la música · 628 canciones · 41 h     │ │  (panel    │
│  ♪ Canciones │  │ [▶ Reproducir] [⤮ Aleatorio]              │ │  derecho   │
│  ◫ Álbumes   │  └───────────────────────────────────────────┘ │  opcional) │
│  ☺ Artistas  │  #  Título            Álbum      Duración      │            │
│              │  1  ▢ Canción ····    Álbum ···   3:21         │            │
│ CARPETA      │  …  (lista virtualizada, hover con ▶)          │            │
│  Cambiar…    │                                                │            │
├──────────────┴────────────────────────────────────────────────┴────────────┤
│ [portada] Título · Artista  │  ⤮  ⏮  ▶  ⏭  ⟲   0:42 ━━━●━━━ 3:10 │ NORM 🔊━● ☰ ⛶ │
└────────────────────────────────────────────────────────────────────────────┘
```

### Pantallas

| Pantalla | Contenido | Equivalente |
|---|---|---|
| **Canciones** | Hero con resumen + acciones; tabla virtualizada con número, miniatura, título/artista, álbum, duración; fila activa con ecualizador animado; hover muestra ▶. | Spotify "Tus canciones" |
| **Álbumes** (nueva) | Grilla de tarjetas (portada grande, título, artista); al abrir, lista de pistas. | Apple Music "Álbumes" |
| **Artistas** | Grilla circular con portada representativa; al abrir, discografía y canciones. | Spotify "Artistas" |
| **Cola** (panel derecho, toggle) | "Reproduciendo ahora" + próximas canciones. | Spotify "Cola" |
| **Barra inferior** | 3 columnas: info de pista · transporte y progreso · volumen, NORM, cola, letras, pantalla completa. Mini visualizador. | Spotify |
| **Pantalla completa "Ahora suena"** | Portada grande, fondo desenfocado, visualizador como capa de fondo o franja inferior, letras a la derecha. | Apple Music |
| **Letras** | Línea activa grande y nítida; inactivas atenuadas con fade por distancia; scroll suave animado; clic en una línea hace seek (si es sincronizada). | Apple Music |

**Nuevas funciones mínimas:** vista de Álbumes, panel de Cola, seek desde letras. Todo lo demás reutiliza `state.rs`. Antes de implementar hay que **verificar en `state.rs`** que exista la cola y los datos de álbum que necesitan estas vistas.

### Estados y detalles de UX

- Estados vacíos con ilustración y CTA (sin carpeta, sin resultados, sin letra).
- Esqueletos con shimmer mientras cargan las carátulas.
- Foco visible con teclado y contraste mínimo AA en texto.
- Atajos actuales conservados (Espacio, Alt+←/→, F6/F7/F8, teclas multimedia).
- Respeto del escalado HiDPI / fraccional.

---

## 5. Arquitectura propuesta

Se conserva la separación actual (lógica / UI). Cambios:

```
native/src/
├─ audio/
│  ├─ player.rs        # sin cambios grandes; spectrum reconfigurado
│  ├─ spectrum.rs      # produce SpectrumFrame{stream_time, bands} → ring buffer
│  └─ clock.rs         # (nuevo) PlaybackClock: posición estimada + resync
├─ viz/                # (nuevo) lógica pura del visualizador, testeable
│  ├─ engine.rs        # selección/interpolación de frames, re-binning log, tilt, AGC, suavizado
│  └─ mod.rs           # VizFrame { bars: [f32; N], peaks: [f32; N] }
├─ library_view.rs     # (nuevo) vista cacheada (orden/filtro/agrupación) con invalidación
├─ ui/
│  ├─ theme/           # tokens, tipografía, estilos, acento dinámico
│  ├─ widgets/         # glass, icon_button, pill_button, slider, card, cover, skeleton, chip
│  ├─ backdrop.rs      # fondo vivo (carátula desenfocada + degradado)
│  ├─ screens/         # songs, albums, artists, queue_panel, now_playing
│  ├─ shell/           # sidebar, topbar, player_bar
│  ├─ visualizers/     # bars, radial, glow (consumen VizFrame)
│  └─ textures.rs      # LRU + tamaños por uso
└─ perf.rs             # (nuevo) HUD de rendimiento oculto (F3)
```

### 5.1 Pipeline del visualizador (corrección de H1, H2, H4)

```
GStreamer streaming thread
  spectrum (bands≈1024, interval≈20–25 ms, message-magnitude)
        │ sync handler (siempre BusSyncReply::Pass)
        ▼
  SpectrumRing  Mutex<VecDeque<SpectrumFrame>>  (cap. ~64, el hilo de UI usa try_lock)
        │
        ▼  cada frame de UI
  PlaybackClock.now()  →  t_obj = now − latency_comp
        │
        ▼
  VizEngine: busca los frames que rodean t_obj → interpola →
             re-binning a ~48 barras en escala logarítmica →
             tilt (+3 dB/oct) + normalización adaptativa (AGC lento) →
             suavizado ataque rápido / caída lenta (por dt) + peak-hold
        ▼
  VizFrame → visualizers::{bars, radial, glow}
```

Decisiones técnicas:

- **Más resolución espectral.** Con `bands=32` las bandas son lineales en frecuencia y los graves quedan en 1–2 barras. Se sube a ~1024 bandas crudas (resolución ~21 Hz) y se agrupan en ~48 barras logarítmicas **en la UI**. El costo es mínimo (mensajes de ~4 KB a 40–50 Hz).
- **Alineación temporal.** Se usa `stream-time` del mensaje, comparado contra `PlaybackClock`.
  - `PlaybackClock` guarda `(posición base, Instant base)` e interpola con el reloj monotónico del sistema.
  - Se resincroniza con `player.position()` a ~5–10 Hz usando `try_lock`.
  - Se corrige tras `seek`, pausa y cambio de pista.
- **Compensación de latencia** (`latency_comp`): empieza como constante ajustable desde el HUD; si es viable, se obtiene consultando la latencia del sink.
- **Reposo.** En pausa o stop las barras decaen suavemente a 0 y el repintado continuo se detiene.
- **Sin fuente de datos.** Si falta el plugin `spectrum`, el visualizador se oculta y la app sigue funcionando.
- **Posición sin bloqueos.** Opcional (según Fase 0): un hilo "reloj" actualiza un `AtomicU64` con la posición a ~20 Hz y la UI solo lee el atómico.

### 5.2 Frame loop y trabajo por frame (H3, H4)

- Quitar `request_repaint()` incondicional. Política:
  - repintado continuo (`request_repaint()` a la tasa del monitor) **solo** si hay música sonando y se muestra el visualizador, letras con scroll animado o alguna animación activa;
  - en el resto de casos, `request_repaint_after(...)` o repintado por evento. Pausado y sin animaciones: ~0 fps y ~0 % de CPU.
- `LibraryView`: contiene la lista ordenada y filtrada como `Arc<[Song]>` o índices. Se recalcula solo cuando cambia la consulta, el orden, la pestaña o la versión de la biblioteca. Lo mismo para grupos de artistas y álbumes.
- La tabla ya no clona canciones por frame; recibe índices o referencias.

### 5.3 Texturas

- Tres tamaños por uso: **64 px** (filas), **256 px** (tarjetas), **512 px** (pantalla completa y fondo).
- Evicción LRU con tope de memoria (p. ej. 64 MB) y tope de cargas simultáneas.
- Se mantiene la decodificación en hilo de fondo. Se agrega priorización: primero lo visible.

---

## 6. Plan de implementación por fases

Cada fase termina en algo ejecutable y verificable. Las fases 1 y 2 **no dependen del diseño** y se pueden validar con la UI actual. Los tamaños son estimaciones relativas (S ≈ medio día, M ≈ 1–2 días, L ≈ 3+ días).

### Fase 0 — Preparación y línea base (S)

1. Crear la rama local desde `origin/nueva-version` (`git switch -c nueva-version --track origin/nueva-version`) y compilar en Linux.
2. **HUD de rendimiento** (`perf.rs`, F3, oculto por defecto) que reemplaza el botón "Grabar FPS". Métricas:
   - tiempo de frame de CPU (ms) y su p50 / p95 / p99 (no `unstable_dt`);
   - intervalo entre llegadas de mensajes de espectro y su jitter;
   - **edad del frame de espectro mostrado** (reloj de reproducción − `stream-time`);
   - porcentaje de frames donde el visualizador no cambió;
   - tiempo de espera de locks de audio.
3. **Línea base** con la UI actual: 60 s de reproducción con visualizador en pantalla completa, más una grabación de pantalla. Si es posible, medir también con **MangoHud** (frametime externo). La métrica interna ya falló una vez, así que la percepción del usuario cuenta como criterio.
4. **Matriz de experimentos** (para confirmar o descartar H1–H4 antes de arreglar nada):

   | Experimento | Qué prueba |
   |---|---|
   | Interpolar solo en la UI, sin otros cambios | H1 |
   | Limitar el repintado a 60 fps | H1 / H3 |
   | Cachear `sorted_songs` | H3 |
   | Leer posición cada 100 ms en lugar de cada frame | H4 |
   | Desactivar el dibujo del visualizador (datos activos) | descarta el costo de pintura |
   | `Renderer::Glow` vs `Renderer::Wgpu` (Vulkan) y modos de presentación | problemas de presentación en el driver |

5. Endurecer el arranque: hotkeys con degradación suave, ícono incrustado.

**Aceptación:** línea base documentada (números + video), hipótesis confirmadas o descartadas y priorizadas.

### Fase 1 — Visualizador fluido (M)

1. Reconfigurar `spectrum` (bandas altas, intervalo ~20–25 ms) y publicar `SpectrumFrame` con `stream-time` en el ring buffer.
2. `audio/clock.rs`: `PlaybackClock` con interpolación y resync (seek, pausa, cambio de pista).
3. `viz/engine.rs`: interpolación entre frames, re-binning logarítmico, tilt, AGC, suavizado y peak-hold.
4. Adaptar `bars`, `radial`, `glow` y el mini visualizador de la barra a `VizFrame`. Barras con degradado vertical con `Mesh`.
5. **Tests unitarios** (`cargo test`) de la lógica pura: interpolación con frames irregulares y huecos, re-binning, suavizado dependiente de `dt`, selección de frame tras seek, ring buffer lleno. Sin GStreamer ni GPU.
6. Ajuste fino de `latency_comp` con el HUD.

**Aceptación:** ver sección 7.

### Fase 2 — Costo por frame y robustez (M)

1. Política de repintado (5.2) y `LibraryView` (5.2).
2. Reloj de posición sin bloqueo (atómico) si la Fase 0 lo justifica.
3. Texturas: tamaños por uso + LRU + priorización (5.3).
4. Medir de nuevo con el HUD y comparar contra la línea base.

**Aceptación:** reposo ≈ 0 % de CPU en pausa; scroll de 628 canciones sin picos de frame; memoria de texturas acotada.

### Fase 3 — Sistema de diseño (L)

1. `ui/theme/`: tokens, `Visuals`/`Style` globales, escala tipográfica, fuentes (GTAArtDeco para marca + sans de interfaz).
2. Íconos vectoriales y migración de los emojis de controles.
3. `ui/widgets/`: `glass_panel`, `icon_button` (hover/pressed animados), `pill_button`, `slider` custom (seek y volumen: pista con degradado, thumb que aparece en hover, tooltip de tiempo), `card` (hover con elevación), `cover` (sombra, esquinas, esqueleto), `chip`.
4. `ui/backdrop.rs`: fondo vivo (carátula desenfocada + crossfade + degradado) y extracción de color dominante para el acento dinámico.
5. Pantalla "galería" de desarrollo (oculta, con un flag) con todos los widgets y estados, para revisar el diseño sin navegar la app.

**Aceptación:** galería revisada y aprobada por el usuario antes de pasar a la Fase 4.

### Fase 4 — Shell y pantallas principales (L)

1. `shell/`: sidebar (logo, secciones, estado activo con píldora, acciones de carpeta), topbar (navegación, búsqueda con atajo `/` o Ctrl+K, estado de escaneo), barra inferior de 3 columnas.
2. `screens/songs`: hero + tabla virtualizada con columnas adaptables, fila activa con ecualizador animado y hover con ▶.
3. `screens/artists` (grilla circular) y `screens/albums` (nueva, grilla + detalle).
4. `screens/queue_panel`: panel derecho con toggle.
5. Estados vacíos, carga (esqueletos) y errores.

**Aceptación:** navegación completa sin regresiones funcionales (búsqueda, orden, shuffle, repeat, cola, selección de artista).

### Fase 5 — "Ahora suena" y letras (M)

1. Pantalla completa: portada grande con sombra, título/artista/álbum, fondo desenfocado, visualizador como capa (modo configurable), controles ampliados.
2. Letras estilo Apple Music: línea activa escalada y nítida, resto atenuado según distancia, scroll suave con interpolación, clic para hacer seek en letras sincronizadas, fallback para letras sin tiempos.
3. Transición animada entre vista normal y pantalla completa.

**Aceptación:** letras sin saltos bruscos de scroll; visualizador dentro de los criterios de la sección 7 en pantalla completa.

### Fase 6 — Pulido y extras (M)

1. Animaciones finas (hover, pulso del botón de reproducir, transiciones de pantalla).
2. Accesibilidad: foco visible, contraste, navegación por teclado, atajos nuevos.
3. Verificar HiDPI y escalado fraccional en Wayland y X11.
4. **Opcional:** blur real de fondo con un `egui_wgpu` paint callback (solo si el fondo falso no convence y el backend wgpu resultó estable en la Fase 0).
5. Revisar consumo de CPU/GPU en uso normal.

### Fase 7 — Empaquetado y limpieza (M)

Continúa el plan existente en `PLAN_SIGUIENTE_SESION.md`:

1. Adaptar `scripts/build-appimage.sh` y `PKGBUILD` a un binario Cargo (sin Node/Tauri). Revisar dependencias de ejecución (GStreamer + plugins, fuentes) y que `eframe` incluya soporte Wayland y X11 (**verificar features**).
2. Eliminar `src/`, `src-tauri/`, `package*.json`, `vite.config.ts`, `tsconfig*.json`, `index.html`, `public/` y los restos de Tauri.
3. Aplanar: mover `native/` a la raíz.
4. Actualizar `README.md` (hoy describe Tauri/React) y borrar `Detalles.md` o archivarlo.

---

## 7. Criterios de aceptación del visualizador

La métrica interna ya engañó una vez, así que se exige **medición y validación visual**.

| Criterio | Meta |
|---|---|
| Percepción | El usuario valida a ojo, en pantalla completa y en la barra inferior, sin "escalones" ni congelamientos, durante canciones de géneros distintos. |
| Tiempo de frame (CPU) | p99 < 1.5 × intervalo de vsync durante 60 s de reproducción con visualizador en pantalla completa. |
| Espectro | Jitter del intervalo de llegada (p95) < 10 ms; edad del frame mostrado estable (variación < 1 frame de espectro). |
| Sincronía | Los picos del visualizador coinciden con los golpes audibles (verificación manual con pistas percusivas). |
| Reposo | En pausa y sin animaciones, ~0 % de CPU y sin repintado continuo. |
| Scroll | Scroll agresivo por 628 canciones sin picos visibles de frame. |
| Externo | Frametime de MangoHud sin picos periódicos (si está disponible). |

---

## 8. Estrategia de pruebas

- **Unitarias (`cargo test`)**: `viz/engine`, `PlaybackClock`, ring buffer, `LibraryView` (invalidación y orden), extracción de color dominante, LRU de texturas. No requieren GPU ni audio.
- **Manuales por fase**: lista de comprobación (reproducir, pausar, seek, cambio de pista, shuffle/repeat, búsqueda, cambio de carpeta, pantalla completa, letras, MPRIS, atajos globales, segunda instancia).
- **Rendimiento**: protocolo de la sección 7 repetido al final de cada fase, comparando con la línea base de la Fase 0.
- **Entornos**: Wayland y X11; escala 100 % y fraccional; con y sin el plugin `spectrum`; con y sin atajos globales disponibles.

---

## 9. Riesgos

| Riesgo | Mitigación |
|---|---|
| Las hipótesis no son la causa real | Fase 0 con experimentos A/B antes de arreglar; el HUD mide edad del frame y jitter, no solo FPS. |
| El driver / compositor genera tirones aunque la app esté bien | Probar `Renderer::Wgpu` y modos de presentación; medir con MangoHud. |
| `stream-time` no se alinea tras `seek` o cambio de pista | `PlaybackClock` con reset explícito en esos eventos; pruebas unitarias dedicadas. |
| El efecto de vidrio sin blur real se ve plano | Fondo desenfocado + capas (sección 3.5); blur real por wgpu como extra de la Fase 6. |
| Texturas y listas grandes | LRU con tope, tamaños por uso, virtualización (ya existe). |
| Alcance de UI crece | Fases cerradas con criterios; la galería de widgets se aprueba antes de las pantallas. |
| Actualizar egui (0.29) | Decisión abierta (sección 10); no mezclar el salto de versión con el rediseño. |

---

## 10. Decisiones abiertas

1. **Versión de egui/eframe:** quedarse en 0.29 o subir antes de la Fase 3 (por nuevas APIs y wgpu). Revisar el changelog; no se asume nada todavía.
2. **Íconos:** `egui-phosphor` frente a SVG propios con `egui_extras`. Depende de la compatibilidad de versión.
3. **Fuente de interfaz sans:** Inter o Noto Sans.
4. **Renderer por defecto:** Glow (actual) o Wgpu, según los resultados de la Fase 0.
5. **Mockups:** el diseño visual se valida en la galería de la Fase 3 y no con mockups previos. Si se prefiere ver mockups HTML antes de escribir código, se pueden generar aparte.
6. **Dónde vive este documento:** hoy es un archivo sin versionar en `master`. Al empezar a implementar debe moverse a `nueva-version`.

---

## 11. Orden recomendado y dependencias

```
Fase 0 ──► Fase 1 ──► Fase 2 ──► Fase 3 ──► Fase 4 ──► Fase 5 ──► Fase 6 ──► Fase 7
(base)    (viz)      (costo)    (diseño)   (pantallas) (full+letras) (pulido)  (empaque)
```

- Fases 0–2 resuelven el bug y la base de rendimiento **sin tocar el aspecto**, así que se pueden validar rápido.
- Fases 3–6 son el rediseño. La 3 bloquea a la 4 y a la 5.
- La 7 va al final, cuando la versión nativa reemplace por completo a la de Tauri.
