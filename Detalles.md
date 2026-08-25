# Análisis técnico — Simple Player

Repositorio analizado: `https://github.com/akilex02/simple-player`
Stack actual: Tauri v2 + React + GStreamer (`gstreamer-player`) + `lofty` + `souvlaki`/MPRIS.

---

## 1. Problema de rendimiento en Wayland

**Ubicación:** `src-tauri/src/lib.rs`, función `run()`, línea ~330.

**Código actual:**

```rust
// Fix WebKitGTK Wayland protocol crash (Error 71 dispatching to Wayland display)
if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
}
if std::env::var("GDK_BACKEND").is_err() {
    std::env::set_var("GDK_BACKEND", "x11");
}
```

**Diagnóstico:**
Se fuerza `GDK_BACKEND=x11` de forma incondicional, para todos los usuarios, sin detectar si realmente hace falta. Esto obliga a WebKitGTK a renderizar vía **XWayland** en cualquier sesión Wayland, lo que provoca:

- Tearing (sin VSync correcto).
- Mayor uso de CPU/GPU por la capa de traducción de protocolo.
- Escalado fraccional roto/borroso en pantallas HiDPI.
- Pérdida de integración nativa con el compositor (gestos, fricción de ventanas, etc.).

El motivo original de este workaround es casi seguro el conocido **"Error 71 dispatching to Wayland display"**, un crash de WebKitGTK relacionado con su renderer DMA-BUF chocando con ciertos drivers (Nvidia propietario, combinaciones específicas de Mesa/compositor).

**Recomendaciones (de menos a más agresivo):**

1. Probar primero una corrección más quirúrgica, que desactiva solo el renderer problemático sin sacrificar Wayland nativo:
   ```rust
   if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
       std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
   }
   ```
2. Aplicar el workaround **condicionalmente**, detectando en runtime si la sesión es Wayland, en vez de forzarlo siempre:
   ```rust
   let is_wayland = std::env::var("WAYLAND_DISPLAY").is_ok();
   if is_wayland {
       std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
   }
   ```
3. Dejar `GDK_BACKEND=x11` como **último recurso**, solo si el fix del dmabuf no resuelve el crash en un hardware específico (idealmente detectado por vendor de GPU, no aplicado globalmente).
4. Revisar la versión de `webkit2gtk` instalada — versiones recientes han corregido en gran parte el Error 71; documentarla como dependencia mínima en el README puede evitar que usuarios con versiones viejas topen con el bug.

---

## 2. Visualizador de espectro / barras / "pulso de fuego"

**Hallazgo clave:** No hace falta añadir `rustfft` ni manipular buffers PCM manualmente. GStreamer ya trae el elemento `spectrum` (parte de `gst-plugins-good`, que el propio README ya lista como dependencia), que calcula la FFT de forma nativa y optimizada, y publica las magnitudes por banda como mensajes en el bus.

**Punto de integración:** el código ya usa `player.pipeline()` para configurar el sink de audio (línea ~375 de `lib.rs`), así que el pipeline ya está expuesto y es el mismo punto donde insertar el filtro de espectro.

**Pasos propuestos:**

1. Crear el elemento `spectrum` e insertarlo vía la propiedad `audio-filter` de `playbin`:
   ```rust
   use gstreamer::prelude::*;

   let spectrum = gst::ElementFactory::make("spectrum")
       .property("bands", 64u32)
       .property("threshold", -80i32)
       .property("interval", 33_333_333u64) // ~30fps en nanosegundos
       .build()
       .expect("no se pudo crear el elemento spectrum");

   pipeline.set_property("audio-filter", &spectrum);
   ```

2. Escuchar el bus para los mensajes de tipo `"spectrum"` y emitir las magnitudes al frontend como evento (push, no polling):
   ```rust
   let bus = pipeline.bus().unwrap();
   let app_handle = handle.clone();
   bus.add_watch(move |_, msg| {
       if let gst::MessageView::Element(el) = msg.view() {
           if let Some(s) = el.structure() {
               if s.name() == "spectrum" {
                   if let Ok(magnitudes) = s.get::<Vec<f32>>("magnitude") {
                       let _ = app_handle.emit("spectrum-data", magnitudes);
                   }
               }
           }
       }
       glib::ControlFlow::Continue
   }).unwrap();
   ```

