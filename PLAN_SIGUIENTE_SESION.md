# Plan para la siguiente sesión — Simple Player (migración a nativo)

## Contexto

Este proyecto se está migrando de **Tauri + React + WebKitGTK** a un binario
nativo en Rust con **eframe/egui** (vive en `native/`, sin tocar `src/` ni
`src-tauri/` todavía — ambas versiones coexisten en el repo mientras se
termina la migración).

**Motivo de la migración:** WebKitGTK en esta máquina no puede usar
aceleración por hardware (falla al asignar buffers GBM/DMA-BUF — un
mecanismo específico de su arquitectura multi-proceso para compartir
superficies renderizadas entre su proceso de UI y su proceso de contenido
web). Confirmamos con un prototipo `eframe` mínimo que una app de un solo
proceso corre a 180fps limpios en esta misma máquina sin ningún workaround,
lo que justificó la reescritura completa.

**Commits relevantes** (orden cronológico, todos en `master`):
1. `30bdd52` — Fase 0-1: andamiaje del crate, backend Rust portado (GStreamer, MPRIS2, atajos globales, biblioteca, letras).
2. `57b3848` — Fase 2: puerto de la lógica de `App.tsx` a `state.rs`.
3. `c047af3` — Fase 3: UI real (sidebar, header, tabla de canciones con `ScrollArea::show_rows`, grilla de artistas).
4. `e6f4adf` — Fase 4: controles compartidos (transporte/volumen/progreso).
5. `56d748b` — Fase 5: pantalla completa + letras sincronizadas con auto-scroll.
6. `9b4315e` — Fase 6: los 3 visualizadores (barras/radial/resplandor) en `egui::Painter`.
7. `ed00fac` — Carátulas + fixes de rendimiento + herramienta de medición de FPS. **Este commit deja un bug de rendimiento confirmado sin resolver — es el punto de partida de esta sesión.**

## Estado funcional actual

La app nativa (`native/`) ya cubre **todo** lo que hacía la versión Tauri:
biblioteca, búsqueda con scoring, sort, shuffle, cola de reproducción,
repeat, MPRIS2, atajos globales (F6/F7/F8 + teclas multimedia), pantalla
completa, letras sincronizadas (LRC + embebidas), y los 3 visualizadores de
espectro reaccionando al audio en tiempo real. Todo validado
interactivamente en cada fase.

**Lo que falta:**
1. **Resolver el bug de rendimiento de las carátulas (prioridad inmediata, ver abajo).**
2. Pulido visual fino (gradientes en botones, animaciones de hover, glow) — quedó básico comparado con el CSS original; se decidió priorizar la fidelidad alta pero no se llegó a implementar en detalle.
3. Fase 7 (empaquetado): adaptar `scripts/build-appimage.sh` y `PKGBUILD` a un binario Cargo plano, sin Node/Tauri.
4. Decidir qué hacer con `src/`, `src-tauri/`, `package.json`, `vite.config.ts`, etc. una vez que la versión nativa esté 100% validada (moverlos fuera del repo o eliminarlos).

---

## 🔴 Bug confirmado: las carátulas causan stutter severo de scroll

### Cómo se confirmó

1. Se agregó una herramienta de medición de FPS en la propia app (botón "⏺ Grabar FPS" visible en la esquina superior izquierda de la ventana, ver `native/src/main.rs`): mientras está grabando, guarda el fps instantáneo de cada frame (`1.0 / ctx.input(|i| i.unstable_dt)`) en un `Vec<f64>`, y al detener calcula promedio, percentiles (p50/p25/p10/p5/p1) y conteo de frames por debajo de 30fps/10fps.

2. **Primera medición** (con el fix de presupuesto de texturas ya aplicado — ver abajo): al hacer scroll agresivo por las 628 canciones, la herramienta reportó **1346 frames, promedio 179fps, p50 180, p25 178, p10 166, p5 160, p1 2, mínimo 0 | <30fps: 17 (1%) <10fps: 17 (1%)**.

