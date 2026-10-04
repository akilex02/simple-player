/// Anchos de las columnas de la tabla de canciones.
#[derive(Debug, PartialEq)]
pub struct TableColumns {
    pub index: f32,
    pub title: f32,
    /// 0 cuando no hay espacio: la columna se oculta.
    pub album: f32,
    pub duration: f32,
}

pub fn column_widths(avail: f32) -> TableColumns {
    const INDEX: f32 = 48.0;
    const DURATION: f32 = 72.0;
    const MIN_FOR_ALBUM: f32 = 560.0;
    let rest = (avail - INDEX - DURATION).max(0.0);
    if rest >= MIN_FOR_ALBUM {
        let album = (rest * 0.38).max(200.0);
        TableColumns { index: INDEX, title: rest - album, album, duration: DURATION }
    } else {
        TableColumns { index: INDEX, title: rest, album: 0.0, duration: DURATION }
    }
}

/// Qué dibujar de la lista de canciones dentro de un solo scroll (hero, encabezado y filas virtualizadas).
#[derive(Debug, PartialEq)]
pub struct ListLayout {
    pub first_row: usize,
    /// Exclusiva.
    pub last_row: usize,
    /// Alto vacío que sustituye a las filas no dibujadas antes de `first_row`.
    pub space_above: f32,
    /// Y el de las posteriores a `last_row`.
    pub space_below: f32,
    /// El hero ya salió de la vista: aparecen la mini barra y el encabezado fijos.
    pub pinned: bool,
}

/// Todas las medidas están en coordenadas del contenido del scroll: `scroll_top` es el desplazamiento,
/// `rows_top` donde empiezan las filas y `hero_end` donde termina el hero (con su separación).
pub fn list_layout(scroll_top: f32, viewport_h: f32, rows_top: f32, hero_end: f32, row_pitch: f32, rows: usize) -> ListLayout {
    let pinned = scroll_top >= hero_end;
    let visible_bottom = scroll_top + viewport_h;
    if rows == 0 || row_pitch <= 0.0 || visible_bottom <= rows_top {
        return ListLayout { first_row: 0, last_row: 0, space_above: 0.0, space_below: rows as f32 * row_pitch.max(0.0), pinned };
    }
    let visible_top = scroll_top.max(rows_top);
    let first_row = (((visible_top - rows_top) / row_pitch).floor() as usize).min(rows);
    let last_row = (((visible_bottom - rows_top) / row_pitch).ceil() as usize).clamp(first_row, rows);
    ListLayout {
        first_row,
        last_row,
        space_above: first_row as f32 * row_pitch,
        space_below: (rows - last_row) as f32 * row_pitch,
        pinned,
    }
}

/// Reparto de una grilla de tarjetas que se adapta al ancho disponible.
#[derive(Debug, PartialEq)]
pub struct GridLayout {
    pub cols: usize,
    pub card_w: f32,
    pub rows: usize,
}

pub fn grid_layout(avail: f32, min_card_w: f32, gap: f32, n: usize) -> GridLayout {
    let cols = (((avail + gap) / (min_card_w + gap)).floor() as usize).max(1);
    let card_w = (avail - gap * (cols as f32 - 1.0)) / cols as f32;
    GridLayout { cols, card_w, rows: n.div_ceil(cols) }
}

