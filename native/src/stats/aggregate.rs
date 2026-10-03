use super::format::{month_short, weekday_full, weekday_short};
use super::model::{EventRow, Habits, StatsRange, TimelineBucket};
use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Timelike, Weekday};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RangeBounds {
    pub start_ms: Option<i64>,
    pub end_ms: i64,
}

/// Hueco máximo entre dos reproducciones para que sigan en la misma sesión.
pub const SESSION_GAP_MS: i64 = 30 * 60 * 1000;

/// Inicio del día local. Si el cambio de horario se come la medianoche, el día
/// empieza en la primera hora que sí existe (si no, el rango se volvería "todo").
fn day_start_ms<Tz: TimeZone>(tz: &Tz, date: NaiveDate) -> Option<i64> {
    (0..=3).find_map(|hour| {
        let naive = date.and_hms_opt(hour, 0, 0)?;
        tz.from_local_datetime(&naive).earliest().map(|d| d.timestamp_millis())
    })
}

fn date_of<Tz: TimeZone>(tz: &Tz, ms: i64) -> Option<NaiveDate> {
    tz.timestamp_millis_opt(ms).single().map(|d| d.date_naive())
}

fn local<Tz: TimeZone>(tz: &Tz, ms: i64) -> Option<DateTime<Tz>> {
    tz.timestamp_millis_opt(ms).single()
}

pub fn range_bounds<Tz: TimeZone>(range: StatsRange, now: &DateTime<Tz>) -> RangeBounds {
    let tz = now.timezone();
    let today = now.date_naive();
    let start_date = match range {
        StatsRange::Today => Some(today),
        StatsRange::Week => Some(today - Duration::days(today.weekday().num_days_from_monday() as i64)),
        StatsRange::Month => today.with_day(1),
        StatsRange::Year => today.with_ordinal(1),
        StatsRange::All => None,
    };
    RangeBounds {
        start_ms: start_date.and_then(|d| day_start_ms(&tz, d)),
        end_ms: now.timestamp_millis(),
    }
}

pub fn days_in_span<Tz: TimeZone>(range: StatsRange, now: &DateTime<Tz>, first_event_end_ms: Option<i64>) -> u32 {
    let today = now.date_naive();
    let days = match range {
        StatsRange::Today => 1,
        StatsRange::Week => today.weekday().num_days_from_monday() as i64 + 1,
        StatsRange::Month => today.day() as i64,
        StatsRange::Year => today.ordinal() as i64,
        StatsRange::All => first_event_end_ms
            .and_then(|ms| date_of(&now.timezone(), ms))
            .map(|first| (today - first).num_days() + 1)
            .unwrap_or(1),
    };
    days.max(1) as u32
}

fn longest_run(days: &BTreeSet<NaiveDate>) -> u32 {
    let (mut best, mut current, mut previous) = (0u32, 0u32, None::<NaiveDate>);
    for day in days {
        current = if previous.map_or(false, |p| p + Duration::days(1) == *day) { current + 1 } else { 1 };
        best = best.max(current);
        previous = Some(*day);
    }
    best
}

fn current_run(days: &BTreeSet<NaiveDate>, today: NaiveDate) -> u32 {
    let yesterday = today - Duration::days(1);
    let mut cursor = if days.contains(&today) {
        today
    } else if days.contains(&yesterday) {
        yesterday
    } else {
        return 0;
    };
    let mut run = 0;
    while days.contains(&cursor) {
        run += 1;
        cursor -= Duration::days(1);
    }
    run
}

