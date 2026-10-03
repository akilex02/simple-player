use crate::library::Song;

/// Datos de la canción congelados al empezar a sonar: el historial sobrevive
/// aunque el archivo se mueva o se borre.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SongSnapshot {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_ms: u64,
}

impl From<&Song> for SongSnapshot {
    fn from(song: &Song) -> Self {
        Self {
            path: song.path.clone(),
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            duration_ms: song.duration_secs * 1000,
        }
    }
}

/// Una reproducción terminada. Tiempos en milisegundos desde epoch UTC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayEvent {
    pub started_at: i64,
    pub ended_at: i64,
    pub listened_ms: u64,
    pub song: SongSnapshot,
}

/// Fila compacta de un evento, suficiente para hábitos y línea de tiempo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventRow {
    pub started_at: i64,
    pub ended_at: i64,
    pub listened_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatsRange {
    Today,
    Week,
    Month,
    Year,
    All,
}

impl StatsRange {
    pub const ALL_RANGES: [StatsRange; 5] =
        [StatsRange::Today, StatsRange::Week, StatsRange::Month, StatsRange::Year, StatsRange::All];

    pub fn label(self) -> &'static str {
        match self {
            StatsRange::Today => "Hoy",
            StatsRange::Week => "Semana",
            StatsRange::Month => "Mes",
            StatsRange::Year => "Año",
            StatsRange::All => "Todo",
        }
    }
}

/// Una fila de los rankings. `name` queda vacío si la etiqueta falta; la UI lo muestra como "desconocido".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopEntry {
    pub name: String,
    /// Artista (para canciones y álbumes); vacío en el ranking de artistas.
    pub detail: String,
    /// Ruta de una canción de referencia, para resolver la carátula en la UI.
    pub reference_path: String,
    pub listened_ms: u64,
    pub plays: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimelineBucket {
    pub label: String,
    pub listened_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Habits {
    pub active_days: u32,
    pub current_streak_days: u32,
    pub longest_streak_days: u32,
    pub sessions: u32,
    pub avg_session_ms: u64,
    pub longest_session_ms: u64,
    pub sessions_per_day: f64,
    /// (nombre completo del día, tiempo total); `None` si el rango abarca un solo día.
    pub peak_weekday: Option<(String, u64)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Totals {
    pub listened_ms: u64,
    pub plays: u32,
    pub unique_songs: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StatsSummary {
    pub range: StatsRange,
    pub totals: Totals,
    pub avg_daily_ms: u64,
    pub days_in_span: u32,
    pub top_songs: Vec<TopEntry>,
    pub top_artists: Vec<TopEntry>,
    pub top_albums: Vec<TopEntry>,
    pub habits: Habits,
    pub timeline: Vec<TimelineBucket>,
    pub peak_bucket: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_snapshot_convierte_segundos_a_milisegundos() {
        let song = Song {
            path: "/m/a.mp3".into(),
            title: "A".into(),
            artist: "B".into(),
            album: "C".into(),
            duration_secs: 200,
            cover_art: None,
        };
        let snap = SongSnapshot::from(&song);
        assert_eq!(snap.duration_ms, 200_000);
        assert_eq!((snap.path.as_str(), snap.title.as_str()), ("/m/a.mp3", "A"));
    }

    #[test]
    fn los_rangos_tienen_etiqueta_y_orden_fijos() {
        let labels: Vec<&str> = StatsRange::ALL_RANGES.iter().map(|r| r.label()).collect();
        assert_eq!(labels, ["Hoy", "Semana", "Mes", "Año", "Todo"]);
    }
}
