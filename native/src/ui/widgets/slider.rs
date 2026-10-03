/// Valor 0..1 que corresponde a la posición `x` del puntero sobre una pista
/// que empieza en `left` y mide `width`.
pub fn value_from_pointer(x: f32, left: f32, width: f32) -> f32 {
    if width <= 0.0 {
        return 0.0;
    }
    ((x - left) / width).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mapea_los_extremos_y_el_centro() {
        assert_eq!(value_from_pointer(100.0, 100.0, 200.0), 0.0);
        assert_eq!(value_from_pointer(300.0, 100.0, 200.0), 1.0);
        assert_eq!(value_from_pointer(200.0, 100.0, 200.0), 0.5);
    }

    #[test]
    fn fuera_de_la_pista_se_satura() {
        assert_eq!(value_from_pointer(-50.0, 100.0, 200.0), 0.0);
        assert_eq!(value_from_pointer(900.0, 100.0, 200.0), 1.0);
    }

    #[test]
    fn una_pista_sin_ancho_da_cero() {
        assert_eq!(value_from_pointer(5.0, 5.0, 0.0), 0.0);
    }
}