/// "41 h 12 min", "1 h", "45 min".
pub fn format_total_duration(secs: u64) -> String {
    let minutes = secs / 60;
    let (hours, rest) = (minutes / 60, minutes % 60);
    match (hours, rest) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

/// Alturas (0..1) de las tres barras del ecualizador de la fila activa.
pub fn eq_heights(t: f32, playing: bool) -> [f32; 3] {
    if !playing {
        return [0.35, 0.6, 0.45];
    }
    const SPEED: [f32; 3] = [5.0, 6.7, 4.2];
    const PHASE: [f32; 3] = [0.0, 1.3, 2.6];
    std::array::from_fn(|i| 0.25 + 0.75 * (0.5 + 0.5 * (t * SPEED[i] + PHASE[i]).sin()))
}

/// Qué muestra la pantalla de canciones; define los textos del encabezado.
pub enum HeroContext<'a> {
    Library,
    Search(&'a str),
    Artist(&'a str),
    Album { album: &'a str, artist: &'a str },
}

#[derive(Debug, PartialEq)]
pub struct HeroText {
    pub eyebrow: String,
    pub title: String,
    pub subtitle: String,
}

pub fn songs_label(count: usize) -> String {
    if count == 1 { "1 canción".to_string() } else { format!("{count} canciones") }
}

fn or_unknown(name: &str, unknown: &str) -> String {
    if name.trim().is_empty() { unknown.to_string() } else { name.to_string() }
}

pub fn hero_text(context: HeroContext, count: usize, total_secs: u64) -> HeroText {
    let summary = format!("{} · {}", songs_label(count), format_total_duration(total_secs));
    match context {
        HeroContext::Library => HeroText { eyebrow: "BIBLIOTECA".into(), title: "Toda la música".into(), subtitle: summary },
        HeroContext::Search(query) => {
            HeroText { eyebrow: "RESULTADOS".into(), title: format!("«{query}»"), subtitle: summary }
        }
        HeroContext::Artist(artist) => {
            HeroText { eyebrow: "ARTISTA".into(), title: or_unknown(artist, "Artista desconocido"), subtitle: summary }
        }
        HeroContext::Album { album, artist } => HeroText {
            eyebrow: "ÁLBUM".into(),
            title: or_unknown(album, "Álbum desconocido"),
            subtitle: format!("{} · {summary}", or_unknown(artist, "Artista desconocido")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columnas_en_pantalla_ancha_reparten_titulo_y_album() {
        let c = column_widths(1200.0);
        assert_eq!((c.index, c.duration), (48.0, 72.0));
        assert!(c.album >= 200.0 && c.title > c.album, "{c:?}");
        assert!((c.index + c.title + c.album + c.duration - 1200.0).abs() < 1.0, "{c:?}");
    }

    #[test]
    fn en_pantalla_estrecha_se_oculta_el_album_y_el_titulo_toma_el_resto() {
        let c = column_widths(520.0);
        assert_eq!(c.album, 0.0);
        assert!((c.index + c.title + c.duration - 520.0).abs() < 1.0, "{c:?}");
    }

    #[test]
    fn el_titulo_nunca_queda_con_ancho_negativo() {
        assert!(column_widths(50.0).title >= 0.0);
    }

    #[test]
    fn la_grilla_calcula_columnas_ancho_de_tarjeta_y_filas() {
        let g = grid_layout(1000.0, 180.0, 16.0, 25);
        assert_eq!(g.cols, 5);
        assert!((g.card_w - (1000.0 - 4.0 * 16.0) / 5.0).abs() < 1e-3, "{g:?}");
        assert_eq!(g.rows, 5);
    }

    #[test]
    fn la_grilla_siempre_tiene_al_menos_una_columna() {
        let g = grid_layout(100.0, 180.0, 16.0, 3);
        assert_eq!(g.cols, 1);
        assert_eq!(g.rows, 3);
    }

    #[test]
    fn una_grilla_vacia_no_tiene_filas() {
        assert_eq!(grid_layout(1000.0, 180.0, 16.0, 0).rows, 0);
    }

    #[test]
    fn duracion_total_legible() {
        assert_eq!(format_total_duration(0), "0 min");
        assert_eq!(format_total_duration(2700), "45 min");
        assert_eq!(format_total_duration(3600), "1 h");
        assert_eq!(format_total_duration(5400), "1 h 30 min");
        assert_eq!(format_total_duration(148_320), "41 h 12 min");
    }

    #[test]
    fn el_ecualizador_en_pausa_queda_quieto() {
        assert_eq!(eq_heights(0.0, false), eq_heights(5.3, false));
    }

    #[test]
    fn el_ecualizador_sonando_se_mueve_y_se_mantiene_en_rango() {
        let (a, b) = (eq_heights(0.0, true), eq_heights(0.4, true));
        assert_ne!(a, b);
        assert_ne!(a[0], a[1]);
        for h in a.iter().chain(b.iter()) {
            assert!((0.2..=1.0).contains(h), "{h}");
        }
    }

    fn text(e: &str, t: &str, s: &str) -> HeroText {
        HeroText { eyebrow: e.into(), title: t.into(), subtitle: s.into() }
    }

    #[test]
    fn el_encabezado_de_la_biblioteca_resume_canciones_y_duracion() {
        assert_eq!(
            hero_text(HeroContext::Library, 628, 148_320),
            text("BIBLIOTECA", "Toda la música", "628 canciones · 41 h 12 min")
        );
    }

    #[test]
    fn el_encabezado_de_busqueda_muestra_la_consulta() {
        assert_eq!(hero_text(HeroContext::Search("rock"), 3, 600), text("RESULTADOS", "«rock»", "3 canciones · 10 min"));
    }

    #[test]
    fn una_sola_cancion_va_en_singular() {
        assert_eq!(hero_text(HeroContext::Artist("Toto"), 1, 295), text("ARTISTA", "Toto", "1 canción · 4 min"));
    }

    #[test]
    fn album_y_artista_sin_nombre_se_muestran_como_desconocidos() {
        let t = hero_text(HeroContext::Album { album: "", artist: "" }, 2, 120);
        assert_eq!(t, text("ÁLBUM", "Álbum desconocido", "Artista desconocido · 2 canciones · 2 min"));
        assert_eq!(hero_text(HeroContext::Artist(""), 2, 120).title, "Artista desconocido");
    }

    fn layout(scroll: f32) -> ListLayout {
        // hero de 164 + 16 de separación, encabezado de 28 → las filas empiezan en 208; 100 filas de 56.
        list_layout(scroll, 500.0, 208.0, 180.0, 56.0, 100)
    }

    #[test]
    fn arriba_del_todo_se_ve_el_hero_y_las_primeras_filas() {
        let l = layout(0.0);
        assert!(!l.pinned);
        assert_eq!(l.first_row, 0);
        // la ventana llega a 500: quedan 292 para filas → 6 filas (5.2 redondeado hacia arriba).
        assert_eq!(l.last_row, 6);
        assert_eq!(l.space_above, 0.0);
    }

    #[test]
    fn con_el_hero_fuera_de_vista_se_fija_la_barra_y_empiezan_las_filas_visibles() {
        let l = layout(560.0);
        assert!(l.pinned);
        // fila superior visible: (560 - 208) / 56 = 6.28 → fila 6
        assert_eq!(l.first_row, 6);
        assert_eq!(l.space_above, 6.0 * 56.0);
        assert!(l.last_row > l.first_row);
    }

    #[test]
    fn se_fija_justo_cuando_el_hero_termina_de_salir() {
        assert!(!layout(179.9).pinned);
        assert!(layout(180.0).pinned);
    }

    #[test]
    fn nunca_se_piden_filas_fuera_de_la_lista_ni_se_pierde_altura() {
        for scroll in [0.0, 100.0, 1000.0, 5000.0, 5700.0, 9999.0] {
            let l = layout(scroll);
            assert!(l.first_row <= l.last_row && l.last_row <= 100, "{scroll}: {l:?}");
            let total = l.space_above + (l.last_row - l.first_row) as f32 * 56.0 + l.space_below;
            assert!((total - 100.0 * 56.0).abs() < 0.01, "{scroll}: {total}");
        }
    }

    #[test]
    fn sin_filas_o_con_el_hero_ocupando_toda_la_ventana_no_hay_filas_que_dibujar() {
        let none = list_layout(0.0, 500.0, 208.0, 180.0, 56.0, 0);
        assert_eq!((none.first_row, none.last_row, none.space_above, none.space_below), (0, 0, 0.0, 0.0));
        let tall_hero = list_layout(0.0, 150.0, 208.0, 180.0, 56.0, 100);
        assert_eq!((tall_hero.first_row, tall_hero.last_row), (0, 0));
        assert_eq!(tall_hero.space_below, 100.0 * 56.0);
    }
}
