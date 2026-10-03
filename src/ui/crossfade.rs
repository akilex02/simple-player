/// Transición entre dos valores (p. ej. dos carátulas de fondo): el anterior
/// se desvanece mientras el nuevo aparece.
pub struct Crossfade<T> {
    current: Option<T>,
    previous: Option<T>,
    progress: f32,
    duration: f32,
}

impl<T: PartialEq + Clone> Crossfade<T> {
    pub fn new(duration: f32) -> Self {
        Self { current: None, previous: None, progress: 1.0, duration }
    }

    pub fn set_target(&mut self, target: Option<T>) {
        if target == self.current {
            return;
        }
        self.previous = self.current.take();
        self.current = target;
        self.progress = 0.0;
    }

    pub fn tick(&mut self, dt: f32) {
        if self.progress >= 1.0 {
            return;
        }
        self.progress = (self.progress + dt / self.duration).min(1.0);
        if self.progress >= 1.0 {
            self.previous = None;
        }
    }

    pub fn current(&self) -> Option<&T> {
        self.current.as_ref()
    }

    pub fn previous(&self) -> Option<&T> {
        self.previous.as_ref()
    }

    fn eased(&self) -> f32 {
        self.progress * self.progress * (3.0 - 2.0 * self.progress)
    }

    pub fn current_alpha(&self) -> f32 {
        self.eased()
    }

    pub fn previous_alpha(&self) -> f32 {
        if self.previous.is_some() { 1.0 - self.eased() } else { 0.0 }
    }

    pub fn is_animating(&self) -> bool {
        self.progress < 1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_primer_valor_aparece_con_fundido_de_entrada() {
        let mut f = Crossfade::new(0.4);
        f.set_target(Some("a"));
        assert_eq!(f.current(), Some(&"a"));
        assert_eq!(f.previous(), None);
        assert_eq!(f.current_alpha(), 0.0);
        f.tick(0.4);
        assert_eq!(f.current_alpha(), 1.0);
        assert!(!f.is_animating());
    }

    #[test]
    fn al_cambiar_el_anterior_se_desvanece_mientras_el_nuevo_aparece() {
        let mut f = Crossfade::new(0.4);
        f.set_target(Some("a"));
        f.tick(1.0);
        f.set_target(Some("b"));
        assert_eq!(f.previous(), Some(&"a"));
        assert_eq!((f.previous_alpha(), f.current_alpha()), (1.0, 0.0));
        f.tick(0.2);
        assert!((f.current_alpha() - 0.5).abs() < 1e-3 && (f.previous_alpha() - 0.5).abs() < 1e-3);
        f.tick(0.2);
        assert_eq!(f.previous(), None);
        assert_eq!(f.current_alpha(), 1.0);
    }

    #[test]
    fn repetir_el_mismo_valor_no_reinicia_la_transicion() {
        let mut f = Crossfade::new(0.4);
        f.set_target(Some("a"));
        f.tick(1.0);
        f.set_target(Some("a"));
        assert!(!f.is_animating());
        assert_eq!(f.previous(), None);
    }

    #[test]
    fn quitar_el_valor_desvanece_el_anterior() {
        let mut f = Crossfade::new(0.4);
        f.set_target(Some("a"));
        f.tick(1.0);
        f.set_target(None);
        assert_eq!(f.current(), None);
        assert_eq!(f.previous(), Some(&"a"));
        assert!(f.is_animating());
        f.tick(0.4);
        assert_eq!(f.previous(), None);
    }
}