3. **El usuario reportó que esto no coincide con lo que vio**: percibió tirones prolongados e intermitencia real, básicamente 0fps en varios momentos — no un problema aislado del 1% de los frames.

4. **Esto revela que la métrica de FPS por `unstable_dt` (tiempo entre llamadas a `update()`) NO está capturando el problema real.** Es un hallazgo importante en sí mismo: hay una desconexión entre "qué tan rápido corre la lógica interna de egui" y "qué tan fluido se ve realmente en pantalla" — muy probablemente relacionado con el mismo tipo de problema de GPU/driver que ya vimos con WebKitGTK en esta máquina (los fallos de GBM/DMA-BUF), ahora manifestándose de otra forma con la subida de texturas vía OpenGL/`glow` (el backend de `eframe`).

5. **Experimento decisivo**: se deshabilitó temporalmente la carga de carátulas en `native/src/ui/song_table.rs` (se reemplazó la llamada a `super::cover_thumb(...)` por un cuadrito de color plano, sin decodificar ni subir ninguna textura). Con esto, **el usuario confirmó que el scroll quedó instantáneo/perfectamente fluido.** Esto aísla la causa de forma concluyente: **es la carga/dibujo de carátulas, no otra cosa** (no es `ScrollArea`, no es el polling de posición de audio, no es la lógica de estado).

### Lo que ya se intentó (y no fue suficiente)

En `native/src/ui/textures.rs` ya existe un `TextureCache` con:
- Caché por ruta (`HashMap<String, egui::TextureHandle>`) — evita redecodificar/resubir una textura ya cargada.
- **Presupuesto de cargas nuevas por frame** (`begin_frame(budget)`, llamado con `budget=3` desde `main.rs::update()`): limita cuántas texturas *nuevas* se pueden decodificar+subir a GPU en un mismo frame, para que un scroll rápido no intente cargar decenas de golpe.
- Downscale a 256×256 máx (`image::imageops::FilterType::Triangle`) antes de crear la textura, para no subir imágenes embebidas de alta resolución sin necesidad.

Esto **mejoró** la situación (antes de este fix, el usuario reportó caídas a ~1fps sostenidas; después, la métrica por dt ya no las mostraba) — pero el experimento de desactivar carátulas por completo demuestra que **sigue habiendo un problema real que el presupuesto por frame no resolvió del todo**, y que la métrica de FPS no está viendo.

### Hipótesis para investigar (en orden de probabilidad)

1. **El costo real no es solo "decodificar la primera vez", sino "dibujar N texturas distintas cada frame" en este GPU/driver.** Con ~15-20 filas visibles simultáneamente, cada una con una textura distinta, eso son 15-20 *bind*/draw calls de textura por frame. Si el stack de OpenGL/Mesa de esta máquina tiene problemas con bindings de textura frecuentes (consistente con los problemas de GBM/DMA-BUF que ya vimos con WebKit todo el resto de la sesión), esto podría causar *stalls* en el driver que no se reflejan en `unstable_dt` si egui mide el tiempo antes de esperar al GPU (falta de sincronización correcta entre CPU y GPU en la medición).
   - **Cómo probarlo:** reducir drásticamente cuántas texturas distintas están visibles a la vez (por ejemplo, limitar `ROW_HEIGHT`/viewport para que solo haya 2-3 filas visibles) y ver si el stutter escala con la cantidad de texturas simultáneas en pantalla.

2. **La decodificación/resize en el hilo de UI sigue siendo demasiado costosa incluso con presupuesto de 3/frame**, si cada `image::open(...).resize(...)` tarda más de ~5-10ms (plausible para JPEGs de portada de álbum de varios cientos de KB a 1MB+). Con budget=3, eso son hasta 15-30ms añadidos a ciertos frames — pero la métrica de dt *debería* haber capturado esto como frames lentos, y el conteo de `<30fps` fue solo 1%, así que esta hipótesis por sí sola no explica el "0fps percibido".

