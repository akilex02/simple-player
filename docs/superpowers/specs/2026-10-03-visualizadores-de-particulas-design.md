# Simple Player — Visualizadores de partículas

> **Estado:** implementado en `nueva-version`; falta la aceptación manual del usuario (criterios 3 y 4 con la app normal, mouse y música reales).
> **Rama de trabajo:** `nueva-version`.
> **Fecha:** 2026-10-03
> **Referencia:** `ascii-particules/src/bin/particles.rs` (proyecto del mismo autor, macroquad).

---

## 1. Objetivo y alcance

**Qué se quiere:** ampliar los visualizadores de la pantalla completa "Ahora suena" con efectos de partículas reactivos al audio, tomados del simulador de `ascii-particules`, y conservar su interacción con el mouse.

**Modos nuevos**

| Modo | Etiqueta | Descripción |
|---|---|---|
| Anillo | `Anillo` | Reemplaza al Radial. Partículas sobre un aro centrado en la portada; late con los graves, se ondula con los medios y chispea con los agudos. Resorte elástico. |
| Partículas | `Partículas` | Campo libre de puntos que derivan, tiemblan y centellean con el audio. |
| Constelación | `Constelación` | Campo libre que une con líneas a las partículas cercanas. |
| Osciloscopio | `Osciloscopio` | Línea horizontal de partículas que modela una onda reactiva. Resorte elástico. |

**Ciclo de modos:** Barras → Anillo → Partículas → Constelación → Osciloscopio → Franja → Apagado → Barras.

**Fuera de alcance (YAGNI)**

- Las paletas neón del original (el color sale del acento dinámico de la portada).
- El HUD, la captura de audio con `cpal` y los filtros IIR del original.
- Las teclas `1–4` (conteo de partículas), `R` (reiniciar) y las flechas de gravedad/fricción: el conteo y las constantes son fijos.
- Persistir el modo elegido entre sesiones.
- Los visualizadores de la barra inferior (siguen siendo las barras de siempre).

---

## 2. Decisiones

- **Fuente de audio:** el `VizFrame` actual (barras 0..1 ya rebinneadas en escala log, alineadas con el reloj). No se agrega ninguna captura nueva.
- **Mouse (decidido por el usuario):** interacción completa en pantalla completa. Clic izquierdo mantenido atrae las partículas al cursor; **`G`** alterna atraer/repeler (Espacio ya es reproducir/pausar en toda la app y no se toca).
- **Anillo reemplaza a Radial:** el centro y el radio base se toman de la portada, como ya hace el Radial.
- **Presupuesto:** los topes de partículas son valores iniciales; se miden con `--bench` y se bajan si el modo rebasa el límite de CPU vigente (guard de 250 ms del ledger de la fase 7 y ~0.2 % de CPU en reposo).

---

## 3. Arquitectura

### 3.1 `src/viz/bands.rs` — lógica pura

```rust
pub struct Bands { pub bass: f32, pub mid: f32, pub high: f32 }   // 0..1

pub struct BandAnalyzer { /* estado suavizado */ }
impl BandAnalyzer {
    pub fn new() -> Self;
    pub fn update(&mut self, bars: &[f32], dt: f32) -> Bands;
}
```

- Dado `bars` (n barras), `bass` = media de las primeras `round(0.12·n)` barras, `mid` = media de las siguientes `round(0.45·n)`, `high` = media del resto. Un `bars` vacío da `0, 0, 0`.
- Cada banda se suaviza con `engine::smooth` (ataque rápido, caída más lenta) para evitar parpadeo; constantes `ATTACK_TAU = 0.04 s`, `RELEASE_TAU = 0.18 s`.
- Sin dependencias de egui ni de GStreamer.

### 3.2 `src/viz/particles.rs` — física pura

```rust
pub enum FieldKind { Free, Ring, Wave }

pub struct Pull { pub pos: (f32, f32), pub attract: bool }

pub struct ParticleField { /* partículas + rng con semilla */ }
impl ParticleField {
    pub fn new(kind: FieldKind, count: usize, bounds: Rect2, seed: u64) -> Self;
    pub fn resize(&mut self, bounds: Rect2);              // reubica sin reiniciar el estado
    pub fn step(&mut self, dt: f32, bands: Bands, anchor: Anchor, pull: Option<Pull>, time: f32);
    pub fn particles(&self) -> &[Particle];
}
pub struct Particle { pub pos: (f32, f32), pub vel: (f32, f32), pub home: (f32, f32), pub size: f32, pub tint: f32 }
```

