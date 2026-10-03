/// Desenfoque de caja (clamp en los bordes) sobre una imagen RGBA, en dos pasadas.
pub fn box_blur_rgba(src: &[u8], w: usize, h: usize, radius: usize) -> Vec<u8> {
    let mut horizontal = vec![0u8; src.len()];
    blur_pass(src, &mut horizontal, w, h, radius, true);
    let mut out = vec![0u8; src.len()];
    blur_pass(&horizontal, &mut out, w, h, radius, false);
    out
}

fn blur_pass(src: &[u8], dst: &mut [u8], w: usize, h: usize, radius: usize, horizontal: bool) {
    let (len, lines) = if horizontal { (w, h) } else { (h, w) };
    let window = (2 * radius + 1) as u32;
    let index = |line: usize, i: usize| {
        let (x, y) = if horizontal { (i, line) } else { (line, i) };
        (y * w + x) * 4
    };
    for line in 0..lines {
        for c in 0..4 {
            let mut acc: u32 = (0..=2 * radius)
                .map(|k| src[index(line, k.saturating_sub(radius).min(len - 1)) + c] as u32)
                .sum();
            for i in 0..len {
                dst[index(line, i) + c] = ((acc + window / 2) / window) as u8;
                let leaving = i.saturating_sub(radius);
                let entering = (i + radius + 1).min(len - 1);
                acc = acc + src[index(line, entering) + c] as u32 - src[index(line, leaving) + c] as u32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_imagen_uniforme_no_cambia() {
        let src: Vec<u8> = (0..81).flat_map(|_| [100, 150, 200, 255]).collect();
        let out = box_blur_rgba(&src, 9, 9, 2);
        assert_eq!(out.len(), src.len());
        assert!(out.chunks(4).all(|p| p[0].abs_diff(100) <= 1 && p[1].abs_diff(150) <= 1 && p[2].abs_diff(200) <= 1 && p[3] == 255));
    }

    fn impulse() -> Vec<u8> {
        let mut v = vec![0u8; 9 * 9 * 4];
        for px in v.chunks_mut(4) {
            px[3] = 255;
        }
        let c = (4 * 9 + 4) * 4;
        v[c..c + 3].copy_from_slice(&[255, 255, 255]);
        v
    }

    #[test]
    fn un_punto_brillante_se_reparte_a_los_vecinos() {
        let out = box_blur_rgba(&impulse(), 9, 9, 1);
        let at = |x: usize, y: usize| out[(y * 9 + x) * 4];
        assert!(at(4, 4) < 255 && at(4, 4) > 0);
        assert!(at(4, 3) > 0 && at(3, 4) > 0 && at(5, 5) > 0);
        assert_eq!(at(0, 0), 0);
    }

    #[test]
    fn el_desenfoque_conserva_la_luz_total() {
        let before: u32 = impulse().chunks(4).map(|p| p[0] as u32).sum();
        let after: u32 = box_blur_rgba(&impulse(), 9, 9, 1).chunks(4).map(|p| p[0] as u32).sum();
        let diff = before.abs_diff(after) as f32 / before as f32;
        assert!(diff < 0.05, "before {before} after {after}");
    }
}
