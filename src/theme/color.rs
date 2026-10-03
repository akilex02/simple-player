use eframe::egui::{ecolor::Hsva, Color32};

/// Color vivo predominante de una imagen RGBA, o `None` si es casi todo
/// negro, blanco o gris (en ese caso la UI conserva su acento por defecto).
pub fn dominant_color(rgba: &[u8], w: usize, h: usize) -> Option<Color32> {
    const BINS: usize = 12;
    let mut weight = [0.0f32; BINS];
    let mut sum = [[0.0f32; 3]; BINS];
    let mut count = [0u32; BINS];

    for px in rgba.chunks_exact(4).take(w * h) {
        if px[3] < 128 {
            continue;
        }
        let hsva = Hsva::from_srgba_unmultiplied([px[0], px[1], px[2], 255]);
        if hsva.v < 0.15 || hsva.s < 0.2 {
            continue;
        }
        let bin = ((hsva.h * BINS as f32) as usize).min(BINS - 1);
        weight[bin] += hsva.s * hsva.v;
        count[bin] += 1;
        for c in 0..3 {
            sum[bin][c] += px[c] as f32;
        }
    }

    let (best, best_weight) = weight.iter().copied().enumerate().max_by(|a, b| a.1.total_cmp(&b.1))?;
    if best_weight < (w * h) as f32 * 0.02 || count[best] == 0 {
        return None;
    }
    let n = count[best] as f32;
    Some(Color32::from_rgb(
        (sum[best][0] / n).round() as u8,
        (sum[best][1] / n).round() as u8,
        (sum[best][2] / n).round() as u8,
    ))
}

/// Sube saturación y brillo hasta que el color sirva de acento sobre fondo oscuro.
pub fn ensure_vibrant(c: Color32) -> Color32 {
    let mut hsva = Hsva::from_srgba_unmultiplied([c.r(), c.g(), c.b(), 255]);
    hsva.s = hsva.s.max(0.45);
    hsva.v = hsva.v.max(0.75);
    let [r, g, b, _] = hsva.to_srgba_unmultiplied();
    Color32::from_rgb(r, g, b)
}

/// Mueve `current` hacia `target` con una constante de tiempo `tau`, independiente del dt.
#[cfg(test)]
pub fn approach_color(current: Color32, target: Color32, dt: f32, tau: f32) -> Color32 {
    let k = 1.0 - (-dt / tau).exp();
    let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * k).round() as u8;
    Color32::from_rgb(mix(current.r(), target.r()), mix(current.g(), target.g()), mix(current.b(), target.b()))
}

/// Como `approach_color` pero en canales flotantes (sin redondear en cada paso,
/// que deja el color atascado a ~10 unidades del destino) y con ajuste exacto
/// al llegar, para que el llamador pueda dejar de animar.
pub fn approach_rgb(current: [f32; 3], target: [f32; 3], dt: f32, tau: f32) -> [f32; 3] {
    let k = 1.0 - (-dt / tau).exp();
    let next: [f32; 3] = std::array::from_fn(|i| current[i] + (target[i] - current[i]) * k);
    if (0..3).all(|i| (next[i] - target[i]).abs() < 0.5) { target } else { next }
}

/// Luminancia relativa WCAG de un color opaco.
#[cfg(test)]
pub fn relative_luminance(c: Color32) -> f32 {
    let linear = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
}