3. **Verdadera solución de fondo (recomendada para probar primero):** mover la decodificación+resize a un **hilo de fondo dedicado** (o un pool pequeño), y que el hilo principal de egui solo haga `ctx.load_texture(...)` (la subida a GPU, que sí debe pasar en el hilo con el contexto GL) cuando el hilo de fondo ya entregó el buffer RGBA decodificado vía un canal `mpsc`. Esto saca por completo el costo de I/O+decodificación del camino crítico de cada frame, dejando solo la subida a GPU (que en teoría debería ser rápida) en el hilo principal. Si el stutter persiste incluso así, confirma que el cuello de botella real está en la subida a GPU / el driver, no en la decodificación — apuntando de vuelta a la Hipótesis 1.

4. **Alternativa/complementaria:** en vez de una textura por canción, usar un **atlas de texturas** (todas las miniaturas de portada empaquetadas en una sola textura grande, referenciadas por UV) para que dibujar la lista completa sea un solo *bind* de textura en vez de N — mitigaría directamente la Hipótesis 1 si resulta ser la causa. Es más trabajo de implementación; solo vale la pena si la Hipótesis 1 se confirma.

### Archivos relevantes para esta investigación

- `native/src/ui/textures.rs` — `TextureCache`, el presupuesto por frame, el resize a 256×256.
- `native/src/ui/mod.rs` — función `cover_thumb(...)`, el punto único donde se pide/dibuja cualquier carátula (usado por `song_table.rs`, `player_bar.rs`, `artists_grid.rs`, `fullscreen.rs`).
- `native/src/ui/song_table.rs` — donde se hizo el experimento de desactivar carátulas (línea ~79, la llamada a `super::cover_thumb(...)` dentro del loop de `show_rows`). **Ya está revertida a su estado normal (carátulas activas) en el commit `ed00fac`** — si se quiere repetir el experimento, comentar esa línea y reemplazar por un `egui::Frame` de color plano, tal como se hizo durante el diagnóstico.
- `native/src/main.rs` — el botón "Grabar FPS" y el cálculo de percentiles, para volver a medir después de cada intento de fix.

### Cómo retomar la medición

1. Compilar y correr: `cargo build --manifest-path native/Cargo.toml && ./native/target/debug/simple-player` (ejecutar desde dentro de `native/` para que las rutas relativas a `assets/` funcionen).
2. **Importante:** si hay otra instancia de la app (Tauri vieja, AppImage instalada, o una instancia previa del binario nativo) corriendo, la nueva fallará al registrar los atajos globales (`X_GrabKey BadAccess`) y el proceso morirá. Verificar con `ps aux | grep simple-player` antes de lanzar.
3. En la ventana, botón "⏺ Grabar FPS" → hacer scroll agresivo por toda la lista → "⏹ Detener grabación" → leer el resumen en pantalla (también se loguea a stdout con el prefijo `[perf]`).
4. **No confiar solo en el número agregado** — como ya vimos, el promedio/percentiles por `unstable_dt` pueden verse bien mientras la experiencia real es mala. Siempre correlacionar con lo que el usuario percibe visualmente, no solo con la métrica.

---

## Fase 7 (pendiente, después de resolver el bug de arriba)

Una vez resuelto el problema de rendimiento y con el pulido visual a un
nivel aceptable:

1. Adaptar `scripts/build-appimage.sh`: hoy hace `npx tauri build --no-bundle`
   y empaqueta el binario de `src-tauri/target/release/`. Cambiar a
   `cargo build --release --manifest-path native/Cargo.toml` y empaquetar
   `native/target/release/simple-player` + `native/assets/` en el AppDir.
2. Adaptar `PKGBUILD`: quitar la dependencia de Node/npm del proceso de build,
   compilar solo con Cargo.
3. Eliminar (o mover fuera del repo) `src/`, `src-tauri/`, `package.json`,
   `vite.config.ts`, `tsconfig*.json`, `index.html`, `public/`, y las
   `capabilities/`/`tauri.conf.json` de Tauri — una vez confirmado que la
   versión nativa reemplaza por completo a la anterior.
4. Considerar mover `native/` a la raíz del repo (aplanar la estructura)
   ahora que ya no coexiste con la versión Tauri.