- `Rect2`/`Anchor` son tipos simples propios (`Anchor` = centro y radio de la portada para el Anillo); la capa de UI los convierte desde `egui::Rect`. No hay dependencia de egui.
- `dt` se normaliza a pasos de 1/60 s (`k = dt·60`, con tope de 3) para que la física sea igual a cualquier frecuencia de repintado.
- Generador pseudoaleatorio propio (xorshift) con semilla: las pruebas son deterministas.

**Comportamiento por tipo** (constantes tomadas del original, expresadas en puntos lógicos de egui)

- **Free:** integra velocidad; fricción 0.98 por paso; con los graves (> 0.1) agrega deriva aleatoria `bass·0.015`; rebote en los bordes con pérdida 0.7.
- **Ring:** destino sobre un círculo, `home = center + (cosθ, sinθ)·r`, con `r = base + bass·B + onda + chispa`, donde la onda es `cos(6θ − 6t)·mid·W` y la chispa solo cada 11.ª partícula (`high·S`). `base` se calcula a partir del radio de la portada. Resorte hacia `home` con `k = 0.055`.
- **Wave:** destino `x = i/n·ancho`, `y = centro + (sin(3τ·x + 5t) + 0.25·cos(10τ·x − 10t))·amp`, con `amp = 30 + mid·14 + bass·9` (escalado al alto de la ventana) y jitter de agudos en cada 7.ª partícula. Resorte con `k = 0.055`.
- **Todos:** los medios sacuden la posición (`mid·0.45`) y los agudos agregan jitter de velocidad (`high·0.12`) cuando superan 0.05.
- **Atracción:** con `Pull`, fuerza `±g/max(dist, 25)` hacia el cursor (`g = 1.5` (el original usaba 0.5 con teclas de ajuste y a ese valor casi no se nota)); los graves aportan un empuje radial opuesto (`−bass·0.035`). En Ring y Wave el resorte baja a `k = 0.005` mientras hay `Pull` (se siente elástico) y vuelve a `0.055` al soltar.

**Constelación (enlaces):** función pura `links(particles, max_dist, cap) -> Vec<(u32, u32, f32)>` con rejilla espacial (celdas de lado `max_dist`); solo mira celdas vecinas, devuelve a lo sumo `cap` enlaces con su opacidad `(1 − d/max_dist)·0.35`. `max_dist = 45 + bass·6` (el original usaba 45 + bass·2 en unidades distintas).

### 3.3 `src/ui/visualizers/particles.rs` — dibujo

- Recibe `&ParticleField`, el acento y la opacidad; arma **un solo `egui::Mesh`** con un quad por punto (y un quad delgado por enlace en Constelación) para no pagar una llamada de pintura por partícula.
- Color: el acento dinámico, con variación de brillo por `tint` y alfa fija (≈ 0.8 para puntos).
- Anillo: además de los puntos, une partículas contiguas con un trazo del grosor de las del original (2.5 px).
- Osciloscopio: une las contiguas con el mismo trazo.
- Indicador del cursor: círculo translúcido (verde atraer, rojo repeler) que pulsa con los graves, solo mientras se mantiene el clic.

### 3.4 Conexión

- `VisualizerMode` pasa a `{ Bars, Ring, Particles, Constellation, Wave, Strip, Off }`; `next()` y `label()` reflejan el ciclo y las etiquetas de la sección 1. `--viz` busca por etiqueta (ya sin distinguir mayúsculas; se aceptan también los nombres sin acento).
- `FullscreenView` guarda un `BandAnalyzer`, un `Option<ParticleField>` y el estado `attract: bool`. El campo se crea al entrar a un modo de partículas y se descarta al salir; se redimensiona (`resize`) cuando cambia el tamaño de la ventana.
- Cada frame: `bands = analyzer.update(&viz.bars, dt)`; si hay música sonando, `field.step(...)`; el dibujo ocurre en la capa del visualizador (antes del contenido), como el resto de los modos.
- El `Radial` se elimina (`radial.rs`); su lógica de "centrado en la portada" se conserva en el cálculo de `Anchor`.

### 3.5 Mouse

- Se hace sobre el fondo de la pantalla completa. Se considera "sobre un control" (y no hay atracción) si el puntero está sobre la portada, el chip de controles, el grupo de volumen, la barra superior, el rectángulo de las letras (cuando se muestran) o el área de información. Los rectángulos se obtienen de `geometry` (portada, controles) más los de volumen y barra superior.
- `Pull` existe mientras el botón izquierdo esté presionado y el puntero no esté sobre un control. `G` alterna `attract`.
- Los modos de barras, franja y apagado ignoran el mouse como hoy.

### 3.6 Repintado y rendimiento

