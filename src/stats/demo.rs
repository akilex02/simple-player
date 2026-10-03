use super::model::{PlayEvent, SongSnapshot};

pub fn demo_events(now_ms: i64, songs: &[SongSnapshot]) -> Vec<PlayEvent> {
    const DAY: i64 = 86_400_000;
    const HOUR: i64 = 3_600_000;

    let fallback: Vec<SongSnapshot>;
    let pool: &[SongSnapshot] = if songs.is_empty() {
        fallback = (0..12u64)
            .map(|i| SongSnapshot {
                path: format!("/demo/{i}.mp3"),
                title: format!("Canción {}", i + 1),
                artist: format!("Artista {}", i % 4 + 1),
                album: format!("Álbum {}", i % 3 + 1),
                duration_ms: 180_000 + i * 7_000,
            })
            .collect();
        &fallback
    } else {
        songs
    };

    // xorshift con semilla fija: la demo es idéntica en cada corrida.
    let mut state = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };

    let mut events = Vec::new();
    for day in 0..30i64 {
        if day != 0 && day % 6 == 5 {
            continue; // algunos días sin escuchar, para que haya rachas rotas
        }
        let sessions = 1 + (next() % 3) as i64;
        for s in 0..sessions {
            let back = (s + 1) * 4 * HOUR + (next() % 1_800_000) as i64;
            let mut cursor = now_ms - day * DAY - back;
            for _ in 0..(3 + next() % 7) {
                let r = (next() % 1000) as f64 / 1000.0;
                let song = &pool[(((r * r) * pool.len() as f64) as usize).min(pool.len() - 1)]; // sesga hacia las primeras
                let full = if song.duration_ms > 0 { song.duration_ms } else { 200_000 };
                let listened = full * (60 + next() % 41) / 100;
                let ended = cursor + listened as i64;
                if ended > now_ms {
                    break;
                }
                events.push(PlayEvent { started_at: cursor, ended_at: ended, listened_ms: listened, song: song.clone() });
                cursor = ended + (next() % 20_000) as i64;
            }
        }
    }
    events.sort_by_key(|e| e.ended_at);
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_790_000_000_000;
    const DAY: i64 = 86_400_000;

    #[test]
    fn genera_un_mes_de_escucha_valida_y_ordenada() {
        let events = demo_events(NOW, &[]);
        assert!(events.len() > 100, "{}", events.len());
        assert!(events.iter().all(|e| e.ended_at <= NOW && e.listened_ms >= 5_000 && e.started_at <= e.ended_at));
        assert!(events.windows(2).all(|w| w[0].ended_at <= w[1].ended_at));
        let days: std::collections::BTreeSet<i64> = events.iter().map(|e| e.ended_at / DAY).collect();
        assert!(days.len() >= 20, "{}", days.len());
    }

    #[test]
    fn es_determinista_y_usa_las_canciones_dadas() {
        let songs = vec![SongSnapshot {
            path: "/m/x.mp3".into(),
            title: "X".into(),
            artist: "Y".into(),
            album: "Z".into(),
            duration_ms: 240_000,
        }];
        let a = demo_events(NOW, &songs);
        let b = demo_events(NOW, &songs);
        assert_eq!(a, b);
        assert!(a.iter().all(|e| e.song.path == "/m/x.mp3"));
    }
}
