use gstreamer as gst;
use gstreamer::prelude::*;
use gstreamer_player as gst_player;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::probe::Probe;
use lofty::tag::Accessor;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use walkdir::WalkDir;

mod mpris;
use mpris::{spawn_mpris_thread, MprisMsg};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Song {
    pub path: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: u64,
    pub cover_art: Option<String>,
}

/// Persisted UI state (folder, queue, current song) saved to disk on each song change
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PlaybackState {
    pub folder_path: Option<String>,
    pub queue_paths: Vec<String>,
    pub current_index: Option<usize>,
    pub volume: f64,
    pub is_shuffle: bool,
    pub repeat_mode: String,
}

fn get_playback_state_path() -> PathBuf {
    get_cache_dir().join("playback_state.json")
}

#[tauri::command]
fn save_playback_state(state: PlaybackState) -> Result<(), String> {
    let json = serde_json::to_string(&state).map_err(|e| e.to_string())?;
    fs::write(get_playback_state_path(), json).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_playback_state() -> Option<PlaybackState> {
    let path = get_playback_state_path();
    if !path.exists() {
        return None;
    }
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
}

/// GStreamer Player wrapped in Arc<Mutex> for thread-safe Tauri state
pub struct GstState {
    pub player: Arc<Mutex<gst_player::Player>>,
}

// SAFETY: gst_player::Player uses GLib reference counting and is safe to share across threads
unsafe impl Send for GstState {}
unsafe impl Sync for GstState {}

/// MPRIS2 state: a cheap sender to the dedicated MPRIS thread
pub struct MprisState {
    pub sender: std::sync::mpsc::SyncSender<MprisMsg>,
}
unsafe impl Send for MprisState {}
unsafe impl Sync for MprisState {}

fn get_cache_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let d = PathBuf::from(home).join(".cache").join("music-player");
    let _ = fs::create_dir_all(&d);
    d
}

fn get_library_cache_path() -> PathBuf {
    get_cache_dir().join("library_cache.json")
}

fn get_covers_dir() -> PathBuf {
    let d = get_cache_dir().join("covers");
    let _ = fs::create_dir_all(&d);
    d
}

fn path_hash(path: &str) -> String {
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    format!("{:x}", h.finish())
}

#[tauri::command]
fn select_folder() -> Option<String> {
    rfd::FileDialog::new()
        .pick_folder()
        .map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
fn scan_music_folder(folder_path: Option<String>) -> Vec<Song> {
    let cache_file = get_library_cache_path();

    // Fast startup: serve from disk cache on first run
    if folder_path.is_none() && cache_file.exists() {
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(mut songs) = serde_json::from_str::<Vec<Song>>(&content) {
                if !songs.is_empty() {
                    let covers_dir = get_covers_dir();
                    for song in &mut songs {
                        if let Some(ref mut c) = song.cover_art {
                            if c.starts_with("cover://") {
                                // Migrate old cover:// URIs to absolute paths
                                let filename = c.trim_start_matches("cover://");
                                *c = covers_dir.join(filename).to_string_lossy().to_string();
                            }
                        }
                    }
                    return songs;
                }
            }
        }
    }

    let target_dir = folder_path
        .filter(|p| !p.trim().is_empty())
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|h| format!("{}/Música", h))
                .unwrap_or_else(|_| "/home".to_string())
        });

    let supported_ext = ["mp3", "flac", "ogg", "wav", "m4a", "aac", "opus", "wma"];
    let covers_dir = get_covers_dir();

    let songs: Vec<Song> = WalkDir::new(&target_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path().is_file()
                && e.path()
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| supported_ext.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .map(|e| extract_song_info(e.path(), &covers_dir))
        .collect();

    // Persist lean JSON (paths only, no base64) for instant future startups
    if let Ok(json) = serde_json::to_string(&songs) {
        let _ = fs::write(&cache_file, json);
    }

    songs
}

fn extract_song_info(path: &Path, covers_dir: &Path) -> Song {
    let mut title = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut artist = "Artista Desconocido".to_string();
    let mut album = "Álbum Desconocido".to_string();
    let mut duration_secs = 0u64;
    let mut cover_art: Option<String> = None;

    if let Ok(tagged_file) = Probe::open(path).and_then(|p| p.read()) {
        duration_secs = tagged_file.properties().duration().as_secs();

        if let Some(tag) = tagged_file.primary_tag().or_else(|| tagged_file.first_tag()) {
            if let Some(t) = tag.title().filter(|t| !t.trim().is_empty()) {
                title = t.trim().to_string();
            }
            if let Some(a) = tag.artist().filter(|a| !a.trim().is_empty()) {
                artist = a.trim().to_string();
            }
            if let Some(al) = tag.album().filter(|al| !al.trim().is_empty()) {
                album = al.trim().to_string();
            }

            if let Some(picture) = tag.pictures().first() {
                let hash = path_hash(&path.to_string_lossy());
                let cover_path = covers_dir.join(format!("{}.jpg", hash));

                if !cover_path.exists() {
                    let _ = fs::write(&cover_path, picture.data());
                }
                cover_art = Some(cover_path.to_string_lossy().to_string());
            }
        }
    }

    Song {
        path: path.to_string_lossy().to_string(),
        title,
        artist,
        album,
        duration_secs,
        cover_art,
    }
}

