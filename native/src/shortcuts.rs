pub const SEEK_STEP_SECS: f64 = 5.0;
pub const VOLUME_STEP: f64 = 0.05;

/// Destino de un seek relativo, acotado a la duración de la pista.
pub fn seek_target(current: f64, delta: f64, duration: f64) -> f64 {
    let target = current + delta;
    if duration > 0.0 { target.clamp(0.0, duration) } else { target.max(0.0) }
}

/// Volumen tras un paso, acotado a 0..1 y sin arrastrar error de redondeo.
pub fn step_volume(volume: f64, delta: f64) -> f64 {
    ((volume + delta).clamp(0.0, 1.0) * 100.0).round() / 100.0
}

/// Cambio de un slider enfocado con las flechas (`shift` = paso grande).
pub fn adjust_with_keys(value: f32, left: bool, right: bool, shift: bool) -> f32 {
    let step = if shift { 0.1 } else { 0.02 };
    let delta = match (left, right) {
        (true, false) => -step,
        (false, true) => step,
        _ => return value,
    };
    (((value + delta).clamp(0.0, 1.0)) * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_seek_relativo_se_acota_al_inicio_y_al_final() {
        assert_eq!(seek_target(30.0, 5.0, 200.0), 35.0);
        assert_eq!(seek_target(30.0, -5.0, 200.0), 25.0);
        assert_eq!(seek_target(2.0, -5.0, 200.0), 0.0);
        assert_eq!(seek_target(198.0, 5.0, 200.0), 200.0);
    }

    #[test]
    fn sin_duracion_conocida_solo_se_acota_el_inicio() {
        assert_eq!(seek_target(10.0, 5.0, 0.0), 15.0);
        assert_eq!(seek_target(2.0, -5.0, 0.0), 0.0);
    }

    #[test]
    fn el_volumen_sube_y_baja_en_pasos_y_se_acota() {
        assert_eq!(step_volume(0.5, VOLUME_STEP), 0.55);
        assert_eq!(step_volume(0.98, VOLUME_STEP), 1.0);
        assert_eq!(step_volume(0.02, -VOLUME_STEP), 0.0);
    }

    #[test]
    fn el_volumen_no_acumula_error_de_punto_flotante() {
        let mut v = 0.0;
        for _ in 0..20 {
            v = step_volume(v, VOLUME_STEP);
        }
        assert_eq!(v, 1.0);
        for _ in 0..7 {
            v = step_volume(v, -VOLUME_STEP);
        }
        assert_eq!(v, 0.65);
    }

    #[test]
    fn las_flechas_ajustan_el_slider_con_paso_chico_o_grande() {
        assert_eq!(adjust_with_keys(0.5, false, true, false), 0.52);
        assert_eq!(adjust_with_keys(0.5, true, false, false), 0.48);
        assert_eq!(adjust_with_keys(0.5, false, true, true), 0.6);
        assert_eq!(adjust_with_keys(0.99, false, true, true), 1.0);
        assert_eq!(adjust_with_keys(0.01, true, false, false), 0.0);
        assert_eq!(adjust_with_keys(0.5, false, false, false), 0.5);
        assert_eq!(adjust_with_keys(0.5, true, true, false), 0.5);
    }
}
