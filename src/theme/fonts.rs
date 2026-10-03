//! Armado de las fuentes: GTA Art Deco en toda la interfaz; Noto Sans solo como respaldo de glifos.
use eframe::egui::{FontData, FontDefinitions, FontFamily};

pub const FONT_REGULAR: &str = "GTAArtDeco";
pub const FONT_BOLD: &str = "GTAArtDecoBold";
pub const FONT_CONDENSED: &str = "GTAArtDecoCondensed";
const FONT_FALLBACK: &str = "NotoSans";

pub fn build_font_definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let files: &[(&str, &[u8])] = &[
        (FONT_REGULAR, include_bytes!("../../assets/fonts/GTAArtDeco_Regular.ttf")),
        (FONT_BOLD, include_bytes!("../../assets/fonts/GTAArtDeco_Bold.ttf")),
        (FONT_CONDENSED, include_bytes!("../../assets/fonts/GTAArtDeco_CondensedBold.ttf")),
        (FONT_FALLBACK, include_bytes!("../../assets/fonts/NotoSans-Regular.ttf")),
    ];
    for (name, bytes) in files {
        fonts.font_data.insert(name.to_string(), FontData::from_static(bytes));
    }

    // Interfaz: Art Deco primero; luego Noto Sans (respaldo de glifos), Phosphor (íconos) y las
    // fuentes por defecto de egui, que aportan cobertura de emoji.
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, FONT_REGULAR.into());
    proportional.insert(1, FONT_FALLBACK.into());
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);

    let fallback: Vec<String> =
        fonts.families[&FontFamily::Proportional].iter().filter(|name| name.as_str() != FONT_REGULAR).cloned().collect();
    for primary in [FONT_BOLD, FONT_CONDENSED] {
        let mut chain = vec![primary.to_string()];
        chain.extend(fallback.iter().cloned());
        fonts.families.insert(FontFamily::Name(primary.into()), chain);
    }
    fonts
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::FontFamily;

    fn chain(fonts: &eframe::egui::FontDefinitions, family: FontFamily) -> Vec<String> {
        fonts.families.get(&family).cloned().unwrap_or_default()
    }

    #[test]
    fn la_interfaz_normal_empieza_con_art_deco_regular() {
        let fonts = build_font_definitions();
        assert_eq!(chain(&fonts, FontFamily::Proportional)[0], FONT_REGULAR);
    }

    #[test]
    fn las_negritas_empiezan_con_art_deco_bold() {
        let fonts = build_font_definitions();
        assert_eq!(chain(&fonts, FontFamily::Name(FONT_BOLD.into()))[0], FONT_BOLD);
    }

    #[test]
    fn los_titulos_empiezan_con_art_deco_condensado() {
        let fonts = build_font_definitions();
        assert_eq!(chain(&fonts, FontFamily::Name(FONT_CONDENSED.into()))[0], FONT_CONDENSED);
    }

    #[test]
    fn noto_sans_queda_como_respaldo_en_las_tres_familias() {
        let fonts = build_font_definitions();
        for family in [FontFamily::Proportional, FontFamily::Name(FONT_BOLD.into()), FontFamily::Name(FONT_CONDENSED.into())] {
            let names = chain(&fonts, family.clone());
            let at = names.iter().position(|n| n == "NotoSans");
            assert!(matches!(at, Some(i) if i > 0), "{family:?}: {names:?}");
        }
    }

    #[test]
    fn los_iconos_de_phosphor_siguen_disponibles() {
        let fonts = build_font_definitions();
        assert!(chain(&fonts, FontFamily::Proportional).iter().any(|n| n == "phosphor"));
        assert!(chain(&fonts, FontFamily::Name(FONT_BOLD.into())).iter().any(|n| n == "phosphor"));
    }

    #[test]
    fn toda_fuente_de_una_cadena_tiene_sus_datos() {
        let fonts = build_font_definitions();
        for (family, names) in &fonts.families {
            for name in names {
                assert!(fonts.font_data.contains_key(name), "{family:?} usa «{name}» sin datos");
            }
        }
    }
}
