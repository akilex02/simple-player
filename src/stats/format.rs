use chrono::Weekday;

/// "41 h 12 min", "1 h", "45 min", "3 min 20 s", "45 s", "0 min".
pub fn format_duration(ms: u64) -> String {
    let secs = ms / 1000;
    let minutes = secs / 60;
    let (hours, rest_minutes) = (minutes / 60, minutes % 60);
    if hours > 0 {
        return if rest_minutes == 0 { format!("{hours} h") } else { format!("{hours} h {rest_minutes} min") };
    }
    if secs == 0 {
        return "0 min".to_string();
    }
    if secs < 60 {
        return format!("{secs} s");
    }
    // Bajo 10 min se muestran los segundos; desde ahí solo minutos.
    if secs < 600 && secs % 60 != 0 {
        return format!("{minutes} min {} s", secs % 60);
    }
    format!("{minutes} min")
}

pub fn weekday_short(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Lun",
        Weekday::Tue => "Mar",
        Weekday::Wed => "Mié",
        Weekday::Thu => "Jue",
        Weekday::Fri => "Vie",
        Weekday::Sat => "Sáb",
        Weekday::Sun => "Dom",
    }
}

pub fn weekday_full(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "lunes",
        Weekday::Tue => "martes",
        Weekday::Wed => "miércoles",
        Weekday::Thu => "jueves",
        Weekday::Fri => "viernes",
        Weekday::Sat => "sábado",
        Weekday::Sun => "domingo",
    }
}

/// `month` de 1 a 12.
pub fn month_short(month: u32) -> &'static str {
    const MONTHS: [&str; 12] = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];
    MONTHS[(month.clamp(1, 12) - 1) as usize]
}

/// El nombre tal cual, o `fallback` si está vacío o solo tiene espacios.
pub fn unknown_if_empty(name: &str, fallback: &str) -> String {
    if name.trim().is_empty() { fallback.to_string() } else { name.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duraciones_largas_y_cortas() {
        assert_eq!(format_duration(0), "0 min");
        assert_eq!(format_duration(45_000), "45 s");
        assert_eq!(format_duration(180_000), "3 min");
        assert_eq!(format_duration(200_000), "3 min 20 s");
        assert_eq!(format_duration(600_000), "10 min");
        assert_eq!(format_duration(2_700_000), "45 min");
        assert_eq!(format_duration(3_600_000), "1 h");
        assert_eq!(format_duration(5_400_000), "1 h 30 min");
        assert_eq!(format_duration(148_320_000), "41 h 12 min");
    }

    #[test]
    fn por_encima_de_10_min_se_ignoran_los_segundos() {
        assert_eq!(format_duration(605_000), "10 min");
        assert_eq!(format_duration(3_659_000), "1 h");
    }

    #[test]
    fn nombres_de_dias_y_meses_en_espanol() {
        assert_eq!(weekday_short(Weekday::Mon), "Lun");
        assert_eq!(weekday_short(Weekday::Wed), "Mié");
        assert_eq!(weekday_short(Weekday::Sat), "Sáb");
        assert_eq!(weekday_full(Weekday::Sat), "sábado");
        assert_eq!(weekday_full(Weekday::Wed), "miércoles");
        assert_eq!(month_short(1), "ene");
        assert_eq!(month_short(9), "sep");
        assert_eq!(month_short(12), "dic");
    }

    #[test]
    fn un_nombre_vacio_se_muestra_como_desconocido() {
        assert_eq!(unknown_if_empty("", "Artista desconocido"), "Artista desconocido");
        assert_eq!(unknown_if_empty("   ", "Artista desconocido"), "Artista desconocido");
        assert_eq!(unknown_if_empty("Toto", "Artista desconocido"), "Toto");
    }
}
