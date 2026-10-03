//! Física de partículas de los visualizadores. Sin egui: se prueba sin ventana.
use super::bands::Bands;
use std::collections::HashMap;

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
const THERMAL: f32 = 0.04;
const BASS_PUSH: f32 = 0.2;
/// El spec decía 0.5 (valor del original con teclas de ajuste); a ese valor casi no se nota.
const PULL_G: f32 = 1.5;
const PULL_MIN_DIST: f32 = 25.0;
const SPRING_HELD: f32 = 0.005;
const SPRING_SNAP: f32 = 0.055;

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
    /// Anillo y Osciloscopio nacen en su destino (en el primer paso) en vez de volar hacia él.
    needs_snap: bool,
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
        Self { kind, particles, bounds, rng, needs_snap: kind != FieldKind::Free }
    }

    #[cfg(test)]
    pub fn set_positions_for_tests(&mut self, positions: &[V2]) {
        self.particles.truncate(positions.len());
        for (p, pos) in self.particles.iter_mut().zip(positions) {
            p.pos = *pos;
            p.home = *pos;
        }
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
    pub fn step(&mut self, dt: f32, bands: Bands, anchor: Anchor, pull: Option<Pull>, time: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        let k = (dt / FRAME).min(MAX_STEPS);
        self.update_homes(bands, anchor, time);
        if std::mem::take(&mut self.needs_snap) {
            for p in &mut self.particles {
                p.pos = p.home;
                p.vel = V2::default();
            }
        }
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
}

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

    #[test]
    fn el_anillo_y_el_osciloscopio_nacen_ya_formados() {
        for kind in [FieldKind::Ring, FieldKind::Wave] {
            let mut field = ParticleField::new(kind, 360, bounds(), 3);
            field.step(FRAME, Bands::default(), anchor(), None, 0.0);
            let worst = field.particles().iter().map(|p| p.pos.dist(p.home)).fold(0.0, f32::max);
            assert!(worst < 3.0, "{kind:?}: la peor partícula está a {worst} px de su destino");
        }
    }

    #[test]
    fn el_modo_libre_no_se_acomoda_en_el_primer_paso() {
        let mut field = ParticleField::new(FieldKind::Free, 100, bounds(), 3);
        let before: Vec<V2> = field.particles().iter().map(|p| p.pos).collect();
        field.step(FRAME, Bands::default(), anchor(), None, 0.0);
        let moved = field.particles().iter().zip(&before).filter(|(p, b)| p.pos != **b).count();
        assert!(moved > 50, "solo {moved} de 100 se movieron por su velocidad");
    }
}
