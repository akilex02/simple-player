use std::time::Duration;

/// Cuándo debe volver a dibujarse la app.
#[derive(Debug, PartialEq)]
pub enum Repaint {
    /// En el siguiente frame (animación o música sonando).
    Now,
    /// En reposo: un latido lento de seguridad. Atajos, MPRIS y carátulas
    /// despiertan a la UI por sí mismos.
    After(Duration),
}

pub const IDLE_HEARTBEAT: Duration = Duration::from_secs(1);
/// Por debajo de esta altura (0..1) se considera que las barras ya se apagaron.
const SETTLED_BAR: f32 = 0.004;

pub fn decide(playing: bool, viz_peak: f32) -> Repaint {
    if playing || viz_peak > SETTLED_BAR {
        Repaint::Now
    } else {
        Repaint::After(IDLE_HEARTBEAT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sonando_se_repinta_continuamente() {
        assert_eq!(decide(true, 0.0), Repaint::Now);
        assert_eq!(decide(true, 0.9), Repaint::Now);
    }

    #[test]
    fn en_pausa_con_barras_cayendo_se_sigue_repintando() {
        assert_eq!(decide(false, 0.3), Repaint::Now);
    }

    #[test]
    fn en_pausa_y_con_las_barras_apagadas_solo_hay_latido() {
        assert_eq!(decide(false, 0.0), Repaint::After(IDLE_HEARTBEAT));
        assert_eq!(decide(false, 0.003), Repaint::After(IDLE_HEARTBEAT));
    }
}
