use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatsLocation {
    File(PathBuf),
    Memory,
}

pub fn default_db_path(xdg_data_home: Option<&str>, home: Option<&str>) -> PathBuf {
    let base = match xdg_data_home.filter(|v| !v.is_empty()) {
        Some(xdg) => PathBuf::from(xdg),
        None => PathBuf::from(home.unwrap_or("/tmp")).join(".local").join("share"),
    };
    base.join("simple-player").join("stats.db")
}

/// Dónde guardar según los argumentos: las corridas de desarrollo nunca tocan la base real.
pub fn location_from_args(args: &[String], xdg_data_home: Option<&str>, home: Option<&str>) -> StatsLocation {
    if let Some(i) = args.iter().position(|a| a == "--stats-db") {
        if let Some(path) = args.get(i + 1) {
            return StatsLocation::File(PathBuf::from(path));
        }
    }
    let dev = ["--stats-demo", "--shot", "--bench", "--gallery"].iter().any(|f| args.iter().any(|a| a == f));
    if dev {
        StatsLocation::Memory
    } else {
        StatsLocation::File(default_db_path(xdg_data_home, home))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn la_ruta_por_defecto_respeta_xdg_data_home() {
        assert_eq!(
            default_db_path(Some("/datos"), Some("/home/u")),
            PathBuf::from("/datos/simple-player/stats.db")
        );
    }

    #[test]
    fn sin_xdg_se_usa_local_share_del_home_y_un_xdg_vacio_se_ignora() {
        let esperado = PathBuf::from("/home/u/.local/share/simple-player/stats.db");
        assert_eq!(default_db_path(None, Some("/home/u")), esperado);
        assert_eq!(default_db_path(Some(""), Some("/home/u")), esperado);
    }

    #[test]
    fn una_corrida_normal_usa_la_base_real() {
        assert_eq!(
            location_from_args(&args(&["simple-player"]), None, Some("/home/u")),
            StatsLocation::File(PathBuf::from("/home/u/.local/share/simple-player/stats.db"))
        );
    }

    #[test]
    fn las_corridas_de_desarrollo_usan_memoria() {
        for flag in ["--shot", "--bench", "--gallery", "--stats-demo"] {
            assert_eq!(
                location_from_args(&args(&["simple-player", flag]), None, Some("/home/u")),
                StatsLocation::Memory,
                "{flag}"
            );
        }
    }

    #[test]
    fn stats_db_fuerza_una_ruta_incluso_en_desarrollo() {
        assert_eq!(
            location_from_args(&args(&["simple-player", "--shot", "--stats-db", "/tmp/x.db"]), None, None),
            StatsLocation::File(PathBuf::from("/tmp/x.db"))
        );
    }
}
