/// MPRIS2 Integration via souvlaki 0.8
/// Runs in a dedicated thread so souvlaki's Linux DBus connection stays
/// on the thread it was created on. The rest of the app communicates
/// with this thread through a cheap `SyncSender<MprisMsg>`.
use crate::events::AppEvent;
use souvlaki::{
    MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback,
    MediaPosition, PlatformConfig,
};
use std::sync::mpsc::{self, Sender};
use std::time::Duration;

#[derive(Debug)]
pub enum MprisMsg {
    /// Update the now-playing metadata + playback state
    Update {
        title: String,
        artist: String,
        album: String,
        cover_path: Option<String>,
        duration_secs: u64,
        position_secs: f64,
        is_playing: bool,
    },
}

/// Spawn the MPRIS2 background thread and return a sender for updates.
/// `player` is the shared GStreamer player.
/// `app_tx` reemplaza el `AppHandle::emit` de Tauri: notifica a la app
/// principal (vía `AppEvent`) cuando el usuario usa los controles del sistema.
pub fn spawn_mpris_thread(
    player: std::sync::Arc<std::sync::Mutex<gstreamer_player::Player>>,
    app_tx: Sender<AppEvent>,
) -> mpsc::SyncSender<MprisMsg> {
    // bounded=4: latest state wins; no need to queue many msgs
    let (tx, rx) = mpsc::sync_channel::<MprisMsg>(4);

    std::thread::Builder::new()
        .name("mpris2".into())
        .spawn(move || {
            let config = PlatformConfig {
                dbus_name: "simple_player",
                display_name: "Simple Player",
                hwnd: None,
            };

            let mut controls = match MediaControls::new(config) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("[MPRIS] Failed to initialize MPRIS2: {:?}", e);
                    return;
                }
            };

            // ── Event handler: user presses controls in taskbar / notifications ──
            let player_ev = std::sync::Arc::clone(&player);
            let app_ev = app_tx.clone();

            // souvlaki 0.8 uses .attach() (not set_event_handler)
            let _ = controls.attach(move |event: MediaControlEvent| match event {
                MediaControlEvent::Play => {
                    if let Ok(p) = player_ev.lock() {
                        p.play();
                    }
                    let _ = app_ev.send(AppEvent::MediaPlaying(true));
                }
                MediaControlEvent::Pause => {
                    if let Ok(p) = player_ev.lock() {
                        p.pause();
                    }
                    let _ = app_ev.send(AppEvent::MediaPlaying(false));
                }
                MediaControlEvent::Toggle => {
                    // Let the frontend figure out the correct toggle
                    let _ = app_ev.send(AppEvent::MediaPlayPause);
                }
                MediaControlEvent::Next => {
                    let _ = app_ev.send(AppEvent::MediaNext);
                }
                MediaControlEvent::Previous => {
                    let _ = app_ev.send(AppEvent::MediaPrev);
                }
                _ => {}
            });

            // ── Update loop ──────────────────────────────────────────────────────
            loop {
                match rx.recv() {
                    Ok(MprisMsg::Update {
                        title,
                        artist,
                        album,
                        cover_path,
                        duration_secs,
                        position_secs,
                        is_playing,
                    }) => {
                        // Cover art: MPRIS2 expects a file:// URI
                        let cover_uri = cover_path
                            .as_ref()
                            .map(|p| format!("file://{}", p));

                        let _ = controls.set_metadata(MediaMetadata {
                            title: Some(title.as_str()),
                            artist: Some(artist.as_str()),
                            album: Some(album.as_str()),
                            cover_url: cover_uri.as_deref(),
                            duration: Some(Duration::from_secs(duration_secs)),
                        });

                        let progress = Some(MediaPosition(Duration::from_secs_f64(
                            position_secs,
                        )));
                        if is_playing {
                            let _ = controls.set_playback(MediaPlayback::Playing { progress });
                        } else {
                            let _ = controls.set_playback(MediaPlayback::Paused { progress });
                        }
                    }
                    Err(_) => break, // Channel closed, exit thread
                }
            }
        })
        .expect("Failed to spawn MPRIS2 thread");

    tx
}