pub fn habits<Tz: TimeZone>(
    rows: &[EventRow],
    all_ended_at: &[i64],
    days_in_span: u32,
    now: &DateTime<Tz>,
) -> Habits {
    let tz = now.timezone();
    let active: BTreeSet<NaiveDate> = rows.iter().filter_map(|r| date_of(&tz, r.ended_at)).collect();
    let history: BTreeSet<NaiveDate> = all_ended_at.iter().filter_map(|ms| date_of(&tz, *ms)).collect();

    // Sesiones: se une la reproducción siguiente si empieza a lo sumo 30 min después de que terminó la sesión.
    let mut sorted: Vec<&EventRow> = rows.iter().collect();
    sorted.sort_by_key(|r| r.started_at);
    let mut sessions: Vec<u64> = Vec::new();
    let mut session_end = i64::MIN;
    for r in sorted {
        if !sessions.is_empty() && r.started_at - session_end <= SESSION_GAP_MS {
            *sessions.last_mut().unwrap() += r.listened_ms;
            session_end = session_end.max(r.ended_at);
        } else {
            sessions.push(r.listened_ms);
            session_end = r.ended_at;
        }
    }
    let total: u64 = sessions.iter().sum();
    let count = sessions.len() as u32;

    let peak_weekday = if days_in_span > 1 {
        let mut by_day: BTreeMap<u32, (Weekday, u64)> = BTreeMap::new();
        for r in rows {
            if let Some(date) = date_of(&tz, r.ended_at) {
                let entry = by_day.entry(date.weekday().num_days_from_monday()).or_insert((date.weekday(), 0));
                entry.1 += r.listened_ms;
            }
        }
        by_day
            .values()
            .fold(None::<(Weekday, u64)>, |best, cur| match best {
                Some(b) if b.1 >= cur.1 => Some(b),
                _ => Some(*cur),
            })
            .filter(|(_, ms)| *ms > 0)
            .map(|(day, ms)| (weekday_full(day).to_string(), ms))
    } else {
        None
    };

    Habits {
        active_days: active.len() as u32,
        current_streak_days: current_run(&history, now.date_naive()),
        longest_streak_days: longest_run(&active),
        sessions: count,
        avg_session_ms: if count > 0 { total / count as u64 } else { 0 },
        longest_session_ms: sessions.iter().copied().max().unwrap_or(0),
        sessions_per_day: count as f64 / days_in_span.max(1) as f64,
        peak_weekday,
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let next = if month == 12 { NaiveDate::from_ymd_opt(year + 1, 1, 1) } else { NaiveDate::from_ymd_opt(year, month + 1, 1) };
    (next.unwrap() - first).num_days() as u32
}

pub fn timeline<Tz: TimeZone>(
    rows: &[EventRow],
    range: StatsRange,
    now: &DateTime<Tz>,
    first_event_end_ms: Option<i64>,
) -> (Vec<TimelineBucket>, Option<usize>) {
    let tz = now.timezone();
    let bucket = |label: String| TimelineBucket { label, listened_ms: 0 };

    // (etiquetas, función que lleva un instante local a su índice de bucket)
    let (mut buckets, index_of): (Vec<TimelineBucket>, Box<dyn Fn(&DateTime<Tz>) -> Option<usize>>) = match range {
        StatsRange::Today => ((0..24).map(|h| bucket(format!("{h:02}"))).collect(), Box::new(|d| Some(d.hour() as usize))),
        StatsRange::Week => (
            [Weekday::Mon, Weekday::Tue, Weekday::Wed, Weekday::Thu, Weekday::Fri, Weekday::Sat, Weekday::Sun]
                .iter()
                .map(|d| bucket(weekday_short(*d).to_string()))
                .collect(),
            Box::new(|d| Some(d.weekday().num_days_from_monday() as usize)),
        ),
        StatsRange::Month => (
            (1..=days_in_month(now.year(), now.month())).map(|d| bucket(d.to_string())).collect(),
            Box::new(|d| Some(d.day0() as usize)),
        ),
        StatsRange::Year => (
            (1..=12).map(|m| bucket(month_short(m).to_string())).collect(),
            Box::new(|d| Some(d.month0() as usize)),
        ),
        StatsRange::All => {
            let Some(first) = first_event_end_ms.and_then(|ms| local(&tz, ms)) else {
                return (Vec::new(), None);
            };
            let months = (now.year() - first.year()) * 12 + now.month0() as i32 - first.month0() as i32 + 1;
            if months <= 36 {
                let (y0, m0) = (first.year(), first.month0() as i32);
                let labels = (0..months)
                    .map(|i| {
                        let total = m0 + i;
                        let (year, month) = (y0 + total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
                        bucket(format!("{} {:02}", month_short(month), year.rem_euclid(100)))
                    })
                    .collect();
                (
                    labels,
                    Box::new(move |d| {
                        let i = (d.year() - y0) * 12 + d.month0() as i32 - m0;
                        usize::try_from(i).ok()
                    }),
                )
            } else {
                let y0 = first.year();
                let labels = (y0..=now.year()).map(|y| bucket(y.to_string())).collect();
                (labels, Box::new(move |d| usize::try_from(d.year() - y0).ok()))
            }
        }
    };

    for row in rows {
        if let Some(i) = local(&tz, row.ended_at).as_ref().and_then(|d| index_of(d)) {
            if let Some(b) = buckets.get_mut(i) {
                b.listened_ms += row.listened_ms;
            }
        }
    }
    let peak = buckets
        .iter()
        .enumerate()
        .filter(|(_, b)| b.listened_ms > 0)
        .fold(None::<(usize, u64)>, |best, (i, b)| match best {
            Some((_, v)) if v >= b.listened_ms => best,
            _ => Some((i, b.listened_ms)),
        })
        .map(|(i, _)| i);
    (buckets, peak)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::FixedOffset;

    fn tz() -> FixedOffset {
        FixedOffset::west_opt(6 * 3600).unwrap()
    }

    /// Milisegundos de una fecha-hora local en la zona de prueba.
    fn ms(y: i32, m: u32, d: u32, h: u32, min: u32) -> i64 {
        tz().with_ymd_and_hms(y, m, d, h, min, 0).unwrap().timestamp_millis()
    }

    fn now() -> DateTime<FixedOffset> {
        tz().with_ymd_and_hms(2026, 10, 3, 15, 30, 0).unwrap() // sábado
    }

    fn row(start: i64, end: i64, listened: u64) -> EventRow {
        EventRow { started_at: start, ended_at: end, listened_ms: listened }
    }

    fn ended_on(y: i32, m: u32, d: u32, h: u32) -> i64 {
        ms(y, m, d, h, 0)
    }

    // ── rangos ─────────────────────────────────────────────────────────────
    #[test]
    fn los_inicios_de_rango_son_medianoche_local_y_la_semana_empieza_el_lunes() {
        let n = now();
        assert_eq!(range_bounds(StatsRange::Today, &n).start_ms, Some(ms(2026, 10, 3, 0, 0)));
        assert_eq!(range_bounds(StatsRange::Week, &n).start_ms, Some(ms(2026, 9, 28, 0, 0)));
        assert_eq!(range_bounds(StatsRange::Month, &n).start_ms, Some(ms(2026, 10, 1, 0, 0)));
        assert_eq!(range_bounds(StatsRange::Year, &n).start_ms, Some(ms(2026, 1, 1, 0, 0)));
        assert_eq!(range_bounds(StatsRange::All, &n).start_ms, None);
        assert_eq!(range_bounds(StatsRange::Week, &n).end_ms, n.timestamp_millis());
    }

    #[test]
    fn la_semana_de_un_lunes_empieza_ese_mismo_dia() {
        let monday = tz().with_ymd_and_hms(2026, 9, 28, 9, 0, 0).unwrap();
        assert_eq!(range_bounds(StatsRange::Week, &monday).start_ms, Some(ms(2026, 9, 28, 0, 0)));
    }

    #[test]
    fn dias_del_rango() {
        let n = now();
        assert_eq!(days_in_span(StatsRange::Today, &n, None), 1);
        assert_eq!(days_in_span(StatsRange::Week, &n, None), 6); // lun..sáb
        assert_eq!(days_in_span(StatsRange::Month, &n, None), 3);
        assert_eq!(days_in_span(StatsRange::Year, &n, None), 276);
        assert_eq!(days_in_span(StatsRange::All, &n, Some(ms(2026, 9, 20, 10, 0))), 14);
    }

    #[test]
    fn todo_sin_eventos_cuenta_un_dia_sin_dividir_por_cero() {
        assert_eq!(days_in_span(StatsRange::All, &now(), None), 1);
    }

    // ── hábitos ────────────────────────────────────────────────────────────
    #[test]
    fn dias_activos_y_racha_mas_larga() {
        let rows = [
            row(0, ended_on(2026, 9, 28, 10), 60_000),
            row(0, ended_on(2026, 10, 1, 10), 60_000),
            row(0, ended_on(2026, 10, 2, 22), 60_000),
            row(0, ended_on(2026, 10, 3, 9), 60_000),
        ];
        let h = habits(&rows, &[], 6, &now());
        assert_eq!(h.active_days, 4);
        assert_eq!(h.longest_streak_days, 3);
    }

    #[test]
    fn la_racha_cruza_el_fin_de_mes() {
        let rows = [
            row(0, ended_on(2026, 9, 29, 10), 60_000),
            row(0, ended_on(2026, 9, 30, 10), 60_000),
            row(0, ended_on(2026, 10, 1, 10), 60_000),
        ];
        assert_eq!(habits(&rows, &[], 30, &now()).longest_streak_days, 3);
    }

    #[test]
    fn la_racha_actual_cuenta_desde_hoy_o_desde_ayer() {
        let con_hoy = [ended_on(2026, 10, 3, 9), ended_on(2026, 10, 2, 9), ended_on(2026, 10, 1, 9), ended_on(2026, 9, 29, 9)];
        assert_eq!(habits(&[], &con_hoy, 1, &now()).current_streak_days, 3);

        let desde_ayer = [ended_on(2026, 10, 2, 9), ended_on(2026, 10, 1, 9)];
        assert_eq!(habits(&[], &desde_ayer, 1, &now()).current_streak_days, 2);

        let rota = [ended_on(2026, 9, 30, 9)];
        assert_eq!(habits(&[], &rota, 1, &now()).current_streak_days, 0);
        assert_eq!(habits(&[], &[], 1, &now()).current_streak_days, 0);
    }

    #[test]
    fn un_hueco_de_exactamente_30_min_une_la_sesion_y_30_min_y_1_ms_la_separa() {
        let a = row(ms(2026, 10, 3, 10, 0), ms(2026, 10, 3, 10, 10), 600_000);
        let b = row(ms(2026, 10, 3, 10, 40), ms(2026, 10, 3, 10, 50), 600_000); // hueco 30 min justos
        let c = row(ms(2026, 10, 3, 11, 20) + 1, ms(2026, 10, 3, 11, 30), 300_000); // hueco 30 min + 1 ms
        let h = habits(&[a, b, c], &[], 1, &now());
        assert_eq!(h.sessions, 2);
        assert_eq!(h.longest_session_ms, 1_200_000);
        assert_eq!(h.avg_session_ms, (1_200_000 + 300_000) / 2);
        assert!((h.sessions_per_day - 2.0).abs() < 1e-9);
    }

    #[test]
    fn el_dia_pico_solo_existe_si_el_rango_abarca_mas_de_un_dia() {
        let rows = [
            row(0, ended_on(2026, 10, 3, 9), 3_000_000), // sábado
            row(0, ended_on(2026, 10, 1, 9), 1_000_000), // jueves
        ];
        let h = habits(&rows, &[], 6, &now());
        assert_eq!(h.peak_weekday, Some(("sábado".to_string(), 3_000_000)));
        assert_eq!(habits(&rows, &[], 1, &now()).peak_weekday, None);
    }

    #[test]
    fn sin_eventos_los_habitos_son_cero() {
        let h = habits(&[], &[], 6, &now());
        assert_eq!((h.active_days, h.sessions, h.longest_streak_days, h.current_streak_days), (0, 0, 0, 0));
        assert_eq!((h.avg_session_ms, h.longest_session_ms), (0, 0));
        assert_eq!(h.peak_weekday, None);
    }

    // ── línea de tiempo ────────────────────────────────────────────────────
    #[test]
    fn hoy_tiene_24_horas_y_marca_el_pico() {
        let rows = [row(0, ms(2026, 10, 3, 15, 30), 60_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::Today, &now(), None);
        assert_eq!(buckets.len(), 24);
        assert_eq!((buckets[0].label.as_str(), buckets[23].label.as_str()), ("00", "23"));
        assert_eq!(buckets[15].listened_ms, 60_000);
        assert_eq!(peak, Some(15));
    }

    #[test]
    fn la_semana_tiene_7_dias_de_lunes_a_domingo() {
        let rows = [row(0, ms(2026, 9, 30, 12, 0), 90_000)]; // miércoles
        let (buckets, peak) = timeline(&rows, StatsRange::Week, &now(), None);
        let labels: Vec<&str> = buckets.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"]);
        assert_eq!((buckets[2].listened_ms, peak), (90_000, Some(2)));
    }

    #[test]
    fn el_mes_tiene_un_bucket_por_dia() {
        let rows = [row(0, ms(2026, 10, 3, 12, 0), 10_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::Month, &now(), None);
        assert_eq!(buckets.len(), 31); // octubre
        assert_eq!((buckets[0].label.as_str(), buckets[30].label.as_str()), ("1", "31"));
        assert_eq!((buckets[2].listened_ms, peak), (10_000, Some(2)));
    }

    #[test]
    fn el_ano_tiene_12_meses() {
        let rows = [row(0, ms(2026, 10, 3, 12, 0), 10_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::Year, &now(), None);
        assert_eq!(buckets.len(), 12);
        assert_eq!((buckets[0].label.as_str(), buckets[11].label.as_str()), ("ene", "dic"));
        assert_eq!(peak, Some(9));
    }

    #[test]
    fn todo_va_por_mes_si_el_historial_cabe_en_36_meses() {
        let first = ms(2026, 8, 15, 10, 0);
        let rows = [row(0, ms(2026, 9, 10, 10, 0), 5_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::All, &now(), Some(first));
        let labels: Vec<&str> = buckets.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["ago 26", "sep 26", "oct 26"]);
        assert_eq!(peak, Some(1));
    }

    #[test]
    fn todo_va_por_ano_si_el_historial_supera_36_meses() {
        let first = ms(2022, 5, 1, 10, 0);
        let rows = [row(0, ms(2024, 3, 1, 10, 0), 5_000)];
        let (buckets, peak) = timeline(&rows, StatsRange::All, &now(), Some(first));
        let labels: Vec<&str> = buckets.iter().map(|b| b.label.as_str()).collect();
        assert_eq!(labels, ["2022", "2023", "2024", "2025", "2026"]);
        assert_eq!(peak, Some(2));
    }

    #[test]
    fn todo_sin_eventos_no_tiene_linea_de_tiempo_ni_pico() {
        let (buckets, peak) = timeline(&[], StatsRange::All, &now(), None);
        assert!(buckets.is_empty());
        assert_eq!(peak, None);
    }

    #[test]
    fn sin_tiempo_en_ningun_bucket_no_hay_pico() {
        let (buckets, peak) = timeline(&[], StatsRange::Week, &now(), None);
        assert_eq!(buckets.len(), 7);
        assert_eq!(peak, None);
    }

    #[test]
    fn un_evento_que_termina_justo_a_medianoche_pertenece_al_dia_nuevo() {
        let rows = [row(0, ms(2026, 10, 3, 0, 0), 60_000)];
        let (buckets, _) = timeline(&rows, StatsRange::Today, &now(), None);
        assert_eq!(buckets[0].listened_ms, 60_000);
        let (week, _) = timeline(&rows, StatsRange::Week, &now(), None);
        assert_eq!(week[5].listened_ms, 60_000); // sábado, no viernes
    }

    // ── medianoche inexistente (cambio de horario) ─────────────────────────
    use chrono::{Duration as ChronoDuration, LocalResult, NaiveDate, NaiveDateTime, Utc};

    /// UTC−4 hasta el 2026-10-03 00:00 local, cuando los relojes saltan a 01:00 (UTC−3):
    /// la medianoche de ese día no existe.
    #[derive(Clone, Copy, Debug)]
    struct GapZone;

    impl GapZone {
        fn gap_start() -> NaiveDateTime {
            NaiveDate::from_ymd_opt(2026, 10, 3).unwrap().and_hms_opt(0, 0, 0).unwrap()
        }
    }

    impl TimeZone for GapZone {
        type Offset = FixedOffset;

        fn from_offset(_: &FixedOffset) -> Self {
            GapZone
        }

        fn offset_from_local_date(&self, date: &NaiveDate) -> LocalResult<FixedOffset> {
            self.offset_from_local_datetime(&date.and_hms_opt(12, 0, 0).unwrap())
        }

        fn offset_from_local_datetime(&self, local: &NaiveDateTime) -> LocalResult<FixedOffset> {
            let gap_end = Self::gap_start() + ChronoDuration::hours(1);
            if *local < Self::gap_start() {
                LocalResult::Single(FixedOffset::west_opt(4 * 3600).unwrap())
            } else if *local < gap_end {
                LocalResult::None
            } else {
                LocalResult::Single(FixedOffset::west_opt(3 * 3600).unwrap())
            }
        }

        fn offset_from_utc_date(&self, utc: &NaiveDate) -> FixedOffset {
            self.offset_from_utc_datetime(&utc.and_hms_opt(12, 0, 0).unwrap())
        }

        fn offset_from_utc_datetime(&self, utc: &NaiveDateTime) -> FixedOffset {
            let transition = NaiveDate::from_ymd_opt(2026, 10, 3).unwrap().and_hms_opt(4, 0, 0).unwrap();
            FixedOffset::west_opt(if *utc >= transition { 3 } else { 4 } * 3600).unwrap()
        }
    }

    #[test]
    fn si_la_medianoche_no_existe_el_dia_empieza_en_la_primera_hora_valida() {
        let now = GapZone.timestamp_millis_opt(Utc.with_ymd_and_hms(2026, 10, 3, 18, 0, 0).unwrap().timestamp_millis()).unwrap();
        let first_valid_instant = Utc.with_ymd_and_hms(2026, 10, 3, 4, 0, 0).unwrap().timestamp_millis(); // 01:00 local
        assert_eq!(range_bounds(StatsRange::Today, &now).start_ms, Some(first_valid_instant));
        // un rango acotado nunca debe degradarse a "sin límite"
        for range in [StatsRange::Today, StatsRange::Week, StatsRange::Month, StatsRange::Year] {
            assert!(range_bounds(range, &now).start_ms.is_some(), "{range:?}");
        }
    }
}
