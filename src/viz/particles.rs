#![allow(dead_code)] // se quita al conectar el módulo (Tarea 7)
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
