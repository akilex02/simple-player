use crate::audio::AudioPlayer;
use crate::library::{scan_music_folder, select_folder, Song};
use crate::lyrics::{self, Lyrics};
use crate::mpris::MprisMsg;
use crate::persistence::{load_playback_state, save_playback_state, PlaybackState};
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::sync::mpsc::SyncSender;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveTab {
    All,
    Artists,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortField {
    Title,
    Artist,
    Album,
    Duration,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    fn toggle(self) -> Self {
        match self {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RepeatMode {
    Off,
    All,
    One,
}

impl RepeatMode {
    fn next(self) -> Self {
        match self {
            RepeatMode::Off => RepeatMode::All,
            RepeatMode::All => RepeatMode::One,
            RepeatMode::One => RepeatMode::Off,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            RepeatMode::Off => "off",
            RepeatMode::All => "all",
            RepeatMode::One => "one",
        }
    }

    fn from_str(s: &str) -> Self {
        match s {
            "all" => RepeatMode::All,
            "one" => RepeatMode::One,
            _ => RepeatMode::Off,
        }
    }
}

pub struct ArtistGroup {
    pub artist: String,
    pub count: usize,
    pub representative_cover: Option<String>,
}

fn shuffle_vec(items: &mut Vec<Song>) {
    items.shuffle(&mut rand::thread_rng());
}

pub struct AppState {
    pub audio: AudioPlayer,
    mpris_tx: SyncSender<MprisMsg>,

    pub songs: Vec<Song>,
    pub current_song_index: Option<usize>,
    pub active_queue: Vec<Song>,
    pub is_playing: bool,
    pub volume: f64,
    last_volume: f64,
    pub is_muted: bool,

    pub search_query: String,
    pub current_time: f64,
    pub is_dragging_seek: bool,
    pub loading: bool,
    pub current_folder_path: Option<String>,

    pub active_tab: ActiveTab,
    pub selected_artist: Option<String>,

    pub sort_field: SortField,
    pub sort_direction: SortDirection,
    pub artist_sort_order: SortDirection,

    pub is_shuffle: bool,
    pub repeat_mode: RepeatMode,
    pub is_normalize_volume: bool,

    pub is_fullscreen: bool,
    pub show_lyrics: bool,
    pub lyrics: Option<Lyrics>,
    lyrics_loaded_for: Option<String>,

    last_position_poll: Instant,
}

impl AppState {
    pub fn new(audio: AudioPlayer, mpris_tx: SyncSender<MprisMsg>) -> Self {
        Self {
            audio,
            mpris_tx,
            songs: Vec::new(),
            current_song_index: None,
            active_queue: Vec::new(),
            is_playing: false,
            volume: 0.8,
            last_volume: 0.8,
            is_muted: false,
            search_query: String::new(),
            current_time: 0.0,
            is_dragging_seek: false,
            loading: false,
            current_folder_path: None,
            active_tab: ActiveTab::All,
            selected_artist: None,
            sort_field: SortField::Title,
            sort_direction: SortDirection::Asc,
            artist_sort_order: SortDirection::Asc,
            is_shuffle: false,
            repeat_mode: RepeatMode::Off,
            is_normalize_volume: true,
            is_fullscreen: false,
            show_lyrics: false,
            lyrics: None,
            lyrics_loaded_for: None,
            last_position_poll: Instant::now(),
        }
    }

    /// Carga la biblioteca (desde caché) y restaura el estado persistido —
    /// equivalente al `useEffect` de inicio en App.tsx.
    pub fn init(&mut self) {
        self.loading = true;
        self.songs = scan_music_folder(None);
        self.loading = false;

        let Some(saved) = load_playback_state() else { return };

        if saved.folder_path.is_some() {
            self.current_folder_path = saved.folder_path;
        }
        self.is_shuffle = saved.is_shuffle;
        self.repeat_mode = RepeatMode::from_str(&saved.repeat_mode);

        if !saved.queue_paths.is_empty() {
            let by_path: HashMap<&str, &Song> =
                self.songs.iter().map(|s| (s.path.as_str(), s)).collect();
            let restored_queue: Vec<Song> = saved
                .queue_paths
                .iter()
                .filter_map(|p| by_path.get(p.as_str()).map(|s| (*s).clone()))
                .collect();

            if !restored_queue.is_empty() {
                let restored_index = saved
                    .current_index
                    .filter(|&i| i < restored_queue.len())
                    .unwrap_or(0);
                self.active_queue = restored_queue;
                self.current_song_index = Some(restored_index);
            }
        }

        self.volume = saved.volume;
        let _ = self.audio.set_volume(self.volume);
    }

    fn persist(&self) {
        let state = PlaybackState {
            folder_path: self.current_folder_path.clone(),
            queue_paths: self.active_queue.iter().map(|s| s.path.clone()).collect(),
            current_index: self.current_song_index,
            volume: self.volume,
            is_shuffle: self.is_shuffle,
            repeat_mode: self.repeat_mode.as_str().to_string(),
        };
        let _ = save_playback_state(&state);
    }

    fn notify_mpris(&self) {
        let Some(song) = self.current_song() else { return };
        let _ = self.mpris_tx.try_send(MprisMsg::Update {
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            cover_path: song.cover_art.clone(),
            duration_secs: song.duration_secs,
            position_secs: self.current_time,
            is_playing: self.is_playing,
        });
    }

    // ─── Datos derivados ────────────────────────────────────────────────────

    /// Búsqueda con scoring tipo "Strawberry": tokeniza por espacios, suma
    /// puntos por coincidencia en título/artista/álbum, descarta lo que no
    /// matchea *todos* los tokens, ordena por relevancia.
    pub fn filtered_songs(&self) -> Vec<&Song> {
        let base: Vec<&Song> = match &self.selected_artist {
            Some(artist) => self.songs.iter().filter(|s| &s.artist == artist).collect(),
            None => self.songs.iter().collect(),
        };

        let query = self.search_query.trim().to_lowercase();
        if query.is_empty() {
            return base;
        }
        let tokens: Vec<&str> = query.split_whitespace().collect();

        let mut scored: Vec<(i32, &Song)> = base
            .into_iter()
            .filter_map(|s| {
                let title_l = s.title.to_lowercase();
                let artist_l = s.artist.to_lowercase();
                let album_l = s.album.to_lowercase();
                let mut score = 0;
                for tok in &tokens {
                    if title_l.starts_with(tok) {
                        score += 10;
                    } else if title_l.contains(&format!(" {tok}")) {
                        score += 8;
                    } else if title_l.contains(tok) {
                        score += 5;
                    }
                    if artist_l.starts_with(tok) {
                        score += 9;
                    } else if artist_l.contains(tok) {
                        score += 4;
                    }
                    if album_l.starts_with(tok) {
                        score += 7;
                    } else if album_l.contains(tok) {
                        score += 3;
                    }
                }
                let all_match = tokens
                    .iter()
                    .all(|tok| title_l.contains(tok) || artist_l.contains(tok) || album_l.contains(tok));
                all_match.then_some((score, s))
            })
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, s)| s).collect()
    }

    pub fn sorted_songs(&self) -> Vec<&Song> {
        let filtered = self.filtered_songs();
        if !self.search_query.trim().is_empty() {
            return filtered; // ya viene ordenado por relevancia
        }

        let mut result = filtered;
        result.sort_by(|a, b| {
            let cmp = match self.sort_field {
                SortField::Title => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
                SortField::Artist => a.artist.to_lowercase().cmp(&b.artist.to_lowercase()),
                SortField::Album => a.album.to_lowercase().cmp(&b.album.to_lowercase()),
                SortField::Duration => a.duration_secs.cmp(&b.duration_secs),
            };
            match self.sort_direction {
                SortDirection::Asc => cmp,
                SortDirection::Desc => cmp.reverse(),
            }
        });
        result
    }

    pub fn artist_groups(&self) -> Vec<ArtistGroup> {
        let query = self.search_query.trim().to_lowercase();
        let tokens: Vec<&str> = query.split_whitespace().collect();

        let songs_to_group: Vec<&Song> = if tokens.is_empty() {
            self.songs.iter().collect()
        } else {
            self.songs
                .iter()
                .filter(|s| {
                    let title_l = s.title.to_lowercase();
                    let artist_l = s.artist.to_lowercase();
                    let album_l = s.album.to_lowercase();
                    tokens
                        .iter()
                        .all(|tok| title_l.contains(tok) || artist_l.contains(tok) || album_l.contains(tok))
                })
                .collect()
        };

        let mut map: HashMap<String, Vec<&Song>> = HashMap::new();
        for song in songs_to_group {
            let artist = if song.artist.trim().is_empty() {
                "Artista Desconocido".to_string()
            } else {
                song.artist.clone()
            };
            map.entry(artist).or_default().push(song);
        }

        let mut groups: Vec<ArtistGroup> = map
            .into_iter()
            .map(|(artist, songs)| {
                let representative_cover = songs
                    .iter()
                    .find(|s| s.cover_art.is_some())
                    .and_then(|s| s.cover_art.clone());
                ArtistGroup {
                    artist,
                    count: songs.len(),
                    representative_cover,
                }
            })
            .collect();

        groups.sort_by(|a, b| {
            let cmp = a.artist.to_lowercase().cmp(&b.artist.to_lowercase());
            match self.artist_sort_order {
                SortDirection::Asc => cmp,
                SortDirection::Desc => cmp.reverse(),
            }
        });
        groups
    }

    /// Fuente de verdad de "canción actual": viene de `active_queue`, no de
    /// `sorted_songs`, para sobrevivir a cambios de filtro/orden de la tabla.
    pub fn current_song(&self) -> Option<&Song> {
        self.current_song_index.and_then(|i| self.active_queue.get(i))
    }

    // ─── Reproducción ───────────────────────────────────────────────────────

    /// Índice circular (wraparound) sobre la cola dada (o la activa si no se pasa una nueva).
    pub fn play_index(&mut self, index: i64, queue_to_use: Option<Vec<Song>>) {
        let queue = queue_to_use.unwrap_or_else(|| self.active_queue.clone());
        if queue.is_empty() {
            return;
        }
        let len = queue.len() as i64;
        let valid_index = ((index % len) + len) % len;
        let song = queue[valid_index as usize].clone();

        self.active_queue = queue;
        self.current_song_index = Some(valid_index as usize);
        self.current_time = 0.0;

        let _ = self.audio.play(&song.path);
        self.is_playing = true;

        self.persist();
        self.notify_mpris();
    }

    pub fn handle_play_song_from_list(&mut self, song: &Song, source_queue: Option<Vec<Song>>) {
        let base_queue = source_queue.unwrap_or_else(|| {
            if !self.active_queue.is_empty() {
                self.active_queue.clone()
            } else {
                self.sorted_songs().into_iter().cloned().collect()
            }
        });
        if base_queue.is_empty() {
            return;
        }

        if self.is_shuffle {
            let mut others: Vec<Song> = base_queue.into_iter().filter(|s| s.path != song.path).collect();
            shuffle_vec(&mut others);
            let mut new_queue = vec![song.clone()];
            new_queue.extend(others);
            self.play_index(0, Some(new_queue));
        } else {
            let idx = base_queue.iter().position(|s| s.path == song.path).unwrap_or(0);
            self.play_index(idx as i64, Some(base_queue));
        }
    }

    pub fn start_shuffle_play(&mut self, song_list: Vec<Song>) {
        if song_list.is_empty() {
            return;
        }
        let mut shuffled = song_list;
        shuffle_vec(&mut shuffled);
        self.is_shuffle = true;
        self.play_index(0, Some(shuffled));
    }

    pub fn toggle_shuffle(&mut self) {
        let cur_song = self.current_song().cloned();

        if !self.is_shuffle {
            let base_list: Vec<Song> = if !self.active_queue.is_empty() {
                self.active_queue.clone()
            } else {
                self.sorted_songs().into_iter().cloned().collect()
            };
            if base_list.is_empty() {
                return;
            }

            let shuffled = if let Some(cur) = &cur_song {
                let mut others: Vec<Song> = base_list.into_iter().filter(|s| s.path != cur.path).collect();
                shuffle_vec(&mut others);
                let mut result = vec![cur.clone()];
                result.extend(others);
                result
            } else {
                let mut result = base_list;
                shuffle_vec(&mut result);
                result
            };

            self.is_shuffle = true;
            self.active_queue = shuffled;
            self.current_song_index = Some(0);
            self.persist();
        } else {
            let base_list: Vec<Song> = self.sorted_songs().into_iter().cloned().collect();
            self.is_shuffle = false;
            let new_idx = cur_song
                .as_ref()
                .and_then(|cur| base_list.iter().position(|s| s.path == cur.path))
                .unwrap_or(0);
            self.active_queue = base_list;
            self.current_song_index = Some(new_idx);
            self.persist();
        }
    }

    pub fn toggle_play_pause(&mut self) {
        if self.current_song_index.is_none() {
            if !self.active_queue.is_empty() {
                self.play_index(0, None);
            } else {
                let sorted: Vec<Song> = self.sorted_songs().into_iter().cloned().collect();
                if let Some(first) = sorted.first().cloned() {
                    self.handle_play_song_from_list(&first, Some(sorted));
                }
            }
            return;
        }

        if self.is_playing {
            let _ = self.audio.pause();
            self.is_playing = false;
        } else {
            if let Some(song) = self.current_song().cloned() {
                let pos = self.audio.position_secs();
                if pos == 0 {
                    let _ = self.audio.play(&song.path);
                } else {
                    let _ = self.audio.resume();
                }
            }
            self.is_playing = true;
        }
        self.notify_mpris();
    }

    fn queue_len(&self) -> usize {
        if !self.active_queue.is_empty() {
            self.active_queue.len()
        } else {
            self.sorted_songs().len()
        }
    }

    pub fn handle_auto_next_song(&mut self) {
        let len = self.queue_len();
        if len == 0 {
            return;
        }
        let cur_idx = self.current_song_index.unwrap_or(0) as i64;

        if self.repeat_mode == RepeatMode::One {
            self.play_index(cur_idx, None);
            return;
        }
        if self.repeat_mode == RepeatMode::Off && cur_idx as usize == len - 1 {
            let _ = self.audio.pause();
            self.is_playing = false;
            return;
        }
        self.play_index(cur_idx + 1, None);
    }

    pub fn handle_next_song(&mut self) {
        if self.queue_len() == 0 {
            return;
        }
        let cur_idx = self.current_song_index.unwrap_or(0) as i64;
        self.play_index(cur_idx + 1, None);
    }

    pub fn handle_prev_song(&mut self) {
        if self.queue_len() == 0 {
            return;
        }
        let cur_idx = self.current_song_index.unwrap_or(0) as i64;
        self.play_index(cur_idx - 1, None);
    }

    pub fn seek_commit(&mut self, new_secs: f64) {
        self.is_dragging_seek = false;
        self.current_time = new_secs;
        let _ = self.audio.seek(new_secs);
    }

    pub fn toggle_mute(&mut self) {
        if self.is_muted {
            self.is_muted = false;
            self.volume = self.last_volume;
            let _ = self.audio.set_volume(self.volume);
        } else {
            self.last_volume = self.volume;
            self.is_muted = true;
            self.volume = 0.0;
            let _ = self.audio.set_volume(0.0);
        }
    }

    pub fn set_volume(&mut self, new_vol: f64) {
        self.volume = new_vol;
        if new_vol > 0.0 {
            self.is_muted = false;
        }
        let _ = self.audio.set_volume(new_vol);
    }

    pub fn toggle_repeat_mode(&mut self) {
        self.repeat_mode = self.repeat_mode.next();
        self.persist();
    }

    pub fn toggle_normalize_volume(&mut self) {
        self.is_normalize_volume = !self.is_normalize_volume;
        let _ = self.audio.set_normalization(self.is_normalize_volume);
    }

    pub fn set_sort(&mut self, field: SortField) {
        if self.sort_field == field {
            self.sort_direction = self.sort_direction.toggle();
        } else {
            self.sort_field = field;
            self.sort_direction = SortDirection::Asc;
        }
    }

    pub fn toggle_artist_sort_order(&mut self) {
        self.artist_sort_order = self.artist_sort_order.toggle();
    }

    pub fn select_tab(&mut self, tab: ActiveTab) {
        self.active_tab = tab;
        self.selected_artist = None;
    }

    pub fn select_folder_and_scan(&mut self) {
        if let Some(folder) = select_folder() {
            self.current_folder_path = Some(folder.clone());
            self.scan_folder(Some(folder));
        }
    }

    pub fn scan_folder(&mut self, folder: Option<String>) {
        self.loading = true;
        self.songs = scan_music_folder(folder);
        self.loading = false;
    }

    // ─── Pantalla completa / letras ─────────────────────────────────────────

    pub fn open_fullscreen(&mut self) {
        self.is_fullscreen = true;
    }

    pub fn close_fullscreen(&mut self) {
        self.is_fullscreen = false;
    }

    pub fn open_lyrics(&mut self) {
        self.show_lyrics = true;
        self.is_fullscreen = true;
    }

    pub fn toggle_lyrics_visibility(&mut self) {
        self.show_lyrics = !self.show_lyrics;
    }

    /// Carga las letras de la canción actual si todavía no se cargaron para
    /// esa ruta (equivalente al `useEffect` de `useLyrics.ts`). Lectura de
    /// disco síncrona — rápida (un .lrc chico o un tag embebido), no amerita
    /// un hilo aparte.
    pub fn ensure_lyrics_for_current_song(&mut self) {
        let Some(path) = self.current_song().map(|s| s.path.clone()) else {
            self.lyrics = None;
            self.lyrics_loaded_for = None;
            return;
        };
        if self.lyrics_loaded_for.as_deref() == Some(path.as_str()) {
            return;
        }
        self.lyrics = lyrics::get_lyrics_for_song(&path);
        self.lyrics_loaded_for = Some(path);
    }

    /// Índice de la línea sincronizada activa según `current_time` — última
    /// línea cuyo `time_ms` ya pasó. `None` si la letra no está sincronizada.
    pub fn active_lyric_line_index(&self) -> Option<usize> {
        let Some(Lyrics::Synced(lines)) = &self.lyrics else { return None };
        let time_ms = (self.current_time * 1000.0) as u64;
        let mut active = None;
        for (i, line) in lines.iter().enumerate() {
            if line.time_ms <= time_ms {
                active = Some(i);
            } else {
                break;
            }
        }
        active
    }

    /// Se llama una vez por frame desde `App::update()`. Reemplaza el
    /// `setInterval` de 500ms de la versión React con un gate por tiempo.
    pub fn tick(&mut self) {
        if !self.is_playing || self.is_dragging_seek {
            return;
        }
        if self.last_position_poll.elapsed().as_millis() < 500 {
            return;
        }
        self.last_position_poll = Instant::now();

        let pos = self.audio.position_secs() as f64;
        self.current_time = pos;

        if let Some(song) = self.current_song() {
            if song.duration_secs > 0 && pos >= song.duration_secs as f64 {
                self.handle_auto_next_song();
            }
        }
    }
}