3. En el frontend, escuchar el evento (en vez de `setInterval` + `invoke`) y dibujar en `<canvas>` con `requestAnimationFrame`, guardando el último array recibido en un `ref` (no en `state`) para evitar re-renders de React 30 veces por segundo:
   ```tsx
   useEffect(() => {
     const unlisten = listen<number[]>("spectrum-data", (event) => {
       latestMagnitudes.current = event.payload;
     });
     return () => { unlisten.then(f => f()); };
   }, []);
   ```

4. Para las barras: gradiente/altura proporcional a cada magnitud de banda.
   Para el efecto "pulso de fuego": gradiente vertical (naranja → amarillo → transparente) + buffer de "calor" que se desplaza hacia arriba cada frame, sumando calor en la base proporcional a la magnitud — un autómata celular simple, barato en canvas 2D.

---

## 3. Letras sincronizadas

**Estado actual:** No hay implementación de letras ni de LRC en el repo. La posición de reproducción se obtiene por polling desde el frontend:

**Ubicación:** `src/App.tsx`, líneas ~244-262.

```tsx
useEffect(() => {
  let interval: ReturnType<typeof setInterval>;
  if (isPlaying && currentSongIndex !== null) {
    interval = setInterval(async () => {
      if (!isDraggingSeek) {
        try {
          const pos = await invoke<number>("get_position");
          setCurrentTime(pos);
          ...
        } catch (_) { }
      }
    }, 500);
  }
  return () => clearInterval(interval);
}, [isPlaying, currentSongIndex, activeQueue, repeatMode, isShuffle, isDraggingSeek]);
```

**Diagnóstico:** un intervalo de 500ms es aceptable para la barra de progreso, pero se sentirá "a saltos" para resaltar líneas de letra en tiempo real.

**Recomendaciones:**

1. **Backend:** reutilizar el mismo `bus.add_watch` propuesto para el espectro (o un timer independiente) para emitir la posición cada ~100-150ms vía evento (`app.emit("position-update", pos)`), en lugar de que el frontend la pida por polling con `invoke`.
2. **Frontend:** interpolar entre actualizaciones con `requestAnimationFrame`, para que el resaltado de la línea activa se vea fluido aunque el dato de posición llegue cada 100-150ms.
3. **Parseo LRC:** formato estándar `[mm:ss.xx] texto`. Se puede parsear en Rust (~40 líneas) o directamente en TypeScript en el frontend — dado que ya hay lógica de UI en React, es razonable resolver el parseo ahí y dejar Rust enfocado en audio/sistema.
4. Si se quiere sincronización más fina (palabra por palabra, estilo Apple Music/Spotify), evaluar el formato LRC enhanced (`<mm:ss.xx>palabra`), aunque muchas fuentes de letras no lo proveen.

---

## 4. Resumen de cambios propuestos

| Área | Archivo afectado | Cambio |
|---|---|---|
| Wayland/rendimiento | `src-tauri/src/lib.rs` (~L326-332) | Reemplazar `GDK_BACKEND=x11` forzado por `WEBKIT_DISABLE_DMABUF_RENDERER=1` condicional a detección de Wayland |
| Visualizador | `src-tauri/src/lib.rs` (cerca de configuración del pipeline, ~L375) | Insertar elemento `spectrum` vía `audio-filter` + bus watch + `emit` a frontend |
| Visualizador | `src/App.tsx` / nuevo componente | Listener de evento `spectrum-data` + render en `<canvas>` con `requestAnimationFrame` |
| Letras | `src-tauri/src/lib.rs` | Emitir posición vía evento cada 100-150ms en vez de exponer solo `get_position` por invoke |
| Letras | `src/App.tsx` / nuevo componente | Parser LRC + interpolación de posición con `requestAnimationFrame` |