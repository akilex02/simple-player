# Visualizadores de partículas — Plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cuatro modos de partículas reactivos al audio (Anillo, Partículas, Constelación, Osciloscopio) en la pantalla completa, con interacción de mouse, y un benchmark de CPU y GPU que los mantenga ligeros.

**Architecture:** Lógica pura sin egui (`viz/bands.rs` para graves/medios/agudos, `viz/particles.rs` para la física y los enlaces de la constelación), un dibujante que arma un solo `egui::Mesh` por frame (`ui/visualizers/particles.rs`) y la conexión en `FullscreenView`. El benchmark amplía `--bench` con GPU (`src/gpu_probe.rs`) y suma `scripts/bench-visualizers.sh`.

**Tech Stack:** Rust, egui/eframe 0.29 (ya en el proyecto); sin dependencias nuevas (`nvidia-smi`/sysfs se leen con `std`).

**Spec:** `docs/superpowers/specs/2026-10-03-visualizadores-de-particulas-design.md`

## Global Constraints

- Ciclo de modos: Barras → Anillo → Partículas → Constelación → Osciloscopio → Franja → Apagado → Barras. Etiquetas: `Barras`, `Anillo`, `Partículas`, `Constelación`, `Osciloscopio`, `Franja`, `Apagado`.
- Mouse: clic izquierdo mantenido atrae; **`G`** alterna atraer/repeler; Espacio no se toca (es reproducir/pausar).
- Topes iniciales: Partículas 1500, Constelación 600 (enlaces ≤ 2500), Anillo 360, Osciloscopio 400. `MAX_MESH_VERTICES = 60 000` por frame.
- Constantes de bandas: `BASS_SHARE = 0.12`, `MID_SHARE = 0.45`, `ATTACK_TAU = 0.04 s`, `RELEASE_TAU = 0.18 s`.
- Físicas: `dt` normalizado a pasos de 1/60 s con tope de 3; fricción 0.98; rebote 0.7; resorte 0.055 (0.005 con clic); trazo de Anillo/Osciloscopio 2.5 px.
- El color sale del acento dinámico de la portada (`theme::accent`); sin paletas neón.
- La simulación solo avanza mientras suena; en pausa se dibuja el último estado. Sin repintado continuo extra.
- Límites del benchmark: `CPU_MAX = 20` (% de un núcleo), `GPU_MAX = 30` (% de la GPU, solo si hay lectura).
- Texto de interfaz en español; sin dependencias nuevas; sin advertencias del compilador.
- Sin push ni merge: todo se commitea en la rama `nueva-version`.
- Commits terminan con `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`.
- Comandos de prueba (desde la raíz del repo): `cargo test <filtro>`; suite completa `cargo test`.

**Aclaraciones al spec (decididas al planificar):**
- Las bandas del spec van de 0 a 1, no 0–30 como en el original. Por eso las constantes de impulso (`DRIFT`, `SHAKE`, `JITTER`, `THERMAL`, `BASS_PUSH`) se expresan por unidad de banda y se afinan a ojo en la Tarea 8 con capturas.
- El spec §3.2 fijaba `g = 0.5`; a ese valor el clic casi no se nota (el original lo multiplicaba con las flechas). Se usa `PULL_G = 1.5` (Tarea 3 corrige el spec).
- §3.5 enumera zonas "sobre un control"; los widgets reales (botones, deslizadores, letras) ya capturan el clic por sí mismos. La función pura `over_control` solo cubre lo que se **pinta** sin widget (portada, información, zona de controles, encabezado y letras).
- Los módulos nuevos llevan `#![allow(dead_code)]` hasta la Tarea 7 (donde se conectan) para no emitir advertencias entre commits; la Tarea 7 lo quita.

## Review Focus

Entradas o condiciones que el spec implica pero que ninguna tarea cubriría sola; cada una tiene su prueba:

1. **Barras vacías o muy cortas** (sin música, una sola barra): bandas en cero, sin pánico. → Tarea 1.
2. **`dt` enorme, cero, negativo o NaN** (ventana suspendida, primer frame): sin NaN y con tope de 3 pasos. → Tarea 2.
3. **Cambio de tamaño con el campo vivo:** misma cantidad de partículas, todas dentro de los nuevos límites. → Tarea 2.
4. **Cursor exactamente sobre una partícula** (distancia 0) y **portada de radio 0:** sin división entre cero ni NaN. → Tarea 3.
5. **Peor caso de enlaces** (todas las partículas apiladas): el mesh no pasa de `MAX_MESH_VERTICES`. → Tarea 6.

---

### Task 1: Bandas (graves, medios y agudos)

**Files:**
- Create: `src/viz/bands.rs`
- Modify: `src/viz/mod.rs` (declarar `pub mod bands;`)
- Test: en `src/viz/bands.rs` (módulo `tests`)

**Interfaces:**
- Consumes: `crate::viz::engine::smooth(current, target, dt, attack_tau, release_tau) -> f32`.
- Produces: `Bands { bass, mid, high }` (`Copy`, `Default`, `PartialEq`, valores 0..1); `split(bars: &[f32]) -> Bands`; `BandAnalyzer::new()`; `BandAnalyzer::update(&mut self, bars: &[f32], dt: f32) -> Bands`.

- [ ] **Step 1: Write the failing tests**

Crear `src/viz/bands.rs` con solo el módulo de pruebas y las declaraciones mínimas para que compile el archivo vacío de implementación (el primer `cargo test` debe fallar por tipos inexistentes):

```rust
#![allow(dead_code)] // se quita al conectar el módulo (Tarea 7)

#[cfg(test)]
mod tests {
    use super::*;

    fn bars_with(n: usize, ones: std::ops::Range<usize>) -> Vec<f32> {
        (0..n).map(|i| if ones.contains(&i) { 1.0 } else { 0.0 }).collect()
    }

    #[test]
    fn sin_barras_todo_es_cero() {
        assert_eq!(split(&[]), Bands::default());
    }

    #[test]
    fn el_silencio_da_ceros() {
        assert_eq!(split(&vec![0.0; 100]), Bands::default());
    }

    #[test]
    fn solo_los_graves_llenan_solo_bass() {
        // 100 barras: graves = primeras 12.
        let b = split(&bars_with(100, 0..12));
        assert_eq!(b, Bands { bass: 1.0, mid: 0.0, high: 0.0 });
    }

    #[test]
    fn solo_los_agudos_llenan_solo_high() {
        // medios = barras 12..57; agudos = 57..100.
        let b = split(&bars_with(100, 57..100));
        assert_eq!(b, Bands { bass: 0.0, mid: 0.0, high: 1.0 });
    }

    #[test]
    fn una_sola_barra_no_hace_panico() {
        let b = split(&[1.0]);
        assert_eq!(b.bass, 1.0);
        assert_eq!(b.mid, 0.0);
        assert_eq!(b.high, 0.0);
    }

    #[test]
    fn los_valores_fuera_de_rango_se_acotan() {
        let b = split(&vec![5.0; 100]);
        assert_eq!(b, Bands { bass: 1.0, mid: 1.0, high: 1.0 });
    }

    #[test]
    fn el_ataque_es_mas_rapido_que_la_caida() {
        let loud = vec![1.0; 100];
        let quiet = vec![0.0; 100];
        let rise = BandAnalyzer::new().update(&loud, 0.016).bass;

        let mut analyzer = BandAnalyzer::new();
        for _ in 0..300 {
            analyzer.update(&loud, 0.016);
        }
        let fall = 1.0 - analyzer.update(&quiet, 0.016).bass;
        assert!(rise > fall, "ataque {rise} debe superar caída {fall}");
    }

    #[test]
    fn la_salida_siempre_queda_entre_cero_y_uno() {
        let mut analyzer = BandAnalyzer::new();
        for _ in 0..100 {
            let b = analyzer.update(&vec![9.0; 64], 0.016);
            assert!((0.0..=1.0).contains(&b.bass) && (0.0..=1.0).contains(&b.mid) && (0.0..=1.0).contains(&b.high));
        }
    }
}
```

Añadir en `src/viz/mod.rs`, junto a `pub mod engine;`: `pub mod bands;`.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test bands 2>&1 | tail -15`
Expected: error de compilación `cannot find function split` / `cannot find type Bands` (RED).

- [ ] **Step 3: Write minimal implementation**

Insertar **antes** de `#[cfg(test)]` en `src/viz/bands.rs`:

```rust
//! Graves, medios y agudos a partir de las barras del visualizador.
use super::engine::smooth;

const BASS_SHARE: f32 = 0.12;
const MID_SHARE: f32 = 0.45;
const ATTACK_TAU: f32 = 0.04;
const RELEASE_TAU: f32 = 0.18;

/// Energía por rango de frecuencia, 0..1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bands {
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
}

/// Medias crudas por rango (sin suavizar): primeras 12 % de barras = graves,
/// siguientes 45 % = medios y el resto = agudos.
pub fn split(bars: &[f32]) -> Bands {
    let n = bars.len();
    if n == 0 {
        return Bands::default();
    }
    let bass_end = ((n as f32 * BASS_SHARE).round() as usize).clamp(1, n);
    let mid_end = (bass_end + (n as f32 * MID_SHARE).round() as usize).clamp(bass_end, n);
    let mean = |s: &[f32]| {
        if s.is_empty() {
            0.0
        } else {
            s.iter().map(|v| v.clamp(0.0, 1.0)).sum::<f32>() / s.len() as f32
        }
    };
    Bands { bass: mean(&bars[..bass_end]), mid: mean(&bars[bass_end..mid_end]), high: mean(&bars[mid_end..]) }
}

/// Suaviza las bandas entre frames (ataque rápido, caída lenta) para que no parpadeen.
pub struct BandAnalyzer {
    current: Bands,
}

impl BandAnalyzer {
    pub fn new() -> Self {
        Self { current: Bands::default() }
    }

    pub fn update(&mut self, bars: &[f32], dt: f32) -> Bands {
        let target = split(bars);
        let step = |current: f32, target: f32| smooth(current, target, dt, ATTACK_TAU, RELEASE_TAU).clamp(0.0, 1.0);
        self.current = Bands {
            bass: step(self.current.bass, target.bass),
            mid: step(self.current.mid, target.mid),
            high: step(self.current.high, target.high),
        };
        self.current
    }
}
```