/// Convert a filesystem path to a valid percent-encoded GStreamer URI (file:///...)
fn path_to_uri(path: &str) -> String {
    if path.starts_with("file://") || path.starts_with("http://") || path.starts_with("https://") {
        path.to_string()
    } else {
        gst::glib::filename_to_uri(path, None)
            .map(|u| u.to_string())
            .unwrap_or_else(|_| format!("file://{}", path))
    }
}

// ─── GStreamer Playback Commands ────────────────────────────────────────────

#[tauri::command]
fn play_song(state: tauri::State<'_, GstState>, song_path: String) -> Result<(), String> {
    let player = state.player.lock().map_err(|e| e.to_string())?;
    let uri = path_to_uri(&song_path);
    player.set_uri(Some(uri.as_str()));
    player.play();
    Ok(())
}

#[tauri::command]
fn pause_song(state: tauri::State<'_, GstState>) -> Result<(), String> {
    let player = state.player.lock().map_err(|e| e.to_string())?;
    player.pause();
    Ok(())
}

#[tauri::command]
fn resume_song(state: tauri::State<'_, GstState>) -> Result<(), String> {
    let player = state.player.lock().map_err(|e| e.to_string())?;
    player.play();
    Ok(())
}

/// Native GStreamer seek — hardware-accelerated 0ms response.
#[tauri::command]
fn seek_song(
    state: tauri::State<'_, GstState>,
    position_secs: f64,
) -> Result<(), String> {
    let player = state.player.lock().map_err(|e| e.to_string())?;
    let nanos = (position_secs.max(0.0) * 1_000_000_000.0) as u64;
    let position = gst::ClockTime::from_nseconds(nanos);
    player.seek(position);
    Ok(())
}

#[tauri::command]
fn set_volume(state: tauri::State<'_, GstState>, volume: f64) -> Result<(), String> {
    let player = state.player.lock().map_err(|e| e.to_string())?;
    player.set_volume(volume.clamp(0.0, 1.0));
    Ok(())
}

#[tauri::command]
fn set_audio_normalization(state: tauri::State<'_, GstState>, enabled: bool) -> Result<(), String> {
    let player = state.player.lock().map_err(|e| e.to_string())?;
    let pipeline = player.pipeline();

    if enabled {
        if gst::ElementFactory::find("rgvolume").is_some() && gst::ElementFactory::find("rglimiter").is_some() {
            if let Ok(filter) = gst::parse::bin_from_description("rgvolume fallback-gain=0.0 ! rglimiter", true) {
                pipeline.set_property("audio-filter", &filter);
                return Ok(());
            }
        }
    }
    let null_elem: Option<&gst::Element> = None;
    pipeline.set_property("audio-filter", null_elem);
    Ok(())
}

/// Returns the current playback position in seconds
#[tauri::command]
fn get_position(state: tauri::State<'_, GstState>) -> u64 {
    if let Ok(player) = state.player.lock() {
        if let Some(pos) = player.position() {
            return pos.seconds();
        }
    }
    0
}

