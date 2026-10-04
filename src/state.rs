use crate::audio::clock::SeekGuard;
use crate::audio::AudioPlayer;
use crate::desktop_integration::{self, Status};
use crate::library::{load_cached_library, scan_and_cache, select_folder, Song};
use crate::library_view::{self, AlbumGroup, LibraryView};
use crate::lyrics::{self, Lyrics};
use crate::mpris::MprisMsg;
use crate::persistence::{
    load_playback_position, load_playback_state, position_moved, restore_position, save_playback_position, save_playback_state,
    PlaybackPosition, PlaybackState,
};
use crate::settings::{self, Settings};
use crate::stats::model::{SongSnapshot, StatsRange};
use crate::stats::recorder::StatsRecorder;
use crate::stats::service::StatsHandle;
use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::SyncSender;
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveTab {
    All,
    Albums,
    Artists,
    Stats,
    Settings,
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

/// Un álbum abierto filtra también por su artista (el nombre de álbum puede repetirse).
fn artist_filter<'a>(artist: &'a Option<String>, album: &'a Option<(String, String)>) -> Option<&'a str> {
    artist.as_deref().or(album.as_ref().map(|(_, a)| a.as_str()))
}

fn shuffle_vec(items: &mut Vec<Song>) {
    items.shuffle(&mut rand::thread_rng());
}

pub struct AppState {
    pub audio: AudioPlayer,
    mpris_tx: SyncSender<MprisMsg>,

    pub songs: Vec<Song>,
    songs_version: u64,
    view: LibraryView,
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
    pub settings: Settings,
    settings_path: Option<PathBuf>,
    /// `true` tras un restablecimiento de fábrica: al cerrar no se guarda el tamaño de la ventana.
    pub window_reset: bool,
    /// Ruta del AppImage en ejecución (solo en ejecuciones normales); `None` si no corre como AppImage.
    appimage: Option<PathBuf>,
    /// ¿Está registrado en el escritorio? Se lee del disco al arrancar y tras registrar o quitar (no cada frame).
    integration_status: Option<Status>,
    integration_prompt_hidden: bool,
    /// Segundo en el que debe continuar la canción restaurada; se aplica cuando el reproductor ya cargó el archivo.
    pending_resume: Option<f64>,
    /// La canción restaurada al arrancar todavía no se cargó en el reproductor (no ha sonado).
    restored_unloaded: bool,
    last_saved_position: Option<f64>,
    last_position_write: Instant,
    /// Solo desarrollo (`--scroll N`): desplazamiento inicial de la lista de canciones.
    pub dev_scroll: Option<f32>,

    pub active_tab: ActiveTab,
    pub selected_artist: Option<String>,
    /// (álbum, artista) del álbum abierto en la pestaña Álbumes.
    pub selected_album: Option<(String, String)>,
    pub show_queue: bool,
    /// Pide que la barra superior enfoque el buscador en el próximo frame.
    pub focus_search: bool,
    /// Registro de estadísticas de escucha (deshabilitado hasta que `main` lo inicie).
    pub stats: StatsRecorder,
    pub stats_range: StatsRange,
    /// Última (rango, versión de eventos) para la que la pantalla pidió un resumen.
    pub stats_requested: Option<(StatsRange, u64)>,
    seek_guard: SeekGuard,

    pub sort_field: SortField,
    pub sort_direction: SortDirection,
    pub artist_sort_order: SortDirection,

    pub is_shuffle: bool,
    pub repeat_mode: RepeatMode,

    pub is_fullscreen: bool,
    pub show_lyrics: bool,
    pub lyrics: Option<Lyrics>,
    lyrics_loaded_for: Option<String>,
}

impl AppState {
    pub fn new(audio: AudioPlayer, mpris_tx: SyncSender<MprisMsg>) -> Self {
        Self {
            audio,
            mpris_tx,
            songs: Vec::new(),
            songs_version: 0,
            view: LibraryView::default(),
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
            settings: Settings::default(),
            settings_path: None,
            window_reset: false,
            appimage: None,
            integration_status: None,
            integration_prompt_hidden: false,
            pending_resume: None,
            restored_unloaded: false,
            last_saved_position: None,
            last_position_write: Instant::now(),
            dev_scroll: None,
            active_tab: ActiveTab::All,
            selected_artist: None,
            selected_album: None,
            show_queue: false,
            focus_search: false,
            stats: StatsRecorder::new(StatsHandle::disabled("Las estadísticas todavía no se iniciaron")),
            stats_range: StatsRange::Week,
            stats_requested: None,
            seek_guard: SeekGuard::new(),
            sort_field: SortField::Title,
            sort_direction: SortDirection::Asc,
            artist_sort_order: SortDirection::Asc,
            is_shuffle: false,
            repeat_mode: RepeatMode::Off,
            is_fullscreen: false,
            show_lyrics: false,
            lyrics: None,
            lyrics_loaded_for: None,
        }
    }

    /// Carga la biblioteca (desde caché) y restaura el estado persistido —
    /// equivalente al `useEffect` de inicio en App.tsx.
    pub fn init(&mut self) {
        self.loading = true;
        self.set_songs(load_cached_library().unwrap_or_else(|| scan_and_cache(&self.settings.music_folders)));
        self.loading = false;

        let Some(saved) = load_playback_state() else { return };

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
                // Queda en pausa en el punto donde se cerró; continúa cuando el usuario dé play.
                self.restored_unloaded = true;
                self.apply_saved_position(load_playback_position());
            }
        }

        self.volume = saved.volume;
        let _ = self.audio.set_volume(self.volume);
    }

    fn persist(&self) {
        let state = PlaybackState {
            folder_path: None,
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

    /// Lista tal como la ve la tabla (para armar colas al reproducir). Para
    /// dibujar cada frame usar `visible_song_indices`, que está cacheada.
    pub fn sorted_songs(&self) -> Vec<&Song> {
        library_view::filter_sort(
            &self.songs,
            &self.search_query,
            artist_filter(&self.selected_artist, &self.selected_album),
            self.selected_album.as_ref().map(|(album, _)| album.as_str()),
            self.sort_field,
            self.sort_direction,
        )
        .into_iter()
        .map(|i| &self.songs[i])
        .collect()
    }

    pub fn visible_song_indices(&mut self) -> Arc<Vec<usize>> {
        self.view.songs(
            &self.songs,
            self.songs_version,
            &self.search_query,
            artist_filter(&self.selected_artist, &self.selected_album),
            self.selected_album.as_ref().map(|(album, _)| album.as_str()),
            self.sort_field,
            self.sort_direction,
        )
    }

    pub fn album_groups(&mut self) -> Arc<Vec<AlbumGroup>> {
        self.view.albums(&self.songs, self.songs_version, &self.search_query, self.artist_sort_order)
    }

    pub fn artist_groups(&mut self) -> Arc<Vec<ArtistGroup>> {
        self.view.groups(&self.songs, self.songs_version, &self.search_query, self.artist_sort_order)
    }

    fn set_songs(&mut self, songs: Vec<Song>) {
        self.songs = songs;
        self.songs_version += 1;
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
        self.stats.song_started(SongSnapshot::from(&song), true);

        self.active_queue = queue;
        self.current_song_index = Some(valid_index as usize);
        self.current_time = 0.0;
        self.pending_resume = None;
        self.restored_unloaded = false;

        let _ = self.audio.play(&song.path);
        self.is_playing = true;

        self.persist();
        self.save_position();
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

    /// Play o pausa explícitos (MPRIS, teclas multimedia): pasan por la misma lógica que el botón, de modo que
    /// una canción restaurada al arrancar se carga antes de sonar y la posición guardada se aplica.
    pub fn set_playing(&mut self, play: bool) {
        if play != self.is_playing {
            self.toggle_play_pause();
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
            self.save_position();
        } else {
            if let Some(song) = self.current_song().cloned() {
                let pos = self.audio.position_secs();
                if pos == 0 {
                    let _ = self.audio.play(&song.path);
                    // Ya se cargó: si hay una posición pendiente, `tick` la aplica en cuanto el reproductor responda.
                    self.restored_unloaded = false;
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
        if self.restored_unloaded {
            // El reproductor aún no cargó el archivo: el punto elegido se aplica al dar play.
            self.pending_resume = Some(new_secs);
            return;
        }
        self.seek_guard.on_seek(new_secs, Instant::now());
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
        self.selected_album = None;
    }

    /// Ajustes cargados al arrancar y dónde guardarlos (`None` en corridas de desarrollo: no se escribe nada).
    pub fn with_settings(mut self, settings: Settings, path: Option<PathBuf>) -> Self {
        self.settings = settings;
        self.settings_path = path;
        self
    }

    /// Indica que la app corre como AppImage (`APPIMAGE`) y lee si ya está registrada en el escritorio.
    pub fn with_appimage(mut self, appimage: Option<PathBuf>) -> Self {
        self.appimage = appimage;
        self.refresh_integration_status();
        self
    }

    fn integration_paths() -> desktop_integration::Paths {
        desktop_integration::paths(std::env::var("XDG_DATA_HOME").ok().as_deref(), std::env::var("HOME").ok().as_deref())
    }

    fn refresh_integration_status(&mut self) {
        self.integration_status = self.appimage.as_deref().map(|app| desktop_integration::status(&Self::integration_paths(), app));
    }

    pub fn is_appimage(&self) -> bool {
        self.appimage.is_some()
    }

    pub fn integration_status(&self) -> Option<Status> {
        self.integration_status
    }

    /// ¿Se muestra el aviso de «Registrar en el menú de aplicaciones»?
    pub fn should_prompt_integration(&self) -> bool {
        match self.integration_status {
            Some(status) => desktop_integration::should_prompt(
                self.is_appimage(),
                status,
                self.settings.integration_prompt_dismissed,
                self.integration_prompt_hidden,
            ),
            None => false,
        }
    }

    /// «Ahora no» oculta el aviso en esta sesión; «No volver a preguntar» lo guarda en los ajustes.
    pub fn dismiss_integration_prompt(&mut self, forever: bool) {
        self.integration_prompt_hidden = true;
        if forever {
            self.update_settings(|s| s.integration_prompt_dismissed = true);
        }
    }

    /// Escribe el `.desktop` y el ícono de usuario. Solo en ejecuciones normales y como AppImage.
    pub fn register_desktop(&mut self) -> Result<(), String> {
        let Some(app) = self.appimage.clone() else { return Err("La app no corre como AppImage".to_string()) };
        if self.settings_path.is_none() {
            return Err("No disponible en ejecuciones de desarrollo".to_string());
        }
        desktop_integration::install(&Self::integration_paths(), &app, include_bytes!("../assets/icons/icon.png")).map_err(|e| e.to_string())?;
        self.refresh_integration_status();
        Ok(())
    }

    pub fn unregister_desktop(&mut self) -> Result<(), String> {
        if self.settings_path.is_none() {
            return Err("No disponible en ejecuciones de desarrollo".to_string());
        }
        desktop_integration::remove(&Self::integration_paths()).map_err(|e| e.to_string())?;
        self.refresh_integration_status();
        Ok(())
    }

    fn save_settings(&self) {
        if let Some(path) = &self.settings_path {
            if let Err(e) = settings::save(path, &self.settings) {
                eprintln!("[aviso] No se pudieron guardar los ajustes: {e}");
            }
        }
    }

    /// Aplica un cambio a los ajustes y lo guarda.
    pub fn update_settings(&mut self, change: impl FnOnce(&mut Settings)) {
        change(&mut self.settings);
        self.save_settings();
    }

    pub fn add_folder_via_dialog(&mut self) {
        if let Some(folder) = select_folder() {
            self.add_music_folder(&folder);
        }
    }

    pub fn add_music_folder(&mut self, folder: &str) {
        if self.settings.add_folder(folder) {
            self.save_settings();
            self.rescan_library();
        }
    }

    pub fn remove_music_folder(&mut self, folder: &str) {
        if self.settings.remove_folder(folder) {
            self.save_settings();
            self.rescan_library();
        }
    }

    /// El restablecimiento de fábrica toca archivos reales: solo existe en ejecuciones normales.
    pub fn can_factory_reset(&self) -> bool {
        self.settings_path.is_some()
    }

    /// Borra los datos de la app (ver `reset`), detiene la reproducción y vuelve a los valores por defecto
    /// en memoria. Conserva el historial de estadísticas. Devuelve cuántas cosas se borraron.
    pub fn factory_reset(&mut self) -> Result<usize, String> {
        let Some(settings_file) = self.settings_path.clone() else {
            return Err("No disponible en ejecuciones de desarrollo".to_string());
        };
        let window_file = crate::window_state::window_state_path(
            std::env::var("XDG_CONFIG_HOME").ok().as_deref(),
            std::env::var("HOME").ok().as_deref(),
        );
        let (files, dirs) = crate::reset::factory_paths(&settings_file, &window_file);
        let report = crate::reset::remove_all(&files, &dirs);

        self.window_reset = true;
        // Cierra la sesión de escucha en curso (si pasó el umbral se registra) antes de vaciar la cola.
        self.stats.finish();
        let _ = self.audio.pause();
        self.is_playing = false;
        self.current_song_index = None;
        self.active_queue.clear();
        self.settings = Settings::default();
        self.set_volume(0.8);
        self.is_shuffle = false;
        self.repeat_mode = RepeatMode::Off;
        self.set_songs(Vec::new());

        if report.errors.is_empty() {
            Ok(report.removed)
        } else {
            Err(report.errors.join("; "))
        }
    }

    /// Aplica la posición guardada si es de la canción actual: queda en ese punto (en pausa) y pendiente
    /// de cargarse al dar play. En los últimos segundos de la canción se descarta.
    pub fn apply_saved_position(&mut self, saved: Option<PlaybackPosition>) {
        let Some(saved) = saved else { return };
        let Some(song) = self.current_song() else { return };
        if song.path != saved.path {
            return;
        }
        let position = restore_position(saved.secs, song.duration_secs);
        if position > 0.0 {
            self.current_time = position;
            self.pending_resume = Some(position);
        }
    }

    /// Canción actual y segundo en que va, para guardarlos.
    pub fn position_snapshot(&self) -> Option<PlaybackPosition> {
        self.current_song().map(|song| PlaybackPosition { path: song.path.clone(), secs: self.current_time })
    }

    /// Las corridas de desarrollo no tienen ruta de ajustes y nunca tocan el estado real.
    pub fn position_saving_enabled(&self) -> bool {
        self.settings_path.is_some()
    }

    /// Guarda la posición ahora (al pausar, al cambiar de canción y al cerrar).
    pub fn save_position(&mut self) {
        if !self.position_saving_enabled() {
            return;
        }
        let Some(position) = self.position_snapshot() else { return };
        match save_playback_position(&position) {
            Ok(()) => self.last_saved_position = Some(position.secs),
            Err(e) => eprintln!("[aviso] No se pudo guardar la posición: {e}"),
        }
        self.last_position_write = Instant::now();
    }

    /// Guardado de respaldo mientras suena: cada 30 s y solo si la posición se movió, para no escribir de más.
    fn save_position_if_due(&mut self) {
        const INTERVAL: std::time::Duration = std::time::Duration::from_secs(30);
        if self.last_position_write.elapsed() < INTERVAL || !self.position_saving_enabled() {
            return;
        }
        if position_moved(self.last_saved_position, self.current_time) {
            self.save_position();
        } else {
            self.last_position_write = Instant::now();
        }
    }

    pub fn rescan_library(&mut self) {
        self.loading = true;
        self.set_songs(scan_and_cache(&self.settings.music_folders));
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
        lyrics::active_line(lines, self.current_time)
    }

    /// Se llama una vez por frame desde `App::update()`. A diferencia del
    /// `setInterval` de 500ms de la versión React, aquí no hace falta ningún
    /// gate de tiempo: la app ya se redibuja cada frame (60fps+) vía
    /// `ctx.request_repaint()`, y `position_secs()` es solo un lock de mutex
    /// + una consulta a GStreamer — barato de llamar cada frame. Esto hace
    /// que el resaltado de la línea de letra activa sea fluido en vez de
    /// actualizarse a saltos de 500ms.
    /// Alimenta las estadísticas cada frame. Si suena una canción sin sesión
    /// (la cola restaurada al arrancar, MPRIS, teclas multimedia) la abre aquí.
    pub fn observe_stats_at(&mut self, now: Instant, epoch_ms: i64) {
        if self.is_playing && !self.stats.has_session() {
            if let Some(song) = self.current_song().map(SongSnapshot::from) {
                self.stats.song_started_at(song, true, now, epoch_ms);
            }
        }
        self.stats.observe_at(self.is_playing, now, epoch_ms);
    }

    pub fn observe_stats(&mut self) {
        self.observe_stats_at(Instant::now(), crate::stats::recorder::epoch_ms_now());
    }

    pub fn tick(&mut self) {
        if !self.is_playing || self.is_dragging_seek {
            return;
        }

        if let Some(secs) = self.pending_resume {
            if !self.restored_unloaded && self.audio.try_position_secs().is_some() {
                self.pending_resume = None;
                self.seek_commit(secs);
            }
        }
        self.save_position_if_due();

        let reported = self.audio.try_position_secs().unwrap_or(self.current_time);
        let pos = self.seek_guard.resolve(reported, Instant::now());
        self.current_time = pos;

        if let Some(song) = self.current_song() {
            if song.duration_secs > 0 && pos >= song.duration_secs as f64 {
                self.handle_auto_next_song();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::location::StatsLocation;
    use std::sync::Mutex;
    use std::time::Duration;

    const E0: i64 = 1_700_000_000_000;

    fn state() -> AppState {
        gstreamer::init().unwrap();
        let player = gstreamer_player::Player::new(
            None::<gstreamer_player::PlayerVideoRenderer>,
            None::<gstreamer_player::PlayerSignalDispatcher>,
        );
        let audio = AudioPlayer { inner: Arc::new(Mutex::new(player)) };
        let (tx, _rx) = std::sync::mpsc::sync_channel(4);
        let mut s = AppState::new(audio, tx);
        s.stats = StatsRecorder::new(StatsHandle::spawn(StatsLocation::Memory, Arc::new(|| {})));
        s.active_queue = vec![Song {
            path: "/m/restaurada.mp3".into(),
            title: "Restaurada".into(),
            artist: "A".into(),
            album: "B".into(),
            duration_secs: 200,
            cover_art: None,
        }];
        s.current_song_index = Some(0);
        s
    }

    fn plays(s: &AppState) -> u32 {
        let h = s.stats.handle().clone();
        h.request_summary(StatsRange::All);
        h.flush(Duration::from_secs(2));
        h.latest_summary().map(|(_, summary)| summary.totals.plays).unwrap_or(0)
    }

    #[test]
    fn la_cancion_restaurada_al_arrancar_se_registra_al_pulsar_play() {
        // `init` restaura la cola sin sonar; play reanuda sin pasar por `play_index`.
        let mut s = state();
        s.is_playing = true;
        let t0 = Instant::now();
        s.observe_stats_at(t0, E0);
        s.observe_stats_at(t0 + Duration::from_secs(8), E0 + 8_000);
        s.stats.finish_at(t0 + Duration::from_secs(8), E0 + 8_000);
        assert_eq!(plays(&s), 1);
    }

    #[test]
    fn en_pausa_no_se_abre_ninguna_sesion() {
        let mut s = state();
        s.is_playing = false;
        let t0 = Instant::now();
        s.observe_stats_at(t0, E0);
        s.observe_stats_at(t0 + Duration::from_secs(30), E0 + 30_000);
        s.stats.finish_at(t0 + Duration::from_secs(30), E0 + 30_000);
        assert_eq!(plays(&s), 0);
    }

    fn saved(path: &str, secs: f64) -> Option<PlaybackPosition> {
        Some(PlaybackPosition { path: path.into(), secs })
    }

    #[test]
    fn al_restaurar_la_posicion_la_cancion_queda_en_ese_punto_y_pendiente_de_cargar() {
        let mut s = state();
        s.apply_saved_position(saved("/m/restaurada.mp3", 61.5));
        assert_eq!(s.current_time, 61.5, "la barra y el tiempo muestran lo guardado");
        assert_eq!(s.pending_resume, Some(61.5));
        assert!(!s.is_playing, "queda en pausa");
    }

    #[test]
    fn una_posicion_de_otra_cancion_o_sin_posicion_no_se_aplica() {
        let mut s = state();
        s.apply_saved_position(saved("/m/otra.mp3", 61.5));
        assert_eq!((s.current_time, s.pending_resume), (0.0, None));
        s.apply_saved_position(None);
        assert_eq!((s.current_time, s.pending_resume), (0.0, None));
    }

    #[test]
    fn una_posicion_en_los_ultimos_segundos_se_descarta() {
        let mut s = state();
        s.apply_saved_position(saved("/m/restaurada.mp3", 199.0));
        assert_eq!((s.current_time, s.pending_resume), (0.0, None));
    }

    #[test]
    fn mover_la_barra_antes_de_dar_play_reemplaza_la_posicion_guardada() {
        let mut s = state();
        s.apply_saved_position(saved("/m/restaurada.mp3", 61.5));
        s.restored_unloaded = true; // como tras `init`: la cola se restauró sin sonar
        s.seek_commit(120.0);
        assert_eq!(s.current_time, 120.0);
        assert_eq!(s.pending_resume, Some(120.0), "el reproductor aún no cargó el archivo");
    }

    #[test]
    fn la_posicion_a_guardar_es_la_de_la_cancion_actual() {
        let mut s = state();
        s.current_time = 33.0;
        assert_eq!(s.position_snapshot(), saved("/m/restaurada.mp3", 33.0));
        s.current_song_index = None;
        assert_eq!(s.position_snapshot(), None);
    }

    #[test]
    fn en_corridas_de_desarrollo_la_posicion_nunca_se_escribe() {
        let mut s = state(); // sin ruta de ajustes = corrida de desarrollo
        s.current_time = 33.0;
        assert!(!s.position_saving_enabled());
        s.save_position();
        assert_eq!(s.last_saved_position, None, "no se registró ningún guardado");
    }

    #[test]
    fn play_por_mpris_con_la_cancion_restaurada_la_carga_y_conserva_la_posicion_pendiente() {
        let mut s = state();
        s.apply_saved_position(saved("/m/restaurada.mp3", 61.5));
        s.restored_unloaded = true;
        s.set_playing(true);
        assert!(s.is_playing);
        assert!(!s.restored_unloaded, "ya se cargó en el reproductor");
        assert_eq!(s.pending_resume, Some(61.5), "la posición se aplica cuando el reproductor responda");
    }

    #[test]
    fn play_cuando_ya_suena_o_pause_cuando_ya_esta_en_pausa_no_cambian_nada() {
        let mut s = state();
        s.is_playing = true;
        s.set_playing(true);
        assert!(s.is_playing, "no debe pausar");
        s.is_playing = false;
        s.set_playing(false);
        assert!(!s.is_playing, "no debe reanudar");
    }

    #[test]
    fn pause_por_mpris_pausa_si_suena() {
        let mut s = state();
        s.is_playing = true;
        s.set_playing(false);
        assert!(!s.is_playing);
    }

    fn as_appimage(s: &mut AppState) {
        s.appimage = Some(PathBuf::from("/opt/Simple.AppImage"));
        s.integration_status = Some(crate::desktop_integration::Status::NotInstalled);
    }

    #[test]
    fn el_aviso_de_registro_solo_sale_como_appimage_sin_registrar() {
        let mut s = state();
        assert!(!s.should_prompt_integration(), "sin AppImage no se ofrece");
        as_appimage(&mut s);
        assert!(s.should_prompt_integration());
        s.integration_status = Some(crate::desktop_integration::Status::Installed);
        assert!(!s.should_prompt_integration(), "ya registrado");
    }

    #[test]
    fn ahora_no_oculta_el_aviso_solo_en_esta_sesion_y_no_volver_a_preguntar_se_guarda() {
        let mut s = state();
        as_appimage(&mut s);
        s.dismiss_integration_prompt(false);
        assert!(!s.should_prompt_integration());
        assert!(!s.settings.integration_prompt_dismissed, "«ahora no» no se guarda");

        let mut s = state();
        as_appimage(&mut s);
        s.dismiss_integration_prompt(true);
        assert!(!s.should_prompt_integration());
        assert!(s.settings.integration_prompt_dismissed, "«no volver a preguntar» se guarda");
    }

    #[test]
    fn en_corridas_de_desarrollo_registrar_no_escribe_nada() {
        let mut s = state(); // sin ruta de ajustes = corrida de desarrollo
        as_appimage(&mut s);
        let result = s.register_desktop();
        assert!(result.is_err(), "no disponible en desarrollo");
        assert_eq!(s.integration_status, Some(crate::desktop_integration::Status::NotInstalled));
    }
}