(El `#![allow(dead_code)]` y el `//!` van al principio del archivo; mover el `#![allow(dead_code)]` arriba del comentario `//!` si el compilador lo pide.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test bands 2>&1 | tail -15`
Expected: `8 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/viz/bands.rs src/viz/mod.rs
git commit -m "Bandas de graves, medios y agudos para los visualizadores

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Campo de partículas — núcleo y modo libre

**Files:**
- Create: `src/viz/particles.rs`
- Modify: `src/viz/mod.rs` (declarar `pub mod particles;`)
- Test: en `src/viz/particles.rs`

**Interfaces:**
- Consumes: `crate::viz::bands::Bands`.
- Produces (todos `pub` en `crate::viz::particles`):
  - `V2 { x, y }` (`Copy`, `Default`, `PartialEq`) con `V2::new`, `V2::dist(self, V2) -> f32`.
  - `Rect2 { min: V2, max: V2 }` con `Rect2::new(x0, y0, x1, y1)`, `width()`, `height()` (mínimo 1.0), `center() -> V2`.
  - `Anchor { center: V2, radius: f32 }`; `Pull { pos: V2, attract: bool }`.
  - `FieldKind { Free, Ring, Wave }` (`Copy`, `PartialEq`).
  - `Particle { pos, vel, home: V2, size, tint: f32 }`.
  - `ParticleField::new(kind, count, bounds, seed: u64)`, `resize(&mut self, bounds)`, `bounds(&self) -> Rect2`, `particles(&self) -> &[Particle]`, `step(&mut self, dt: f32, bands: Bands, anchor: Anchor, pull: Option<Pull>, time: f32)`.
  - `FRAME: f32 = 1.0 / 60.0`.

- [ ] **Step 1: Write the failing tests**

Crear `src/viz/particles.rs` con solo estas pruebas (más `#![allow(dead_code)]`), y declarar `pub mod particles;` en `src/viz/mod.rs`:

```rust
#![allow(dead_code)] // se quita al conectar el módulo (Tarea 7)

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> Rect2 {
        Rect2::new(0.0, 0.0, 1000.0, 600.0)
    }

    fn anchor() -> Anchor {
        Anchor { center: V2::new(500.0, 300.0), radius: 100.0 }
    }

    fn loud() -> Bands {
        Bands { bass: 1.0, mid: 1.0, high: 1.0 }
    }

    /// Avanza `steps` pasos de 1/60 s con tiempo fijo (los destinos no se mueven).
    fn run(field: &mut ParticleField, steps: usize, bands: Bands, pull: Option<Pull>) {
        for _ in 0..steps {
            field.step(FRAME, bands, anchor(), pull, 0.0);
        }
    }

    fn single(kind: FieldKind, pos: V2) -> ParticleField {
        let mut field = ParticleField::new(kind, 1, bounds(), 1);
        field.particles[0] = Particle { pos, vel: V2::default(), home: pos, size: 2.0, tint: 0.5 };
        field
    }

    fn inside(p: &Particle, b: Rect2) -> bool {
        p.pos.x >= b.min.x && p.pos.x <= b.max.x && p.pos.y >= b.min.y && p.pos.y <= b.max.y
    }

    #[test]
    fn crea_la_cantidad_pedida_dentro_de_los_limites() {
        let field = ParticleField::new(FieldKind::Free, 300, bounds(), 7);
        assert_eq!(field.particles().len(), 300);
        assert!(field.particles().iter().all(|p| inside(p, bounds())));
    }

    #[test]
    fn la_misma_semilla_da_las_mismas_posiciones() {
        let mut a = ParticleField::new(FieldKind::Free, 100, bounds(), 7);
        let mut b = ParticleField::new(FieldKind::Free, 100, bounds(), 7);
        run(&mut a, 100, loud(), None);
        run(&mut b, 100, loud(), None);
        let same = a.particles().iter().zip(b.particles()).all(|(p, q)| p.pos == q.pos);
        assert!(same);
    }

    #[test]
    fn una_semilla_distinta_da_otras_posiciones() {
        let a = ParticleField::new(FieldKind::Free, 100, bounds(), 7);
        let b = ParticleField::new(FieldKind::Free, 100, bounds(), 8);
        assert!(a.particles().iter().zip(b.particles()).any(|(p, q)| p.pos != q.pos));
    }

    #[test]
    fn sin_audio_ni_mouse_la_velocidad_decae_por_la_friccion() {
        let mut field = ParticleField::new(FieldKind::Free, 200, bounds(), 3);
        run(&mut field, 600, Bands::default(), None);
        let fastest = field.particles().iter().map(|p| p.vel.x.hypot(p.vel.y)).fold(0.0, f32::max);
        assert!(fastest < 0.05, "velocidad máxima {fastest}");
    }

    #[test]
    fn con_audio_fuerte_nada_sale_de_los_limites() {
        let mut field = ParticleField::new(FieldKind::Free, 200, bounds(), 3);
        run(&mut field, 600, loud(), None);
        assert!(field.particles().iter().all(|p| inside(p, bounds())));
    }

    #[test]
    fn en_silencio_una_particula_quieta_se_queda_quieta() {
        let start = V2::new(500.0, 300.0);
        let mut field = single(FieldKind::Free, start);
        run(&mut field, 100, Bands::default(), None);
        assert_eq!(field.particles()[0].pos, start);
    }

    #[test]
    fn con_graves_una_particula_quieta_deriva() {
        let start = V2::new(500.0, 300.0);
        let mut field = single(FieldKind::Free, start);
        run(&mut field, 50, Bands { bass: 1.0, mid: 0.0, high: 0.0 }, None);
        assert!(field.particles()[0].pos.dist(start) > 0.01);
    }

    #[test]
    fn un_dt_invalido_no_produce_nan_ni_mueve_nada() {
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut field = ParticleField::new(FieldKind::Free, 50, bounds(), 3);
            let before: Vec<V2> = field.particles().iter().map(|p| p.pos).collect();
            field.step(dt, loud(), anchor(), None, 0.0);
            let after: Vec<V2> = field.particles().iter().map(|p| p.pos).collect();
            assert_eq!(before, after, "dt = {dt}");
        }
    }

    #[test]
    fn un_dt_enorme_se_acota_a_tres_pasos() {
        let mut big = ParticleField::new(FieldKind::Free, 50, bounds(), 3);
        let mut three = ParticleField::new(FieldKind::Free, 50, bounds(), 3);
        big.step(10.0, Bands::default(), anchor(), None, 0.0);
        three.step(3.0 * FRAME, Bands::default(), anchor(), None, 0.0);
        let same = big.particles().iter().zip(three.particles()).all(|(p, q)| {
            p.pos.x.is_finite() && (p.pos.x - q.pos.x).abs() < 1e-3 && (p.pos.y - q.pos.y).abs() < 1e-3
        });
        assert!(same);
    }

    #[test]
    fn resize_conserva_la_cantidad_y_deja_todo_dentro() {
        let mut field = ParticleField::new(FieldKind::Free, 200, bounds(), 3);
        let smaller = Rect2::new(0.0, 0.0, 500.0, 300.0);
        field.resize(smaller);
        assert_eq!(field.particles().len(), 200);
        assert!(field.particles().iter().all(|p| inside(p, smaller)));
        assert_eq!(field.bounds(), smaller);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test viz::particles 2>&1 | tail -15`
Expected: error de compilación (`cannot find type ParticleField` …) (RED).

- [ ] **Step 3: Write minimal implementation**

Insertar **antes** de `#[cfg(test)]` (después del `#![allow(dead_code)]`):

```rust
//! Física de partículas de los visualizadores. Sin egui: se prueba sin ventana.
use super::bands::Bands;

/// Un paso de simulación equivale a un frame a 60 Hz.
pub const FRAME: f32 = 1.0 / 60.0;
const MAX_STEPS: f32 = 3.0;
const FRICTION: f32 = 0.98;
const BOUNCE_LOSS: f32 = 0.7;
/// Por debajo de esto una banda se considera silencio.
const BAND_FLOOR: f32 = 0.02;
// Impulsos por unidad de banda (0..1); se afinan a ojo (Tarea 8).
const DRIFT: f32 = 0.15;
const SHAKE: f32 = 1.5;
const JITTER: f32 = 0.6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

impl V2 {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dist(self, other: V2) -> f32 {
        (self.x - other.x).hypot(self.y - other.y)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect2 {
    pub min: V2,
    pub max: V2,
}

impl Rect2 {
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Self {
        Self { min: V2::new(x0, y0), max: V2::new(x1, y1) }
    }

    pub fn width(&self) -> f32 {
        (self.max.x - self.min.x).max(1.0)
    }

    pub fn height(&self) -> f32 {
        (self.max.y - self.min.y).max(1.0)
    }

    pub fn center(&self) -> V2 {
        V2::new((self.min.x + self.max.x) / 2.0, (self.min.y + self.max.y) / 2.0)
    }
}

/// Dónde está la portada: el anillo se centra en ella.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchor {
    pub center: V2,
    pub radius: f32,
}

/// Punto de gravedad del cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pull {
    pub pos: V2,
    pub attract: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldKind {
    /// Flotan libres y rebotan en los bordes.
    Free,
    /// Resorte hacia un aro alrededor de la portada.
    Ring,
    /// Resorte hacia una línea de osciloscopio.
    Wave,
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: V2,
    pub vel: V2,
    /// Destino del resorte (Anillo y Osciloscopio).
    pub home: V2,
    pub size: f32,
    /// 0..1, variación de brillo entre partículas.
    pub tint: f32,
}

/// Generador xorshift con semilla: las pruebas son deterministas.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// 0..1
    fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }

    fn signed(&mut self, amount: f32) -> f32 {
        self.range(-amount, amount)
    }
}

pub struct ParticleField {
    kind: FieldKind,
    particles: Vec<Particle>,
    bounds: Rect2,
    rng: Rng,
}

impl ParticleField {
    pub fn new(kind: FieldKind, count: usize, bounds: Rect2, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let particles = (0..count)
            .map(|_| {
                let pos = V2::new(rng.range(bounds.min.x, bounds.max.x), rng.range(bounds.min.y, bounds.max.y));
                Particle { pos, vel: V2::new(rng.signed(2.0), rng.signed(2.0)), home: pos, size: rng.range(1.5, 3.0), tint: rng.unit() }
            })
            .collect();
        Self { kind, particles, bounds, rng }
    }

    pub fn bounds(&self) -> Rect2 {
        self.bounds
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Reubica las partículas proporcionalmente al nuevo rectángulo, sin reiniciar la simulación.
    pub fn resize(&mut self, bounds: Rect2) {
        let old = self.bounds;
        for p in &mut self.particles {
            let fx = (p.pos.x - old.min.x) / old.width();
            let fy = (p.pos.y - old.min.y) / old.height();
            p.pos = V2::new(bounds.min.x + fx * bounds.width(), bounds.min.y + fy * bounds.height());
        }
        self.bounds = bounds;
    }

    /// Avanza la simulación `dt` segundos (acotado a tres pasos de 1/60 s).
    pub fn step(&mut self, dt: f32, bands: Bands, _anchor: Anchor, pull: Option<Pull>, _time: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let k = (dt / FRAME).min(MAX_STEPS);
        let damping = FRICTION.powf(k);
        let (bounds, kind) = (self.bounds, self.kind);
        for p in &mut self.particles {
            p.pos.x += p.vel.x * k;
            p.pos.y += p.vel.y * k;
            p.vel.x *= damping;
            p.vel.y *= damping;

            if kind == FieldKind::Free && pull.is_none() && bands.bass > BAND_FLOOR {
                let drift = bands.bass * DRIFT * k;
                p.vel.x += self.rng.signed(drift);
                p.vel.y += self.rng.signed(drift);
            }
            if bands.mid > BAND_FLOOR {
                let shake = bands.mid * SHAKE * k;
                p.pos.x += self.rng.signed(shake);
                p.pos.y += self.rng.signed(shake);
            }
            if bands.high > BAND_FLOOR {
                let jitter = bands.high * JITTER * k;
                p.vel.x += self.rng.signed(jitter);
                p.vel.y += self.rng.signed(jitter);
            }
            if kind == FieldKind::Free || pull.is_some() {
                bounce(p, bounds);
            }
        }
    }
}

fn bounce(p: &mut Particle, b: Rect2) {
    if p.pos.x < b.min.x {
        p.pos.x = b.min.x;
        p.vel.x = -p.vel.x * BOUNCE_LOSS;
    } else if p.pos.x > b.max.x {
        p.pos.x = b.max.x;
        p.vel.x = -p.vel.x * BOUNCE_LOSS;
    }
    if p.pos.y < b.min.y {
        p.pos.y = b.min.y;
        p.vel.y = -p.vel.y * BOUNCE_LOSS;
    } else if p.pos.y > b.max.y {
        p.pos.y = b.max.y;
        p.vel.y = -p.vel.y * BOUNCE_LOSS;
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test viz::particles 2>&1 | tail -15`
Expected: `10 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/viz/particles.rs src/viz/mod.rs
git commit -m "Campo de partículas: núcleo y modo libre

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Anillo, osciloscopio y atracción del cursor

**Files:**
- Modify: `src/viz/particles.rs`
- Modify: `docs/superpowers/specs/2026-10-03-visualizadores-de-particulas-design.md` (corregir `g`)
- Test: `src/viz/particles.rs` (módulo `tests`)

**Interfaces:**
- Consumes: todo lo de la Tarea 2.
- Produces: `spring_constant(has_pull: bool) -> f32` (privada, probada); comportamiento de `step` para `Ring`/`Wave` y para `Pull`. La firma pública no cambia (se renombran `_anchor`/`_time` a `anchor`/`time`).

- [ ] **Step 1: Write the failing tests**

Añadir dentro de `mod tests` de `src/viz/particles.rs`:

```rust
    fn mean_radius(field: &ParticleField) -> f32 {
        let c = anchor().center;
        field.particles().iter().map(|p| p.pos.dist(c)).sum::<f32>() / field.particles().len() as f32
    }

    fn max_deviation_from_center_y(field: &ParticleField) -> f32 {
        field.particles().iter().map(|p| (p.pos.y - 300.0).abs()).fold(0.0, f32::max)
    }

    #[test]
    fn atraer_acerca_la_particula_al_cursor() {
        let mut field = single(FieldKind::Free, V2::new(300.0, 300.0));
        let pull = Pull { pos: V2::new(500.0, 300.0), attract: true };
        run(&mut field, 120, Bands::default(), Some(pull));
        assert!(field.particles()[0].pos.x > 305.0, "x = {}", field.particles()[0].pos.x);
    }

    #[test]
    fn repeler_aleja_la_particula_del_cursor() {
        let mut field = single(FieldKind::Free, V2::new(300.0, 300.0));
        let pull = Pull { pos: V2::new(500.0, 300.0), attract: false };
        run(&mut field, 120, Bands::default(), Some(pull));
        assert!(field.particles()[0].pos.x < 295.0, "x = {}", field.particles()[0].pos.x);
    }

    #[test]
    fn el_cursor_justo_sobre_la_particula_no_produce_nan() {
        let pos = V2::new(500.0, 300.0);
        let mut field = single(FieldKind::Free, pos);
        run(&mut field, 10, loud(), Some(Pull { pos, attract: true }));
        let p = field.particles()[0];
        assert!(p.pos.x.is_finite() && p.pos.y.is_finite() && p.vel.x.is_finite() && p.vel.y.is_finite());
    }

    #[test]
    fn el_resorte_es_mas_debil_mientras_hay_clic() {
        assert!(spring_constant(true) < spring_constant(false));
    }

    #[test]
    fn el_anillo_sin_audio_converge_a_un_aro_alrededor_de_la_portada() {
        let mut field = ParticleField::new(FieldKind::Ring, 360, bounds(), 3);
        run(&mut field, 3000, Bands::default(), None);
        let expected = 100.0 * 1.15 + 12.0;
        for p in field.particles() {
            assert!(p.pos.dist(p.home) < 1.0, "lejos de su destino: {}", p.pos.dist(p.home));
            assert!((p.pos.dist(anchor().center) - expected).abs() < 1.5);
        }
    }

    #[test]
    fn el_anillo_crece_con_los_graves() {
        let mut quiet = ParticleField::new(FieldKind::Ring, 360, bounds(), 3);
        let mut bassy = ParticleField::new(FieldKind::Ring, 360, bounds(), 3);
        run(&mut quiet, 3000, Bands::default(), None);
        run(&mut bassy, 3000, Bands { bass: 1.0, mid: 0.0, high: 0.0 }, None);
        assert!(mean_radius(&bassy) > mean_radius(&quiet) + 20.0);
    }

    #[test]
    fn el_anillo_con_portada_de_radio_cero_no_produce_nan() {
        let mut field = ParticleField::new(FieldKind::Ring, 120, bounds(), 3);
        let flat = Anchor { center: V2::new(500.0, 300.0), radius: 0.0 };
        for _ in 0..200 {
            field.step(FRAME, loud(), flat, None, 0.0);
        }
        assert!(field.particles().iter().all(|p| p.pos.x.is_finite() && p.pos.y.is_finite()));
    }

    #[test]
    fn el_osciloscopio_sin_audio_converge_y_cubre_todo_el_ancho() {
        let mut field = ParticleField::new(FieldKind::Wave, 400, bounds(), 3);
        run(&mut field, 3000, Bands::default(), None);
        assert!(field.particles().iter().all(|p| p.pos.dist(p.home) < 1.0));
        let min_x = field.particles().iter().map(|p| p.pos.x).fold(f32::MAX, f32::min);
        let max_x = field.particles().iter().map(|p| p.pos.x).fold(f32::MIN, f32::max);
        assert!(min_x < 5.0 && max_x > 995.0, "x de {min_x} a {max_x}");
    }

    #[test]
    fn el_osciloscopio_crece_con_los_medios() {
        let mut quiet = ParticleField::new(FieldKind::Wave, 400, bounds(), 3);
        let mut mids = ParticleField::new(FieldKind::Wave, 400, bounds(), 3);
        run(&mut quiet, 3000, Bands::default(), None);
        run(&mut mids, 3000, Bands { bass: 0.0, mid: 1.0, high: 0.0 }, None);
        assert!(max_deviation_from_center_y(&mids) > max_deviation_from_center_y(&quiet) + 20.0);
    }

    #[test]
    fn al_soltar_el_clic_el_anillo_vuelve_a_su_destino() {
        let mut field = ParticleField::new(FieldKind::Ring, 360, bounds(), 3);
        run(&mut field, 300, Bands::default(), Some(Pull { pos: V2::new(50.0, 50.0), attract: true }));
        run(&mut field, 3000, Bands::default(), None);
        assert!(field.particles().iter().all(|p| p.pos.dist(p.home) < 1.5));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test viz::particles 2>&1 | tail -25`
Expected: error de compilación `cannot find function spring_constant` (RED). Tras añadir solo esa función vacía las demás fallarían por aserciones; el error de compilación ya es el RED aceptable.

- [ ] **Step 3: Write minimal implementation**

3a. Añadir constantes junto a las demás (arriba de `V2`):

```rust
const THERMAL: f32 = 0.04;
const BASS_PUSH: f32 = 0.2;
/// El spec decía 0.5 (valor del original con teclas de ajuste); a ese valor casi no se nota.
const PULL_G: f32 = 1.5;
const PULL_MIN_DIST: f32 = 25.0;
const SPRING_HELD: f32 = 0.005;
const SPRING_SNAP: f32 = 0.055;
```

3b. Reemplazar **por completo** `ParticleField::step` por esta versión (y añadir `update_homes`), todo dentro de `impl ParticleField`:

```rust
    /// Avanza la simulación `dt` segundos (acotado a tres pasos de 1/60 s).
    pub fn step(&mut self, dt: f32, bands: Bands, anchor: Anchor, pull: Option<Pull>, time: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let k = (dt / FRAME).min(MAX_STEPS);
        self.update_homes(bands, anchor, time);
        let spring = spring_constant(pull.is_some());
        let damping = FRICTION.powf(k);
        let (bounds, kind) = (self.bounds, self.kind);
        for p in &mut self.particles {
            p.pos.x += p.vel.x * k;
            p.pos.y += p.vel.y * k;
            p.vel.x *= damping;
            p.vel.y *= damping;

            match kind {
                FieldKind::Free => {
                    if pull.is_none() && bands.bass > BAND_FLOOR {
                        let drift = bands.bass * DRIFT * k;
                        p.vel.x += self.rng.signed(drift);
                        p.vel.y += self.rng.signed(drift);
                    }
                }
                FieldKind::Ring | FieldKind::Wave => {
                    p.vel.x += (p.home.x - p.pos.x) * spring * k;
                    p.vel.y += (p.home.y - p.pos.y) * spring * k;
                    if bands.bass > BAND_FLOOR {
                        let thermal = bands.bass * THERMAL * k;
                        p.vel.x += self.rng.signed(thermal);
                        p.vel.y += self.rng.signed(thermal);
                    }
                }
            }
            if bands.mid > BAND_FLOOR {
                let shake = bands.mid * SHAKE * k;
                p.pos.x += self.rng.signed(shake);
                p.pos.y += self.rng.signed(shake);
            }
            if bands.high > BAND_FLOOR {
                let jitter = bands.high * JITTER * k;
                p.vel.x += self.rng.signed(jitter);
                p.vel.y += self.rng.signed(jitter);
            }
            if let Some(pull) = pull {
                apply_pull(p, pull, bands.bass, k);
            }
            if kind == FieldKind::Free || pull.is_some() {
                bounce(p, bounds);
            }
        }
    }

    /// Recalcula los destinos del resorte (Anillo y Osciloscopio).
    fn update_homes(&mut self, bands: Bands, anchor: Anchor, time: f32) {
        use std::f32::consts::TAU;
        let n = self.particles.len();
        match self.kind {
            FieldKind::Free => {}
            FieldKind::Ring => {
                let base = anchor.radius * 1.15 + 12.0;
                for (i, p) in self.particles.iter_mut().enumerate() {
                    let angle = i as f32 / n as f32 * TAU;
                    // Onda viajera de medios y chispas de agudos solo cada 11.ª partícula.
                    let wave = (angle * 6.0 - time * 6.0).cos() * bands.mid * anchor.radius * 0.10;
                    let spark = if i % 11 == 0 { bands.high * anchor.radius * 0.30 } else { 0.0 };
                    let r = base + bands.bass * anchor.radius * 0.35 + wave + spark;
                    p.home = V2::new(anchor.center.x + angle.cos() * r, anchor.center.y + angle.sin() * r);
                }
            }
            FieldKind::Wave => {
                let (w, h) = (self.bounds.width(), self.bounds.height());
                let center_y = self.bounds.center().y;
                let amp = h * 0.04 + bands.mid * h * 0.10 + bands.bass * h * 0.07;
                for i in 0..n {
                    let ratio = i as f32 / (n.max(2) - 1) as f32;
                    let wave1 = (ratio * TAU * 3.0 + time * 5.0).sin();
                    let wave2 = (ratio * TAU * 10.0 - time * 10.0).cos() * 0.25;
                    // Jitter de agudos en cada 7.ª partícula.
                    let jitter = if i % 7 == 0 { self.rng.signed(1.0) * bands.high * h * 0.02 } else { 0.0 };
                    self.particles[i].home = V2::new(self.bounds.min.x + ratio * w, center_y + (wave1 + wave2) * amp + jitter);
                }
            }
        }
    }
```

3c. Añadir las funciones libres (junto a `bounce`):

```rust
/// Resorte hacia el destino: flojo mientras se mantiene el clic (se siente elástico) y firme al soltar.
fn spring_constant(has_pull: bool) -> f32 {
    if has_pull { SPRING_HELD } else { SPRING_SNAP }
}

fn apply_pull(p: &mut Particle, pull: Pull, bass: f32, k: f32) {
    let dx = pull.pos.x - p.pos.x;
    let dy = pull.pos.y - p.pos.y;
    let dist = dx.hypot(dy);
    if dist <= 2.0 {
        return;
    }
    let direction = if pull.attract { 1.0 } else { -1.0 };
    // Los graves empujan hacia afuera: las partículas "laten" alrededor del cursor.
    let bass_push = if bass > 0.15 { -bass * BASS_PUSH } else { 0.0 };
    let force = direction * PULL_G / dist.max(PULL_MIN_DIST) + bass_push;
    p.vel.x += dx / dist * force * k;
    p.vel.y += dy / dist * force * k;
}
```

3d. Corregir el spec: en `docs/superpowers/specs/2026-10-03-visualizadores-de-particulas-design.md`, en §3.2 «Atracción», cambiar `(g = 0.5)` por `(g = 1.5; el original usaba 0.5 pero con teclas de ajuste y a ese valor casi no se nota)`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test viz::particles 2>&1 | tail -25`
Expected: `20 passed` (10 de la Tarea 2 + 10 nuevas). Si `el_anillo_...converge` falla por tolerancia, aumentar los pasos de 3000 a 4000 antes de tocar las constantes.

- [ ] **Step 5: Commit**

```bash
git add src/viz/particles.rs docs/superpowers/specs/2026-10-03-visualizadores-de-particulas-design.md
git commit -m "Partículas: anillo, osciloscopio y atracción del cursor

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Enlaces de la constelación

**Files:**
- Modify: `src/viz/particles.rs`
- Test: `src/viz/particles.rs`

**Interfaces:**
- Consumes: `Particle`, `V2` (Tarea 2).
- Produces: `Link { a: u32, b: u32, alpha: f32 }` y `links(particles: &[Particle], max_dist: f32, cap: usize) -> Vec<Link>`; cada par aparece una sola vez con `a < b`; `alpha = (1 - d / max_dist) * 0.35`.

- [ ] **Step 1: Write the failing tests**

Añadir en `mod tests`:

```rust
    fn particle_at(x: f32, y: f32) -> Particle {
        Particle { pos: V2::new(x, y), vel: V2::default(), home: V2::default(), size: 2.0, tint: 0.5 }
    }

    #[test]
    fn une_solo_los_pares_dentro_de_la_distancia() {
        let ps = [particle_at(0.0, 0.0), particle_at(10.0, 0.0), particle_at(500.0, 500.0)];
        let found = links(&ps, 45.0, 100);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].a, found[0].b), (0, 1));
    }

    #[test]
    fn la_opacidad_baja_con_la_distancia() {
        let near = links(&[particle_at(0.0, 0.0), particle_at(5.0, 0.0)], 45.0, 10)[0].alpha;
        let far = links(&[particle_at(0.0, 0.0), particle_at(40.0, 0.0)], 45.0, 10)[0].alpha;
        assert!(near > far && far > 0.0 && near <= 0.35);
    }

    #[test]
    fn nunca_devuelve_mas_enlaces_que_el_tope() {
        let ps: Vec<Particle> = (0..100).map(|i| particle_at(i as f32 * 0.1, 0.0)).collect();
        assert_eq!(links(&ps, 45.0, 50).len(), 50);
    }

    #[test]
    fn con_cero_o_una_particula_no_hay_enlaces() {
        assert!(links(&[], 45.0, 10).is_empty());
        assert!(links(&[particle_at(1.0, 1.0)], 45.0, 10).is_empty());
        assert!(links(&[particle_at(0.0, 0.0), particle_at(1.0, 0.0)], 45.0, 0).is_empty());
    }

    #[test]
    fn la_rejilla_encuentra_los_mismos_pares_que_la_fuerza_bruta() {
        let field = ParticleField::new(FieldKind::Free, 300, Rect2::new(0.0, 0.0, 300.0, 200.0), 5);
        let ps = field.particles();
        let mut brute = 0;
        for i in 0..ps.len() {
            for j in (i + 1)..ps.len() {
                if ps[i].pos.dist(ps[j].pos) < 45.0 {
                    brute += 1;
                }
            }
        }
        let mut found = links(ps, 45.0, usize::MAX);
        assert_eq!(found.len(), brute);
        found.sort_by_key(|l| (l.a, l.b));
        found.dedup_by_key(|l| (l.a, l.b));
        assert_eq!(found.len(), brute, "hay pares repetidos");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test viz::particles 2>&1 | tail -15`
Expected: error de compilación `cannot find function links` (RED).

- [ ] **Step 3: Write minimal implementation**

Añadir en `src/viz/particles.rs` (antes de `#[cfg(test)]`; y `use std::collections::HashMap;` arriba):

```rust
const LINK_ALPHA: f32 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Link {
    pub a: u32,
    pub b: u32,
    pub alpha: f32,
}

/// Pares de partículas a menos de `max_dist`, con rejilla espacial (no es O(n²)) y a lo más `cap`.
pub fn links(particles: &[Particle], max_dist: f32, cap: usize) -> Vec<Link> {
    if particles.len() < 2 || max_dist <= 0.0 || cap == 0 {
        return Vec::new();
    }
    let cell_of = |pos: V2| ((pos.x / max_dist).floor() as i32, (pos.y / max_dist).floor() as i32);
    let mut grid: HashMap<(i32, i32), Vec<u32>> = HashMap::with_capacity(particles.len());
    for (i, p) in particles.iter().enumerate() {
        grid.entry(cell_of(p.pos)).or_default().push(i as u32);
    }
    let mut out = Vec::new();
    'all: for (i, p) in particles.iter().enumerate() {
        let (cx, cy) = cell_of(p.pos);
        for dx in -1..=1 {
            for dy in -1..=1 {
                let Some(bucket) = grid.get(&(cx + dx, cy + dy)) else { continue };
                for &j in bucket {
                    // Cada par se cuenta una vez: solo hacia índices mayores.
                    if (j as usize) <= i {
                        continue;
                    }
                    let d = p.pos.dist(particles[j as usize].pos);
                    if d < max_dist {
                        out.push(Link { a: i as u32, b: j, alpha: (1.0 - d / max_dist) * LINK_ALPHA });
                        if out.len() >= cap {
                            break 'all;
                        }
                    }
                }
            }
        }
    }
    out
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test viz::particles 2>&1 | tail -15`
Expected: `25 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/viz/particles.rs
git commit -m "Enlaces de la constelación con rejilla espacial

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Modos del visualizador y ciclo

**Files:**
- Modify: `src/ui/visualizers/mod.rs`
- Modify: `src/main.rs:125-132` (la lectura de `--viz`)
- Test: `src/ui/visualizers/mod.rs`

**Interfaces:**
- Consumes: `crate::viz::particles::FieldKind`.
- Produces: `VisualizerMode { Bars, Ring, Particles, Constellation, Wave, Strip, Off }` con `next()`, `label()`, `field_kind(self) -> Option<FieldKind>` y `from_name(&str) -> Option<VisualizerMode>` (sin distinguir mayúsculas ni acentos). `visualizers::draw(ui, rect, frame, mode, cover)` conserva su firma; **Ring sigue usando `radial::draw` y los demás modos nuevos no dibujan nada** hasta la Tarea 7.

- [ ] **Step 1: Write the failing tests**

Reemplazar el test del ciclo en `src/ui/visualizers/mod.rs` y añadir los nuevos:

```rust
    #[test]
    fn el_ciclo_recorre_los_siete_modos_y_vuelve_al_inicio() {
        let mut mode = VisualizerMode::Bars;
        let mut labels = Vec::new();
        for _ in 0..7 {
            mode = mode.next();
            labels.push(mode.label());
        }
        assert_eq!(labels, ["Anillo", "Partículas", "Constelación", "Osciloscopio", "Franja", "Apagado", "Barras"]);
    }

    #[test]
    fn el_nombre_se_reconoce_sin_mayusculas_ni_acentos() {
        assert_eq!(VisualizerMode::from_name("CONSTELACION"), Some(VisualizerMode::Constellation));
        assert_eq!(VisualizerMode::from_name("particulas"), Some(VisualizerMode::Particles));
        assert_eq!(VisualizerMode::from_name("Osciloscopio"), Some(VisualizerMode::Wave));
        assert_eq!(VisualizerMode::from_name("anillo"), Some(VisualizerMode::Ring));
        assert_eq!(VisualizerMode::from_name("radial"), None);
        assert_eq!(VisualizerMode::from_name(""), None);
    }

    #[test]
    fn solo_los_modos_de_particulas_tienen_campo() {
        use crate::viz::particles::FieldKind;
        assert_eq!(VisualizerMode::Ring.field_kind(), Some(FieldKind::Ring));
        assert_eq!(VisualizerMode::Particles.field_kind(), Some(FieldKind::Free));
        assert_eq!(VisualizerMode::Constellation.field_kind(), Some(FieldKind::Free));
        assert_eq!(VisualizerMode::Wave.field_kind(), Some(FieldKind::Wave));
        assert_eq!(VisualizerMode::Bars.field_kind(), None);
        assert_eq!(VisualizerMode::Strip.field_kind(), None);
        assert_eq!(VisualizerMode::Off.field_kind(), None);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test visualizers 2>&1 | tail -15`
Expected: error de compilación (`no variant named Particles` …) (RED).

- [ ] **Step 3: Write minimal implementation**

Reemplazar el enum y su `impl` en `src/ui/visualizers/mod.rs` (mantener `pub mod bars; pub mod radial;` por ahora) por:

```rust
use crate::viz::particles::FieldKind;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VisualizerMode {
    Bars,
    Ring,
    Particles,
    Constellation,
    Wave,
    /// Barras en una franja baja, sin tapar el contenido.
    Strip,
    Off,
}

impl VisualizerMode {
    pub fn next(self) -> Self {
        match self {
            VisualizerMode::Bars => VisualizerMode::Ring,
            VisualizerMode::Ring => VisualizerMode::Particles,
            VisualizerMode::Particles => VisualizerMode::Constellation,
            VisualizerMode::Constellation => VisualizerMode::Wave,
            VisualizerMode::Wave => VisualizerMode::Strip,
            VisualizerMode::Strip => VisualizerMode::Off,
            VisualizerMode::Off => VisualizerMode::Bars,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            VisualizerMode::Bars => "Barras",
            VisualizerMode::Ring => "Anillo",
            VisualizerMode::Particles => "Partículas",
            VisualizerMode::Constellation => "Constelación",
            VisualizerMode::Wave => "Osciloscopio",
            VisualizerMode::Strip => "Franja",
            VisualizerMode::Off => "Apagado",
        }
    }

    /// Tipo de campo de partículas que usa el modo, si usa uno.
    pub fn field_kind(self) -> Option<FieldKind> {
        match self {
            VisualizerMode::Ring => Some(FieldKind::Ring),
            VisualizerMode::Particles | VisualizerMode::Constellation => Some(FieldKind::Free),
            VisualizerMode::Wave => Some(FieldKind::Wave),
            _ => None,
        }
    }

    /// Para `--viz`: ignora mayúsculas y acentos.
    pub fn from_name(name: &str) -> Option<Self> {
        let wanted = fold(name);
        let mut mode = VisualizerMode::Bars;
        for _ in 0..7 {
            if fold(mode.label()) == wanted {
                return Some(mode);
            }
            mode = mode.next();
        }
        None
    }
}

fn fold(text: &str) -> String {
    text.to_lowercase().chars().map(|c| match c {
        'á' => 'a',
        'é' => 'e',
        'í' => 'i',
        'ó' => 'o',
        'ú' => 'u',
        other => other,
    }).collect()
}
```

Y actualizar `draw` para los modos nuevos:

```rust
pub fn draw(ui: &egui::Ui, rect: egui::Rect, frame: &VizFrame, mode: VisualizerMode, cover: egui::Rect) {
    let painter = ui.painter_at(rect);
    match mode {
        VisualizerMode::Bars | VisualizerMode::Strip => bars::draw(&painter, rect, frame),
        // El anillo usa el radial hasta que las partículas se conecten (Tarea 7).
        VisualizerMode::Ring => radial::draw(&painter, frame, cover),
        VisualizerMode::Particles | VisualizerMode::Constellation | VisualizerMode::Wave | VisualizerMode::Off => {}
    }
}
```

En `src/main.rs`, reemplazar el bloque de `--viz` (líneas ~125-132, el `while ... { ... }` y el `if` posterior) por:

```rust
        if let Some(mode) = ui::gallery::arg_value("--viz").and_then(|v| ui::visualizers::VisualizerMode::from_name(&v)) {
            fullscreen_view.mode = mode;
        }
```

(Leer primero el bloque exacto con `sed -n 120,135p src/main.rs` y reemplazarlo completo.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test 2>&1 | grep -E "^test result|error|warning" -A4`
Expected: suite completa en verde (≈ 245 pruebas), sin advertencias.

- [ ] **Step 5: Commit**

```bash
git add src/ui/visualizers/mod.rs src/main.rs
git commit -m "Modos Anillo, Partículas, Constelación y Osciloscopio en el ciclo

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Mesh de partículas y peso máximo

**Files:**
- Create: `src/ui/visualizers/particles.rs`
- Modify: `src/ui/visualizers/mod.rs` (declarar `pub mod particles;`)
- Test: `src/ui/visualizers/particles.rs`

**Interfaces:**
- Consumes: `ParticleField`, `Particle`, `V2`, `links`, `Link` (viz::particles); `VisualizerMode` (Tarea 5).
- Produces: constantes `COUNT_PARTICLES = 1500`, `COUNT_CONSTELLATION = 600`, `COUNT_RING = 360`, `COUNT_WAVE = 400`, `MAX_LINKS = 2500`, `MAX_MESH_VERTICES = 60_000`; `count_for(mode: VisualizerMode) -> usize` (0 para modos sin campo); `dots_mesh(field, accent) -> egui::Mesh`; `links_mesh(field, accent, max_dist) -> egui::Mesh`; `trace_mesh(field, closed: bool, accent) -> egui::Mesh`; `mesh_for(mode, field, accent, bass: f32) -> egui::Mesh`.

- [ ] **Step 1: Write the failing tests**

Crear `src/ui/visualizers/particles.rs` con `#![allow(dead_code)] // se quita al conectar (Tarea 7)` y estas pruebas; declarar `pub mod particles;` en `visualizers/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::viz::particles::{FieldKind, ParticleField, Rect2, V2};

    fn accent() -> egui::Color32 {
        egui::Color32::from_rgb(0xff, 0x9e, 0xbd)
    }

    fn field_at(kind: FieldKind, positions: &[V2]) -> ParticleField {
        let mut field = ParticleField::new(kind, positions.len(), Rect2::new(0.0, 0.0, 1000.0, 600.0), 1);
        field.set_positions_for_tests(positions);
        field
    }

    fn valid_indices(mesh: &egui::Mesh) -> bool {
        mesh.indices.iter().all(|&i| (i as usize) < mesh.vertices.len())
    }

    #[test]
    fn cada_punto_aporta_un_hexagono() {
        let field = field_at(FieldKind::Free, &[V2::new(10.0, 10.0), V2::new(50.0, 50.0)]);
        let mesh = dots_mesh(&field, accent());
        assert_eq!(mesh.vertices.len(), 12);
        assert_eq!(mesh.indices.len(), 24);
        assert!(valid_indices(&mesh));
    }

    #[test]
    fn un_enlace_cercano_aporta_un_quad_y_uno_lejano_nada() {
        let near = field_at(FieldKind::Free, &[V2::new(0.0, 0.0), V2::new(10.0, 0.0)]);
        let far = field_at(FieldKind::Free, &[V2::new(0.0, 0.0), V2::new(500.0, 0.0)]);
        assert_eq!(links_mesh(&near, accent(), 45.0).vertices.len(), 4);
        assert_eq!(links_mesh(&far, accent(), 45.0).vertices.len(), 0);
    }

    #[test]
    fn el_trazo_cerrado_une_todas_y_el_abierto_deja_un_hueco() {
        let ps: Vec<V2> = (0..5).map(|i| V2::new(i as f32 * 20.0, 10.0)).collect();
        let ring = field_at(FieldKind::Ring, &ps);
        assert_eq!(trace_mesh(&ring, true, accent()).vertices.len(), 5 * 4);
        assert_eq!(trace_mesh(&ring, false, accent()).vertices.len(), 4 * 4);
    }

    #[test]
    fn los_modos_sin_campo_no_tienen_particulas() {
        assert_eq!(count_for(VisualizerMode::Bars), 0);
        assert_eq!(count_for(VisualizerMode::Strip), 0);
        assert_eq!(count_for(VisualizerMode::Off), 0);
        assert_eq!(count_for(VisualizerMode::Particles), COUNT_PARTICLES);
        assert_eq!(count_for(VisualizerMode::Constellation), COUNT_CONSTELLATION);
    }

    #[test]
    fn ningun_modo_supera_el_tope_de_vertices_ni_en_el_peor_caso() {
        // Peor caso: todas las partículas apiladas en 100 × 100, así que se llega al tope de enlaces.
        for mode in [VisualizerMode::Ring, VisualizerMode::Particles, VisualizerMode::Constellation, VisualizerMode::Wave] {
            let kind = mode.field_kind().unwrap();
            let field = ParticleField::new(kind, count_for(mode), Rect2::new(0.0, 0.0, 100.0, 100.0), 1);
            let mesh = mesh_for(mode, &field, accent(), 1.0);
            assert!(valid_indices(&mesh), "{mode:?}: índices inválidos");
            assert!(
                mesh.vertices.len() <= MAX_MESH_VERTICES,
                "{mode:?}: {} vértices > {MAX_MESH_VERTICES}",
                mesh.vertices.len()
            );
        }
    }

    #[test]
    fn la_constelacion_no_pasa_del_tope_de_enlaces() {
        let field = ParticleField::new(FieldKind::Free, COUNT_CONSTELLATION, Rect2::new(0.0, 0.0, 100.0, 100.0), 1);
        let mesh = links_mesh(&field, accent(), 45.0 + 6.0);
        assert!(mesh.vertices.len() <= MAX_LINKS * 4);
    }
}
```

Para poder fijar posiciones en pruebas desde otro módulo, añadir en `src/viz/particles.rs` (dentro de `impl ParticleField`, **solo para pruebas**):

```rust
    #[cfg(test)]
    pub fn set_positions_for_tests(&mut self, positions: &[V2]) {
        self.particles.truncate(positions.len());
        for (p, pos) in self.particles.iter_mut().zip(positions) {
            p.pos = *pos;
            p.home = *pos;
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test visualizers::particles 2>&1 | tail -15`
Expected: error de compilación (`cannot find function dots_mesh` …) (RED).

- [ ] **Step 3: Write minimal implementation**

Insertar **antes** de `#[cfg(test)]` en `src/ui/visualizers/particles.rs`:

```rust
//! Dibujo de los modos de partículas: un solo `Mesh` por frame (una draw call).
use super::VisualizerMode;
use crate::viz::particles::{links, ParticleField, V2};
use eframe::egui;

pub const COUNT_PARTICLES: usize = 1500;
pub const COUNT_CONSTELLATION: usize = 600;
pub const COUNT_RING: usize = 360;
pub const COUNT_WAVE: usize = 400;
pub const MAX_LINKS: usize = 2500;
/// Tope de vértices por frame: un modo que lo rebase no pasa las pruebas.
pub const MAX_MESH_VERTICES: usize = 60_000;
const LINK_BASE_DIST: f32 = 45.0;
const LINK_BASS_DIST: f32 = 6.0;
const LINK_WIDTH: f32 = 1.0;
const TRACE_WIDTH: f32 = 2.5;
const DOT_ALPHA: u8 = 204;
const TRACE_ALPHA: u8 = 140;
/// Hexágono unitario: se ve redondo a tamaños de 1.5–3 px con la mitad de vértices que un círculo fino.
const HEX: [(f32, f32); 6] = [(1.0, 0.0), (0.5, 0.866), (-0.5, 0.866), (-1.0, 0.0), (-0.5, -0.866), (0.5, -0.866)];

pub fn count_for(mode: VisualizerMode) -> usize {
    match mode {
        VisualizerMode::Particles => COUNT_PARTICLES,
        VisualizerMode::Constellation => COUNT_CONSTELLATION,
        VisualizerMode::Ring => COUNT_RING,
        VisualizerMode::Wave => COUNT_WAVE,
        VisualizerMode::Bars | VisualizerMode::Strip | VisualizerMode::Off => 0,
    }
}

fn vertex(x: f32, y: f32, color: egui::Color32) -> egui::epaint::Vertex {
    egui::epaint::Vertex { pos: egui::pos2(x, y), uv: egui::epaint::WHITE_UV, color }
}

/// El acento, más claro según `tint` (hasta 35 % hacia blanco), con la opacidad dada.
fn tinted(accent: egui::Color32, tint: f32, alpha: u8) -> egui::Color32 {
    let mix = |c: u8| (c as f32 + (255.0 - c as f32) * tint.clamp(0.0, 1.0) * 0.35).round() as u8;
    egui::Color32::from_rgba_unmultiplied(mix(accent.r()), mix(accent.g()), mix(accent.b()), alpha)
}

/// Un cuadrilátero delgado entre `a` y `b`.
fn push_segment(mesh: &mut egui::Mesh, a: V2, b: V2, width: f32, color: egui::Color32) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = dx.hypot(dy);
    if len < 0.01 {
        return;
    }
    let (nx, ny) = (-dy / len * width / 2.0, dx / len * width / 2.0);
    let base = mesh.vertices.len() as u32;
    mesh.vertices.extend([
        vertex(a.x + nx, a.y + ny, color),
        vertex(b.x + nx, b.y + ny, color),
        vertex(b.x - nx, b.y - ny, color),
        vertex(a.x - nx, a.y - ny, color),
    ]);
    mesh.indices.extend([base, base + 1, base + 2, base, base + 2, base + 3]);
}

pub fn dots_mesh(field: &ParticleField, accent: egui::Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    mesh.reserve_vertices(field.particles().len() * 6);
    mesh.reserve_triangles(field.particles().len() * 4);
    for p in field.particles() {
        let color = tinted(accent, p.tint, DOT_ALPHA);
        let base = mesh.vertices.len() as u32;
        for (cx, cy) in HEX {
            mesh.vertices.push(vertex(p.pos.x + cx * p.size, p.pos.y + cy * p.size, color));
        }
        for i in 1..5u32 {
            mesh.indices.extend([base, base + i, base + i + 1]);
        }
    }
    mesh
}

pub fn links_mesh(field: &ParticleField, accent: egui::Color32, max_dist: f32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let ps = field.particles();
    for link in links(ps, max_dist, MAX_LINKS) {
        let alpha = (link.alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        let color = tinted(accent, 0.0, alpha);
        push_segment(&mut mesh, ps[link.a as usize].pos, ps[link.b as usize].pos, LINK_WIDTH, color);
    }
    mesh
}

/// Une cada partícula con la siguiente; `closed` cierra el aro con la última.
pub fn trace_mesh(field: &ParticleField, closed: bool, accent: egui::Color32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let ps = field.particles();
    let n = ps.len();
    if n < 2 {
        return mesh;
    }
    let segments = if closed { n } else { n - 1 };
    for i in 0..segments {
        let (a, b) = (&ps[i], &ps[(i + 1) % n]);
        push_segment(&mut mesh, a.pos, b.pos, TRACE_WIDTH, tinted(accent, a.tint, TRACE_ALPHA));
    }
    mesh
}

/// Todo lo que dibuja un modo de partículas, en un solo mesh.
pub fn mesh_for(mode: VisualizerMode, field: &ParticleField, accent: egui::Color32, bass: f32) -> egui::Mesh {
    let mut mesh = match mode {
        VisualizerMode::Constellation => links_mesh(field, accent, LINK_BASE_DIST + bass * LINK_BASS_DIST),
        VisualizerMode::Ring => trace_mesh(field, true, accent),
        VisualizerMode::Wave => trace_mesh(field, false, accent),
        _ => egui::Mesh::default(),
    };
    mesh.append(dots_mesh(field, accent));
    mesh
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test visualizers::particles 2>&1 | tail -15`
Expected: `6 passed`. Si el peor caso excede 60 000 vértices, bajar el tope del modo (`COUNT_*` o `MAX_LINKS`) y anotarlo en el mensaje del commit.

- [ ] **Step 5: Commit**

```bash
git add src/ui/visualizers/particles.rs src/ui/visualizers/mod.rs src/viz/particles.rs
git commit -m "Mesh único para los modos de partículas y prueba de peso máximo

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Conexión en la pantalla completa (campo, mouse y tecla G)

**Files:**
- Modify: `src/ui/fullscreen.rs`
- Modify: `src/ui/visualizers/mod.rs` (`draw` sin `cover`; quitar `radial`)
- Delete: `src/ui/visualizers/radial.rs`
- Modify: `src/viz/bands.rs`, `src/viz/particles.rs`, `src/ui/visualizers/particles.rs` (quitar `#![allow(dead_code)]`)
- Test: `src/ui/fullscreen.rs` (módulo `tests` nuevo, solo `over_control`)

**Interfaces:**
- Consumes: `BandAnalyzer`, `Bands`, `ParticleField`, `Anchor`, `Pull`, `Rect2`, `V2` (viz); `mesh_for`, `count_for` (ui::visualizers::particles); `VisualizerMode::field_kind`.
- Produces: `FullscreenView::mesh_vertices(&self) -> usize` (vértices del último mesh de partículas, para el benchmark de la Tarea 8); `over_control(pos: egui::Pos2, blockers: &[egui::Rect]) -> bool` (privada, probada); `visualizers::draw(ui, rect, frame, mode)` (ya sin `cover`; solo Barras/Franja).

- [ ] **Step 1: Write the failing test**

Añadir al final de `src/ui/fullscreen.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_puntero_sobre_una_zona_bloqueada_no_atrae() {
        let cover = egui::Rect::from_min_size(egui::pos2(100.0, 100.0), egui::vec2(200.0, 200.0));
        let controls = egui::Rect::from_min_size(egui::pos2(0.0, 600.0), egui::vec2(800.0, 100.0));
        let blockers = [cover, controls];
        assert!(over_control(egui::pos2(150.0, 150.0), &blockers));
        assert!(over_control(egui::pos2(400.0, 650.0), &blockers));
        assert!(!over_control(egui::pos2(500.0, 300.0), &blockers));
        assert!(!over_control(egui::pos2(500.0, 300.0), &[]));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test fullscreen 2>&1 | tail -10`
Expected: error de compilación `cannot find function over_control` (RED).

- [ ] **Step 3: Write the implementation**

3a. En `src/ui/fullscreen.rs`, imports nuevos:

```rust
use super::visualizers::particles as particle_draw;
use crate::viz::bands::{BandAnalyzer, Bands};
use crate::viz::particles::{Anchor, ParticleField, Pull, Rect2, V2};
```

3b. Reemplazar `FullscreenView` y su `Default`:

```rust
pub struct FullscreenView {
    pub mode: VisualizerMode,
    lyrics: LyricsView,
    bands: BandAnalyzer,
    bands_now: Bands,
    /// Campo de partículas del modo actual (se recrea al cambiar de modo).
    field: Option<(VisualizerMode, ParticleField)>,
    /// `true` atrae al cursor; `G` alterna con repeler.
    attract: bool,
    mesh_vertices: usize,
}

impl Default for FullscreenView {
    fn default() -> Self {
        Self {
            mode: VisualizerMode::Bars,
            lyrics: LyricsView::default(),
            bands: BandAnalyzer::new(),
            bands_now: Bands::default(),
            field: None,
            attract: true,
            mesh_vertices: 0,
        }
    }
}

/// Semilla fija: la disposición inicial es la misma en cada arranque.
const FIELD_SEED: u64 = 0x5EED_F1E1D;

/// ¿El puntero está sobre algo con lo que se interactúa o que no debe "atraer"?
fn over_control(pos: egui::Pos2, blockers: &[egui::Rect]) -> bool {
    blockers.iter().any(|r| r.contains(pos))
}
```

3c. En `show`, reemplazar el cuerpo del closure del `Area` (desde `ui.allocate_exact_size(...)` hasta `self.content(...)`), manteniendo `set_opacity`/`set_clip_rect`:

```rust
                // Reclama toda la ventana para que nada de abajo reciba el mouse.
                let (_, background) = ui.allocate_exact_size(rect.size(), egui::Sense::click_and_drag());

                let painter = ui.painter().clone();
                painter.rect_filled(rect, 0.0, theme::BG_BASE);
                backdrop.paint(textures, &painter, rect, ctx.input(|i| i.time) as f32);
                painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(70));

                let geo = self.geometry(ctx, rect, state.show_lyrics);
                let pull = self.pull_from_input(ctx, &background, &geo, state.show_lyrics);
                self.update_field(ctx, rect, &geo, viz, state.is_playing, pull);
                self.paint_visualizer(ui, rect, viz, anim, pull);
                self.content(ui, rect, state, textures, anim);
```

(Las tres primeras líneas de pintura — `painter`, `rect_filled`, `backdrop.paint`, `rect_filled` — ya existen; solo se sustituyen las dos últimas llamadas y se añaden las nuevas. Verificar con `sed -n 60,85p src/ui/fullscreen.rs`.)

3d. Sustituir `paint_visualizer` y añadir los métodos nuevos (dentro de `impl FullscreenView`):

```rust
    pub fn mesh_vertices(&self) -> usize {
        self.mesh_vertices
    }

    /// Clic izquierdo mantenido sobre el fondo; no cuenta sobre widgets (ya capturan el clic)
    /// ni sobre portada, información, controles, encabezado o letras.
    fn pull_from_input(&self, ctx: &egui::Context, background: &egui::Response, geo: &Geometry, show_lyrics: bool) -> Option<Pull> {
        self.mode.field_kind()?;
        let (down, pos) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.hover_pos()));
        let pos = pos?;
        if !down || !background.is_pointer_button_down_on() {
            return None;
        }
        let info = egui::Rect::from_min_max(
            egui::pos2(geo.left.left(), geo.cover_rect.bottom() + 24.0),
            egui::pos2(geo.left.right(), geo.cover_rect.bottom() + 120.0),
        );
        let lyrics = egui::Rect::from_min_max(egui::pos2(geo.left.right() + 56.0, geo.body.top()), geo.body.max);
        let mut blockers = vec![geo.cover_rect, info, geo.controls, geo.top];
        if show_lyrics || geo.lyrics_anim > 0.01 {
            blockers.push(lyrics);
        }
        if over_control(pos, &blockers) {
            return None;
        }
        Some(Pull { pos: V2::new(pos.x, pos.y), attract: self.attract })
    }

    /// Bandas, creación/redimensión del campo y un paso de simulación (solo si suena).
    fn update_field(&mut self, ctx: &egui::Context, rect: egui::Rect, geo: &Geometry, viz: &VizFrame, playing: bool, pull: Option<Pull>) {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        self.bands_now = self.bands.update(&viz.bars, dt);
        if ctx.input(|i| i.key_pressed(egui::Key::G)) {
            self.attract = !self.attract;
        }
        let Some(kind) = self.mode.field_kind() else {
            self.field = None;
            self.mesh_vertices = 0;
            return;
        };
        let bounds = Rect2::new(rect.left(), rect.top(), rect.right(), rect.bottom());
        match &mut self.field {
            Some((mode, field)) if *mode == self.mode => {
                if field.bounds() != bounds {
                    field.resize(bounds);
                }
            }
            _ => {
                self.field = Some((self.mode, ParticleField::new(kind, particle_draw::count_for(self.mode), bounds, FIELD_SEED)));
            }
        }
        if playing {
            let anchor = Anchor {
                center: V2::new(geo.cover_rect.center().x, geo.cover_rect.center().y),
                radius: geo.cover_rect.width() * 0.5,
            };
            let time = ctx.input(|i| i.time) as f32;
            if let Some((_, field)) = &mut self.field {
                field.step(dt, self.bands_now, anchor, pull, time);
            }
        }
    }

    fn paint_visualizer(&mut self, ui: &mut egui::Ui, rect: egui::Rect, viz: &VizFrame, anim: f32, pull: Option<Pull>) {
        let (area, opacity) = match self.mode {
            VisualizerMode::Off => return,
            VisualizerMode::Strip => {
                let bottom = rect.bottom() - BOTTOM_MARGIN - CONTROLS_H - 6.0;
                let band = egui::Rect::from_min_max(
                    egui::pos2(rect.left() + 56.0, bottom - STRIP_H),
                    egui::pos2(rect.right() - 56.0, bottom),
                );
                (band, 0.75)
            }
            _ => (rect, 0.4),
        };
        let mut layer = ui.new_child(egui::UiBuilder::new().max_rect(area));
        layer.set_opacity(anim * opacity);
        let Some((_, field)) = self.field.as_ref().filter(|_| self.mode.field_kind().is_some()) else {
            visualizers::draw(&layer, area, viz, self.mode);
            return;
        };
        let accent = theme::accent(ui.ctx());
        let mesh = particle_draw::mesh_for(self.mode, field, accent, self.bands_now.bass);
        self.mesh_vertices = mesh.vertices.len();
        let painter = layer.painter_at(area);
        painter.add(egui::Shape::mesh(mesh));
        if let Some(pull) = pull {
            // Indicador del cursor: verde atrae, rojo repele; late con los graves.
            let color = if pull.attract { egui::Color32::from_rgb(0, 255, 128) } else { egui::Color32::from_rgb(255, 40, 40) };
            let radius = 36.0 + self.bands_now.bass * 30.0;
            painter.circle_filled(egui::pos2(pull.pos.x, pull.pos.y), radius, color.gamma_multiply(0.18));
            painter.circle_filled(egui::pos2(pull.pos.x, pull.pos.y), 3.0, color);
        }
    }
```

3e. En `content`, la ayuda del chip: cambiar `.on_hover_text("Cambiar visualizador")` por:

```rust
            let hint = if self.mode.field_kind().is_some() { "Cambiar visualizador · clic: atraer · G: atraer/repeler" } else { "Cambiar visualizador" };
            if chip(ui, self.mode.label(), self.mode != VisualizerMode::Off).on_hover_text(hint).clicked() {
```

(Mantener el cuerpo `{ self.mode = self.mode.next(); }` tal cual.)

3f. En `src/ui/visualizers/mod.rs`: quitar `pub mod radial;`, simplificar `draw` a:

```rust
/// Barras y franja; los modos de partículas se dibujan con `particles::mesh_for`.
pub fn draw(ui: &egui::Ui, rect: egui::Rect, frame: &VizFrame, mode: VisualizerMode) {
    let painter = ui.painter_at(rect);
    if matches!(mode, VisualizerMode::Bars | VisualizerMode::Strip) {
        bars::draw(&painter, rect, frame);
    }
}
```

Borrar `src/ui/visualizers/radial.rs` (`git rm`). Quitar la primera línea `#![allow(dead_code)] ...` de `src/viz/bands.rs`, `src/viz/particles.rs` y `src/ui/visualizers/particles.rs`.

3g. `FullscreenView` usa `Bands` en el campo `bands_now`; si el compilador avisa de import sin uso (`Bands`), conservarlo solo si se usa. Si aparece alguna advertencia de código sin uso (p. ej. `Pull`/`Anchor` campos), resolverla quitando lo sobrante, no con `allow`.

- [ ] **Step 4: Run tests and build**

Run: `cargo test 2>&1 | grep -E "^test result|^(error|warning)" -A6; cargo build --release 2>&1 | grep -E "^(error|warning)" -A6`
Expected: suite en verde (≈ 252 pruebas), sin advertencias, build release correcto.

- [ ] **Step 5: Verify visually and commit**

```bash
S=/tmp/claude-1000/-home-budja8-Documents-Proyectos-simple-player-master/5badf05a-e152-478c-aeb0-3922f4d01526/scratchpad
for m in anillo particulas constelacion osciloscopio; do
  timeout 40 ./target/release/simple-player --allow-multiple --no-hotkeys --fullscreen --viz $m --play --time 60 --window-size 1050x750 --shot $S/viz_$m.png >/dev/null 2>&1
done
```

Leer cada `viz_*.png` con la herramienta Read y comprobar: el Anillo rodea la portada; Partículas se ven repartidas; Constelación muestra líneas finas entre cercanas; el Osciloscopio forma una línea ondulada de lado a lado; nada tapa los controles. Repetir el Anillo con `--lyrics` (el aro debe seguir a la portada). Anotar los ajustes de constantes que hagan falta (se hacen en la Tarea 8 junto con el benchmark).

```bash
git add -A
git commit -m "Conecta los modos de partículas en la pantalla completa (mouse y tecla G)

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Benchmark de CPU y GPU

**Files:**
- Create: `src/gpu_probe.rs`
- Create: `scripts/bench-visualizers.sh`
- Modify: `src/main.rs` (declarar `mod gpu_probe;`; ampliar `Bench`)
- Test: `src/gpu_probe.rs`

**Interfaces:**
- Consumes: `FullscreenView::mesh_vertices()` (Tarea 7).
- Produces:
  - `gpu_probe::parse_nvidia_line(&str) -> Option<f32>`, `parse_sysfs_busy(&str) -> Option<f32>`, `mean(&[f32]) -> Option<f32>` (puras, probadas).
  - `GpuProbe::start() -> GpuProbe`, `GpuProbe::begin_window(&self)`, `GpuProbe::report(&self) -> String` (`"GPU 12 % (nvidia-smi, 9 muestras)"` o `"GPU n/d"`).
  - La línea de `--bench` pasa a: `[bench] CPU 8.3 % | 60 fps | 1.38 ms de CPU por frame | GPU 12 % (nvidia-smi, 9 muestras) | 5400 vértices/frame`.
  - `scripts/bench-visualizers.sh` (variables `SECS`, `CPU_MAX`, `GPU_MAX`, `BIN`).

- [ ] **Step 1: Write the failing tests**

Crear `src/gpu_probe.rs` con (más `mod gpu_probe;` en `src/main.rs`, junto a los demás `mod`):

```rust
#![allow(dead_code)] // se quita al conectar el sondeo (más abajo en esta misma tarea)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lee_la_linea_de_nvidia_smi() {
        assert_eq!(parse_nvidia_line("37"), Some(37.0));
        assert_eq!(parse_nvidia_line(" 37 \n"), Some(37.0));
        assert_eq!(parse_nvidia_line("37 %"), Some(37.0));
        assert_eq!(parse_nvidia_line("[N/A]"), None);
        assert_eq!(parse_nvidia_line(""), None);
    }

    #[test]
    fn lee_el_valor_de_sysfs() {
        assert_eq!(parse_sysfs_busy("42\n"), Some(42.0));
        assert_eq!(parse_sysfs_busy("abc"), None);
    }

    #[test]
    fn rechaza_porcentajes_imposibles() {
        assert_eq!(parse_nvidia_line("250"), None);
        assert_eq!(parse_sysfs_busy("-3"), None);
    }

    #[test]
    fn la_media_de_nada_es_nada() {
        assert_eq!(mean(&[]), None);
        assert_eq!(mean(&[10.0, 20.0, 30.0]), Some(20.0));
    }

    #[test]
    fn sin_muestras_el_reporte_dice_no_disponible() {
        let probe = GpuProbe::disabled();
        assert_eq!(probe.report(), "GPU n/d");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test gpu_probe 2>&1 | tail -15`
Expected: error de compilación (`cannot find function parse_nvidia_line` …) (RED).

- [ ] **Step 3: Write minimal implementation**

Insertar antes de `#[cfg(test)]` en `src/gpu_probe.rs`:

```rust
//! Utilización de la GPU para `--bench` (solo desarrollo). Sin dependencias: lee
//! `nvidia-smi` (NVIDIA) o `gpu_busy_percent` de sysfs (AMD) y, si no hay ninguno, informa `n/d`.
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Un porcentaje válido (0..=100) a partir de texto como `37`, ` 37 ` o `37 %`.
fn parse_percent(text: &str) -> Option<f32> {
    let value: f32 = text.trim().trim_end_matches('%').trim().parse().ok()?;
    (0.0..=100.0).contains(&value).then_some(value)
}

pub fn parse_nvidia_line(line: &str) -> Option<f32> {
    parse_percent(line)
}

pub fn parse_sysfs_busy(text: &str) -> Option<f32> {
    parse_percent(text)
}

pub fn mean(samples: &[f32]) -> Option<f32> {
    (!samples.is_empty()).then(|| samples.iter().sum::<f32>() / samples.len() as f32)
}

/// Archivo `gpu_busy_percent` de la primera GPU AMD que lo expone.
fn find_sysfs_busy_file() -> Option<PathBuf> {
    std::fs::read_dir("/sys/class/drm").ok()?.flatten().map(|e| e.path().join("device/gpu_busy_percent")).find(|p| p.exists())
}

pub struct GpuProbe {
    samples: Arc<Mutex<Vec<f32>>>,
    source: &'static str,
    child: Option<Child>,
}

impl GpuProbe {
    /// Sin fuente: el reporte siempre es `GPU n/d`.
    pub fn disabled() -> Self {
        Self { samples: Arc::new(Mutex::new(Vec::new())), source: "", child: None }
    }

    /// Intenta `nvidia-smi` (una muestra por segundo) y luego sysfs.
    pub fn start() -> Self {
        let samples = Arc::new(Mutex::new(Vec::new()));
        let spawned = Command::new("nvidia-smi")
            .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits", "-l", "1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();
        if let Ok(mut child) = spawned {
            if let Some(stdout) = child.stdout.take() {
                let sink = Arc::clone(&samples);
                std::thread::spawn(move || {
                    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                        if let Some(value) = parse_nvidia_line(&line) {
                            sink.lock().unwrap().push(value);
                        }
                    }
                });
                return Self { samples, source: "nvidia-smi", child: Some(child) };
            }
        }
        if let Some(path) = find_sysfs_busy_file() {
            let sink = Arc::clone(&samples);
            std::thread::spawn(move || loop {
                if let Some(value) = std::fs::read_to_string(&path).ok().and_then(|t| parse_sysfs_busy(&t)) {
                    sink.lock().unwrap().push(value);
                }
                std::thread::sleep(Duration::from_secs(1));
            });
            return Self { samples, source: "sysfs", child: None };
        }
        Self::disabled()
    }

    /// Descarta lo muestreado hasta ahora (el calentamiento no cuenta).
    pub fn begin_window(&self) {
        self.samples.lock().unwrap().clear();
    }

    pub fn report(&self) -> String {
        let samples = self.samples.lock().unwrap();
        match mean(&samples) {
            Some(avg) => format!("GPU {avg:.0} % ({}, {} muestras)", self.source, samples.len()),
            None => "GPU n/d".to_string(),
        }
    }
}

impl Drop for GpuProbe {
    fn drop(&mut self) {
        if let Some(child) = &mut self.child {
            let _ = child.kill();
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test gpu_probe 2>&1 | tail -15`
Expected: `5 passed`.

- [ ] **Step 5: Conectar el sondeo a `--bench` y commit**

En `src/main.rs`, ampliar `Bench` (struct y métodos) y su llamada:

```rust
struct Bench {
    secs: f64,
    started: Instant,
    from: Option<u64>,
    frames: u64,
    frames_from: u64,
    gpu: gpu_probe::GpuProbe,
}
```

- `Bench::new`: añadir `gpu: gpu_probe::GpuProbe::start()`.
- En `poll`, al fijar `self.from = Some(Self::cpu_ticks());` añadir `self.gpu.begin_window();`.
- Cambiar la firma a `fn poll(&mut self, mesh_vertices: usize) -> Option<String>` y el `format!` final por:

```rust
                Some(format!(
                    "[bench] CPU {:.1} % | {:.0} fps | {:.2} ms de CPU por frame | {} | {} vértices/frame",
                    used / self.secs * 100.0,
                    frames / self.secs,
                    used / frames * 1000.0,
                    self.gpu.report(),
                    mesh_vertices
                ))
```

- En `handle_dev_bench`: `if let Some(report) = bench.poll(self.fullscreen.mesh_vertices()) {`.
- Quitar el `#![allow(dead_code)]` de `src/gpu_probe.rs`.

Crear `scripts/bench-visualizers.sh` (y `chmod +x`):

```bash
#!/usr/bin/env bash
# Mide CPU y GPU de cada visualizador de la pantalla completa y falla si alguno rebasa los límites.
# Variables: SECS (ventana de medición, 10), CPU_MAX (% de un núcleo, 20), GPU_MAX (% de la GPU, 30), BIN.
set -euo pipefail
cd "$(dirname "$0")/.."

SECS="${SECS:-10}"
CPU_MAX="${CPU_MAX:-20}"
GPU_MAX="${GPU_MAX:-30}"
BIN="${BIN:-target/release/simple-player}"
MODES=("Barras" "Anillo" "Partículas" "Constelación" "Osciloscopio" "Franja")

[ -x "$BIN" ] || cargo build --release

over() { awk -v v="$1" -v max="$2" 'BEGIN { exit !(v > max) }'; }

fail=0
printf '%-14s %8s %8s %10s\n' "Modo" "CPU %" "GPU %" "Vértices"
for mode in "${MODES[@]}"; do
  line=$("$BIN" --allow-multiple --no-hotkeys --fullscreen --viz "$mode" --play --bench "$SECS" 2>/dev/null | grep '^\[bench\]' || true)
  cpu=$(grep -oP 'CPU \K[0-9.]+' <<<"$line" || echo "n/d")
  gpu=$(grep -oP 'GPU \K[0-9.]+' <<<"$line" || echo "n/d")
  verts=$(grep -oP '\| \K[0-9]+(?= vértices)' <<<"$line" || echo "n/d")
  printf '%-14s %8s %8s %10s\n' "$mode" "$cpu" "$gpu" "$verts"
  if [[ "$cpu" != "n/d" ]] && over "$cpu" "$CPU_MAX"; then echo "  ✗ CPU $cpu % supera $CPU_MAX %"; fail=1; fi
  if [[ "$gpu" != "n/d" ]] && over "$gpu" "$GPU_MAX"; then echo "  ✗ GPU $gpu % supera $GPU_MAX %"; fail=1; fi
done
exit "$fail"
```

Verificar y medir:

```bash
cargo test 2>&1 | grep -E "^test result|^(error|warning)" -A4
cargo build --release 2>&1 | grep -E "^(error|warning)" -A4
SECS=8 scripts/bench-visualizers.sh
```

Expected: la tabla muestra una fila por modo con CPU, GPU (o `n/d`) y vértices; `Barras` y `Franja` dan `0` vértices de partículas. Reglas de decisión, en este orden, y cada una se anota en el mensaje del commit:
1. Si **Barras** ya rebasa un límite en esta máquina (la GPU se mide completa, incluye el escritorio), subir ese límite por encima de Barras + 8 puntos y dejar el nuevo valor por defecto en el script.
2. Si un modo de partículas rebasa `CPU_MAX` o `GPU_MAX` con los límites ya fijados, bajar su `COUNT_*` (y `MAX_LINKS` si es Constelación) en `src/ui/visualizers/particles.rs` y repetir la medición hasta que pase.
3. En esta misma pasada, con las capturas de la Tarea 7 a la vista, afinar a ojo las constantes de impulso (`DRIFT`, `SHAKE`, `JITTER`, `THERMAL`, `BASS_PUSH`, amplitudes de Anillo/Osciloscopio) solo si algún modo se ve exagerado o apagado; las pruebas siguen mandando (si una prueba falla por un valor, se corrige el valor, no la prueba, salvo que el ajuste cambie el diseño).

```bash
git add -A
git commit -m "Benchmark de CPU y GPU para los visualizadores

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```

---

### Task 9: Documentación y verificación final

**Files:**
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-10-03-visualizadores-de-particulas-design.md` (estado y valores finales)

**Interfaces:**
- Consumes: constantes finales de la Tarea 8 (`COUNT_*`, límites del script).
- Produces: documentación al día; criterios de aceptación del spec verificados.

- [ ] **Step 1: Actualizar el README**

En `README.md`:
- Característica del visualizador: reemplazar la línea por `- **Visualizadores:** barras, anillo alrededor de la portada, partículas, constelación, osciloscopio o franja inferior, alineados con el reloj de reproducción; con un clic mantenido las partículas siguen al cursor.`
- Tabla de atajos: añadir la fila `| G | Atraer / repeler partículas con el clic (pantalla completa) |`.
- Sección «Opciones útiles»: añadir `| --viz barras\|anillo\|particulas\|constelacion\|osciloscopio\|franja\|apagado | Abre la pantalla completa con ese visualizador |` si falta, y debajo un párrafo: `Para medir CPU y GPU de cada visualizador: scripts/bench-visualizers.sh (variables SECS, CPU_MAX, GPU_MAX; la GPU se lee de nvidia-smi o sysfs y, si no hay lectura, se omite).`

- [ ] **Step 2: Actualizar el spec**

Cambiar `> **Estado:**` a `implementado en nueva-version; falta la aceptación manual del usuario (criterios 3 y 4 con la app normal).` y dejar en §3.6 los topes finales y en §3.7 los límites finales si cambiaron en la Tarea 8.

- [ ] **Step 3: Verificación final**

```bash
cargo test 2>&1 | grep -E "^test result|^(error|warning)" -A4
cargo build --release 2>&1 | grep -E "^(error|warning)" -A4
SECS=10 scripts/bench-visualizers.sh; echo "exit=$?"
```

Expected: suite en verde, build sin advertencias, el script termina con `exit=0`. Revisar los 7 criterios de aceptación del spec uno por uno con la evidencia (capturas de la Tarea 7, tabla del script, `cargo test`); los criterios 3 y 4 con el mouse real y la música sonando los verifica el usuario.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/superpowers/specs/2026-10-03-visualizadores-de-particulas-design.md
git commit -m "Documenta los visualizadores de partículas y su benchmark

Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>"
```