/// Update MPRIS2 now-playing info (called from frontend on every song/state change)
#[tauri::command]
fn update_now_playing(
    mpris: tauri::State<'_, MprisState>,
    title: String,
    artist: String,
    album: String,
    cover_path: Option<String>,
    duration_secs: u64,
    position_secs: f64,
    is_playing: bool,
) -> Result<(), String> {
    mpris
        .sender
        .try_send(MprisMsg::Update {
            title,
            artist,
            album,
            cover_path,
            duration_secs,
            position_secs,
            is_playing,
        })
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Fix WebKitGTK Wayland protocol crash (Error 71 dispatching to Wayland display)
    if std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").is_err() {
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }
    if std::env::var("GDK_BACKEND").is_err() {
        std::env::set_var("GDK_BACKEND", "x11");
    }

    // Configure GStreamer plugin search paths
    let candidate_paths = [
        "/usr/lib/gstreamer-1.0",
        "/usr/lib64/gstreamer-1.0",
        "/usr/lib/x86_64-linux-gnu/gstreamer-1.0",
        "/usr/local/lib/gstreamer-1.0",
    ];
    let existing_paths: Vec<String> = candidate_paths
        .iter()
        .filter(|p| Path::new(p).exists())
        .map(|s| s.to_string())
        .collect();

    if !existing_paths.is_empty() {
        let current_path = std::env::var("GST_PLUGIN_PATH_1_0").unwrap_or_default();
        let new_path = if current_path.is_empty() {
            existing_paths.join(":")
        } else {
            format!("{}:{}", current_path, existing_paths.join(":"))
        };
        std::env::set_var("GST_PLUGIN_PATH_1_0", new_path);
    }

    // Initialize GStreamer
    gst::init().expect("No se pudo inicializar GStreamer");

    // Scan system plugin paths into GStreamer registry
    let registry = gst::Registry::get();
    for dir in &existing_paths {
        let _ = registry.scan_path(dir);
    }

    // Create GstPlayer
    let dispatcher = gst_player::PlayerGMainContextSignalDispatcher::new(None);
    let player = gst_player::Player::new(
        None::<gst_player::PlayerVideoRenderer>,
        Some(dispatcher),
    );

    // Dynamically test and configure explicit audio sink since autoaudiosink is missing
    player.set_volume(0.8);
    let pipeline = player.pipeline();
    
    for sink_name in ["pulsesink", "pipewiresink", "alsasink"] {
        if let Ok(sink) = gst::ElementFactory::make(sink_name).build() {
            // Test if the sink can actually open the audio device
            if sink.set_state(gst::State::Ready) == Ok(gst::StateChangeSuccess::Success) {
                let _ = sink.set_state(gst::State::Null);
                pipeline.set_property("audio-sink", &sink);
                println!("[GStreamer] Successfully verified and set audio-sink to '{}'", sink_name);
                break;
            } else {
                let _ = sink.set_state(gst::State::Null);
                println!("[GStreamer] Sink '{}' built but failed to reach READY state, trying next...", sink_name);
            }
        }
    }
    
    player.connect_error(|_, err| {
        eprintln!("[GStreamer Error] {}", err);
    });

    let gst_state = GstState {
        player: Arc::new(Mutex::new(player)),
    };

    let player_for_shortcuts = Arc::clone(&gst_state.player);

    tauri::Builder::default()
        .register_uri_scheme_protocol("cover", |_app, request| {
            let path = request.uri().path();
            let file_name = path.trim_start_matches('/');
            let covers_dir = get_covers_dir();
            let cover_path = if file_name.ends_with(".jpg") {
                covers_dir.join(file_name)
            } else {
                covers_dir.join(format!("{}.jpg", file_name))
            };

            if cover_path.exists() {
                if let Ok(bytes) = fs::read(&cover_path) {
                    return tauri::http::Response::builder()
                        .header("Access-Control-Allow-Origin", "*")
                        .header("Content-Type", "image/jpeg")
                        .body(bytes)
                        .unwrap_or_else(|_| tauri::http::Response::builder().status(500).body(Vec::new()).unwrap());
                }
            }

            tauri::http::Response::builder()
                .status(404)
                .header("Access-Control-Allow-Origin", "*")
                .body(Vec::new())
                .unwrap_or_else(|_| tauri::http::Response::builder().status(500).body(Vec::new()).unwrap())
        })
        .manage(gst_state)
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(move |app, shortcut, event| {
                    if event.state() != ShortcutState::Pressed {
                        return;
                    }

                    let shortcut_id = shortcut.id();

                    let is_playpause = shortcut_id == Shortcut::new(None, tauri_plugin_global_shortcut::Code::F7).id()
                        || shortcut_id == Shortcut::new(None, tauri_plugin_global_shortcut::Code::MediaPlayPause).id();

                    let is_prev = shortcut_id == Shortcut::new(None, tauri_plugin_global_shortcut::Code::F6).id()
                        || shortcut_id == Shortcut::new(None, tauri_plugin_global_shortcut::Code::MediaTrackPrevious).id();

                    let is_next = shortcut_id == Shortcut::new(None, tauri_plugin_global_shortcut::Code::F8).id()
                        || shortcut_id == Shortcut::new(None, tauri_plugin_global_shortcut::Code::MediaTrackNext).id();

                    if is_playpause {
                        if let Ok(p) = player_for_shortcuts.lock() {
                            let pipeline = p.pipeline();
                            let (_, state, _) = pipeline.state(gst::ClockTime::ZERO);
                            if state == gst::State::Playing {
                                p.pause();
                            } else {
                                p.play();
                            }
                        }
                        let _ = app.emit("media-playpause", ());
                    } else if is_prev {
                        let _ = app.emit("media-prev", ());
                    } else if is_next {
                        let _ = app.emit("media-next", ());
                    }
                })
                .build(),
        )
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Some(icon) = app.default_window_icon() {
                    let _ = window.set_icon(icon.clone());
                }
            }

            let handle = app.handle();
            let mpris_sender = spawn_mpris_thread(
                Arc::clone(&handle.state::<GstState>().player),
                handle.clone(),
            );
            handle.manage(MprisState { sender: mpris_sender });

            use tauri_plugin_global_shortcut::Code;
            let manager = app.global_shortcut();

            let shortcuts = [
                Shortcut::new(None, Code::F6),
                Shortcut::new(None, Code::F7),
                Shortcut::new(None, Code::F8),
                Shortcut::new(None, Code::MediaTrackPrevious),
                Shortcut::new(None, Code::MediaPlayPause),
                Shortcut::new(None, Code::MediaTrackNext),
            ];

            for sc in shortcuts {
                let _ = manager.register(sc);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            select_folder,
            scan_music_folder,
            play_song,
            pause_song,
            resume_song,
            seek_song,
            set_volume,
            set_audio_normalization,
            get_position,
            update_now_playing,
            save_playback_state,
            load_playback_state
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