- La simulación solo avanza mientras suena; en pausa se dibuja el último estado, sin repintado continuo (política vigente: continuo solo reproduciendo; latido de 1 s en reposo).
- Topes (confirmados con el benchmark, sin cambios): Partículas 1500, Constelación 600 (enlaces ≤ 2500), Anillo 360, Osciloscopio 400.
- Cada modo se verifica con `--bench`; si alguno rebasa el presupuesto, se baja su tope antes de cerrar la tarea (se anota en el ledger).

### 3.7 Benchmark de CPU y GPU

Tres controles, de lo determinista a lo medido:

1. **Peso del mesh (prueba unitaria, determinista):** cada modo, a su tope de partículas y con el peor caso de enlaces, genera como máximo `MAX_MESH_VERTICES = 60 000` vértices por frame (un solo `Mesh`, una sola draw call). Un test falla si algún modo lo excede. Es el control de "GPU" que corre en `cargo test`.
2. **`--bench <s>` ampliado:** además de `CPU % | fps | ms de CPU por frame`, imprime los vértices/frame del modo activo y la **utilización de GPU** muestreada durante la ventana de medición. Fuentes, en orden: `nvidia-smi` (`utilization.gpu`, un proceso hijo con `-l 1`), `gpu_busy_percent` de sysfs (AMD) y, si no hay ninguna, `GPU n/d` (nunca falla). El muestreo vive en `src/gpu_probe.rs`, con el parseo como funciones puras con pruebas. Mide la GPU **completa** (no solo este proceso), así que se compara contra la línea base del modo Barras en la misma sesión.
3. **`scripts/bench-visualizers.sh`:** corre `--bench` en cada modo (`--fullscreen --viz <modo> --play`), imprime una tabla y sale con error si algún modo rebasa `CPU_MAX` (por defecto 20 %, de un núcleo), `MS_MAX` (1.5 ms de CPU por frame; el CPU % depende de los fps de la pantalla y fluctúa entre mediciones, los ms por frame no) o supera `GPU_MAX` (por defecto 30 % de la GPU, solo si hay lectura). Los límites se pueden pasar por variables de entorno. Si el modo Barras ya rebasa un límite en esta máquina, el límite se sube y la decisión se anota en el ledger.

Si un modo rebasa, la primera palanca es bajar su tope de partículas; el valor final queda en constantes con nombre (`COUNT_*`) y en este documento.

---

## 4. Pruebas

**Bandas (`viz/bands.rs`):** silencio → ceros; solo graves → solo `bass`; solo agudos → solo `high`; lista vacía y listas muy cortas no hacen pánico; el suavizado sube rápido y baja despacio (ataque < caída); valores acotados a 0..1.

**Física (`viz/particles.rs`):**
- Free: nada sale de los límites tras muchos pasos (rebote); sin audio ni mouse, la velocidad decae por la fricción.
- Ring: sin audio y sin mouse, las partículas convergen a su destino (error < 1 px tras N pasos); con graves el radio crece; con `Pull` el resorte es más débil y al soltar vuelve a converger.
- Wave: converge al destino; la amplitud crece con medios y graves.
- Atracción acerca las partículas al cursor; repulsión las aleja.
- `dt` grande (hasta el tope) no explota ni genera `NaN`; `resize` conserva el número de partículas.
- Determinismo: misma semilla → mismas posiciones.

**Enlaces:** solo une pares dentro de `max_dist`; nunca devuelve más de `cap`; no duplica pares; con 0 o 1 partículas devuelve vacío.

**Ciclo de modos:** recorre los siete modos y vuelve al inicio, con las etiquetas esperadas.

**Mouse:** función pura que decide si el puntero está sobre un control (dentro/fuera de cada rectángulo).

**Manual (con capturas del propio programa y `--bench`):** los cuatro modos a 1050×750 y 1000×650, con y sin letras; reproducción real con el clic atrayendo partículas.

---

## 5. Criterios de aceptación

1. El ciclo muestra Barras, Anillo, Partículas, Constelación, Osciloscopio, Franja y Apagado; Resplandor y Radial ya no existen.
2. Los cuatro modos reaccionan al audio (graves, medios, agudos) con el acento de la portada.
3. El Anillo está centrado en la portada, también con las letras abiertas.
4. Clic mantenido atrae; `G` alterna atraer/repeler; el clic sobre un control no atrae.
5. Ningún modo rebasa el presupuesto de CPU vigente ni los límites de `scripts/bench-visualizers.sh` (CPU y GPU), el test de peso del mesh pasa y, en pausa, no hay repintado continuo.
6. `cargo test` en verde; sin advertencias.
7. README actualizado (modos y tecla `G`).
