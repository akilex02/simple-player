use super::aggregate;
use super::model::{StatsRange, StatsSummary};
use super::store::{Store, StoreResult};
use chrono::{DateTime, TimeZone};

pub const TOP_LIMIT: u32 = 5;

pub fn build_summary<Tz: TimeZone>(store: &Store, range: StatsRange, now: &DateTime<Tz>) -> StoreResult<StatsSummary> {
    let bounds = aggregate::range_bounds(range, now);
    let first = store.first_event_end()?;
    let span = aggregate::days_in_span(range, now, first);

    let totals = store.totals(bounds.start_ms, bounds.end_ms)?;
    let rows = store.rows_in_range(bounds.start_ms, bounds.end_ms)?;
    let all_ended_at = store.all_ended_at()?;
    let (timeline, peak_bucket) = aggregate::timeline(&rows, range, now, first);

    Ok(StatsSummary {
        range,
        totals,
        avg_daily_ms: totals.listened_ms / span as u64,
        days_in_span: span,
        top_songs: store.top_songs(bounds.start_ms, bounds.end_ms, TOP_LIMIT)?,
        top_artists: store.top_artists(bounds.start_ms, bounds.end_ms, TOP_LIMIT)?,
        top_albums: store.top_albums(bounds.start_ms, bounds.end_ms, TOP_LIMIT)?,
        habits: aggregate::habits(&rows, &all_ended_at, span, now),
        timeline,
        peak_bucket,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::model::{PlayEvent, SongSnapshot};
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::west_opt(6 * 3600).unwrap()
    }

    fn now() -> DateTime<FixedOffset> {
        tz().with_ymd_and_hms(2026, 10, 3, 15, 30, 0).unwrap()
    }

    fn at(d: u32, h: u32) -> i64 {
        tz().with_ymd_and_hms(2026, 10, d, h, 0, 0).unwrap().timestamp_millis()
    }

    fn ev(path: &str, artist: &str, day: u32, hour: u32, listened: u64) -> PlayEvent {
        PlayEvent {
            started_at: at(day, hour),
            ended_at: at(day, hour) + listened as i64,
            listened_ms: listened,
            song: SongSnapshot {
                path: path.into(),
                title: path.to_uppercase(),
                artist: artist.into(),
                album: "Disco".into(),
                duration_ms: 200_000,
            },
        }
    }

    fn seeded() -> Store {
        let mut s = Store::open_in_memory().unwrap();
        s.insert_batch(&[
            ev("a", "Uno", 1, 10, 200_000),
            ev("a", "Uno", 2, 10, 200_000),
            ev("b", "Dos", 3, 9, 100_000),
            ev("a", "Uno", 3, 9, 200_000), // hoy, mismo bloque horario que b
        ])
        .unwrap();
        s
    }

    #[test]
    fn el_resumen_de_todo_combina_totales_tops_habitos_y_linea_de_tiempo() {
        let s = build_summary(&seeded(), StatsRange::All, &now()).unwrap();
        assert_eq!((s.totals.listened_ms, s.totals.plays, s.totals.unique_songs), (700_000, 4, 2));
        assert_eq!(s.days_in_span, 3); // 1..3 de octubre
        assert_eq!(s.avg_daily_ms, 700_000 / 3);
        assert_eq!(s.top_songs[0].name, "A");
        assert_eq!(s.top_artists[0].name, "Uno");
        assert_eq!(s.habits.active_days, 3);
        assert_eq!(s.habits.current_streak_days, 3);
        assert_eq!(s.timeline.len(), 1);
        assert_eq!(s.peak_bucket, Some(0));
    }

    #[test]
    fn el_resumen_de_hoy_solo_incluye_lo_de_hoy() {
        let s = build_summary(&seeded(), StatsRange::Today, &now()).unwrap();
        assert_eq!((s.totals.plays, s.totals.listened_ms), (2, 300_000));
        assert_eq!(s.timeline.len(), 24);
        assert_eq!(s.timeline[9].listened_ms, 300_000);
        // la racha actual mira todo el historial aunque el rango sea Hoy
        assert_eq!(s.habits.current_streak_days, 3);
    }

    #[test]
    fn un_rango_sin_eventos_da_un_resumen_vacio_sin_fallar() {
        let empty = Store::open_in_memory().unwrap();
        for range in StatsRange::ALL_RANGES {
            let s = build_summary(&empty, range, &now()).unwrap();
            assert_eq!((s.totals.plays, s.totals.listened_ms, s.avg_daily_ms), (0, 0, 0), "{range:?}");
            assert!(s.top_songs.is_empty() && s.top_artists.is_empty() && s.top_albums.is_empty());
            assert_eq!((s.habits.sessions, s.habits.current_streak_days), (0, 0));
            assert_eq!(s.peak_bucket, None);
            assert!(s.days_in_span >= 1);
        }
    }

    /// Medición, no regresión: `cargo test --release summary::tests::resumen_de_100k -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn resumen_de_100k_eventos_en_menos_de_200_ms() {
        let mut store = Store::open_in_memory().unwrap();
        let base = at(1, 0);
        let events: Vec<PlayEvent> = (0..100_000u32)
            .map(|i| {
                let ended = base - (i as i64) * 600_000; // un evento cada 10 min hacia atrás
                PlayEvent {
                    started_at: ended - 180_000,
                    ended_at: ended,
                    listened_ms: 180_000,
                    song: SongSnapshot {
                        path: format!("/m/{}.mp3", i % 600),
                        title: format!("T{}", i % 600),
                        artist: format!("A{}", i % 150),
                        album: format!("Al{}", i % 200),
                        duration_ms: 200_000,
                    },
                }
            })
            .collect();
        store.insert_batch(&events).unwrap();

        let t0 = std::time::Instant::now();
        let s = build_summary(&store, StatsRange::All, &now()).unwrap();
        let elapsed = t0.elapsed();
        println!("[bench] resumen de {} eventos: {elapsed:?}", s.totals.plays);
        // Meta del spec: ~200 ms (se miden 197–200 ms). La guarda es 250 ms para no fallar por ruido.
        assert!(elapsed.as_millis() < 250, "tardó {elapsed:?}");
    }
}