/// Razón de contraste WCAG (1.0 a 21.0) entre dos colores opacos.
#[cfg(test)]
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let (la, lb) = (relative_luminance(a), relative_luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// `fg` (con su alfa) compuesto sobre un `bg` opaco.
#[cfg(test)]
pub fn over(fg: Color32, bg: Color32) -> Color32 {
    let [r, g, b, a] = fg.to_srgba_unmultiplied();
    let k = a as f32 / 255.0;
    let mix = |f: u8, back: u8| (f as f32 * k + back as f32 * (1.0 - k)).round() as u8;
    Color32::from_rgb(mix(r, bg.r()), mix(g, bg.g()), mix(b, bg.b()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(w: usize, h: usize, pixel: impl Fn(usize, usize) -> [u8; 4]) -> Vec<u8> {
        let mut out = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                out.extend(pixel(x, y));
            }
        }
        out
    }

    #[test]
    fn el_color_vivo_gana_al_gris() {
        let img = image(10, 10, |x, _| if x < 7 { [220, 30, 30, 255] } else { [128, 128, 128, 255] });
        let c = dominant_color(&img, 10, 10).expect("hay un color vivo");
        assert!(c.r() > 180 && c.g() < 80 && c.b() < 80, "{c:?}");
    }

    #[test]
    fn gana_el_matiz_con_mas_presencia() {
        let img = image(10, 10, |x, _| if x < 8 { [30, 60, 220, 255] } else { [220, 30, 30, 255] });
        let c = dominant_color(&img, 10, 10).unwrap();
        assert!(c.b() > c.r(), "{c:?}");
    }

    #[test]
    fn negro_blanco_y_gris_no_dan_acento() {
        assert_eq!(dominant_color(&image(8, 8, |_, _| [0, 0, 0, 255]), 8, 8), None);
        assert_eq!(dominant_color(&image(8, 8, |_, _| [255, 255, 255, 255]), 8, 8), None);
        assert_eq!(dominant_color(&image(8, 8, |_, _| [128, 128, 128, 255]), 8, 8), None);
    }

    #[test]
    fn un_acento_apagado_se_vuelve_vibrante() {
        let c = ensure_vibrant(Color32::from_rgb(40, 45, 60));
        let hsva = Hsva::from_srgba_unmultiplied([c.r(), c.g(), c.b(), 255]);
        assert!(hsva.s >= 0.44 && hsva.v >= 0.74, "{hsva:?}");
    }

    #[test]
    fn un_color_ya_vivo_no_cambia_de_matiz() {
        let c = ensure_vibrant(Color32::from_rgb(255, 0, 100));
        assert!(c.r() >= 250 && c.g() <= 10, "{c:?}");
    }

    #[test]
    fn approach_no_se_mueve_sin_tiempo_y_llega_con_mucho() {
        let (a, b) = (Color32::from_rgb(0, 0, 0), Color32::from_rgb(200, 100, 50));
        assert_eq!(approach_color(a, b, 0.0, 0.2), a);
        let far = approach_color(a, b, 5.0, 0.2);
        assert!(far.r() >= 198 && far.g() >= 99, "{far:?}");
    }

    #[test]
    fn approach_a_medio_camino_queda_entre_ambos() {
        let (a, b) = (Color32::from_rgb(0, 0, 0), Color32::from_rgb(200, 200, 200));
        let mid = approach_color(a, b, 0.2, 0.2);
        assert!(mid.r() > 60 && mid.r() < 140, "{mid:?}");
    }

    // ── Contraste (WCAG AA) ────────────────────────────────────────────────
    #[test]
    fn negro_sobre_blanco_es_21_a_1_y_un_color_consigo_mismo_1_a_1() {
        assert!((contrast_ratio(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.05);
        assert!((contrast_ratio(Color32::GRAY, Color32::GRAY) - 1.0).abs() < 1e-3);
    }

    #[test]
    fn mezclar_un_velo_sobre_un_fondo_interpola_por_el_alfa() {
        let mixed = over(Color32::from_rgba_unmultiplied(0, 0, 0, 128), Color32::WHITE);
        assert!((126..=129).contains(&mixed.r()), "{mixed:?}");
    }

    /// Fondo más claro posible: una carátula blanca bajo el velo del fondo vivo.
    fn worst_case_backdrop() -> Color32 {
        over(crate::theme::with_alpha(crate::theme::BG_BASE, crate::theme::BACKDROP_SCRIM_ALPHA), Color32::WHITE)
    }

    #[test]
    fn el_texto_principal_cumple_aaa_sobre_el_peor_fondo() {
        let ratio = contrast_ratio(crate::theme::TEXT_MAIN, worst_case_backdrop());
        assert!(ratio >= 7.0, "{ratio}");
    }

    #[test]
    fn el_texto_atenuado_cumple_aa_sobre_el_peor_fondo() {
        let ratio = contrast_ratio(crate::theme::TEXT_MUTED, worst_case_backdrop());
        assert!(ratio >= 4.5, "{ratio}");
    }

    #[test]
    fn el_acento_llega_exactamente_a_su_destino_a_60_fps_y_deja_de_animar() {
        let mut c = [255.0, 158.0, 189.0];
        let target = [30.0, 60.0, 200.0];
        let mut frames = 0;
        while c != target && frames < 600 {
            c = approach_rgb(c, target, 1.0 / 60.0, 0.35);
            frames += 1;
        }
        assert_eq!(c, target, "tras {frames} frames");
        assert!(frames < 300, "tarda demasiado: {frames}");
    }

    #[test]
    fn approach_rgb_no_se_mueve_sin_tiempo_y_avanza_con_tiempo() {
        let (a, b) = ([0.0; 3], [100.0; 3]);
        assert_eq!(approach_rgb(a, b, 0.0, 0.35), a);
        let step = approach_rgb(a, b, 0.016, 0.35);
        assert!(step[0] > 0.0 && step[0] < 100.0);
    }
}
