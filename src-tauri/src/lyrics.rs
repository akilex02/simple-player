use lofty::file::TaggedFileExt;
use lofty::probe::Probe;
use lofty::tag::ItemKey;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SyncedLine {
    pub time_ms: u64,
    pub text: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(tag = "kind", content = "lines")]
pub enum Lyrics {
    Synced(Vec<SyncedLine>),
    Plain(Vec<String>),
}

/// [mm:ss.xx] o [mm:ss:xx] al inicio de línea
fn parse_lrc_timestamp(tag: &str) -> Option<u64> {
    let tag = tag.trim_start_matches('[').trim_end_matches(']');
    let (mins, rest) = tag.split_once(':')?;
    let (secs, frac) = rest.split_once(['.', ':']).unwrap_or((rest, "0"));

    let mins: u64 = mins.trim().parse().ok()?;
    let secs: u64 = secs.trim().parse().ok()?;
    let frac_digits = frac.trim();
    let frac_ms: u64 = frac_digits.parse().ok()?;
    // 2 dígitos = centésimas, 3 dígitos = milésimas
    let frac_ms = if frac_digits.len() <= 2 { frac_ms * 10 } else { frac_ms };

    Some(mins * 60_000 + secs * 1_000 + frac_ms)
}

/// Convierte texto LRC en líneas sincronizadas; si no hay marcas de tiempo, lo trata como texto plano.
pub fn parse_lyrics(text: &str) -> Lyrics {
    let mut synced = Vec::new();
    let mut plain = Vec::new();
    let mut is_synced = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') {
            if let Some(end) = line.find(']') {
                let tag = &line[..=end];
                if let Some(time_ms) = parse_lrc_timestamp(tag) {
                    is_synced = true;
                    let content = line[end + 1..].trim().to_string();
                    synced.push(SyncedLine { time_ms, text: content });
                    continue;
                }
                // Línea de metadata como [ar:Artista] o [id:...] — se ignora
                if tag.len() > 2 && tag[1..tag.len() - 1].contains(':') {
                    continue;
                }
            }
        }

        plain.push(line.to_string());
    }

    if is_synced && !synced.is_empty() {
        synced.sort_by_key(|l| l.time_ms);
        Lyrics::Synced(synced)
    } else {
        Lyrics::Plain(plain)
    }
}

fn read_local_lrc_file(song_path: &Path) -> Option<String> {
    let dir = song_path.parent()?;
    let stem = song_path.file_stem()?.to_str()?;
    let lrc_path = dir.join(format!("{}.lrc", stem));
    fs::read_to_string(lrc_path).ok()
}

fn read_embedded_lyrics(song_path: &Path) -> Option<String> {
    let tagged_file = Probe::open(song_path).ok()?.read().ok()?;
    let tag = tagged_file.primary_tag().or_else(|| tagged_file.first_tag())?;
    tag.get_string(&ItemKey::Lyrics)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Prioridad: archivo .lrc local junto a la canción, luego letra embebida en los metadatos.
pub fn get_lyrics_for_song(song_path: &str) -> Option<Lyrics> {
    let path = Path::new(song_path);

    let raw = read_local_lrc_file(path).or_else(|| read_embedded_lyrics(path))?;
    Some(parse_lyrics(&raw))
}
