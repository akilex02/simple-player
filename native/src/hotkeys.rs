use crate::events::AppEvent;
use global_hotkey::hotkey::{Code, HotKey};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::collections::HashSet;
use std::sync::mpsc::Sender;

/// Atajos globales (funcionan con la ventana minimizada): F6/F7/F8 y las
/// teclas multimedia estándar. Reemplaza a `tauri-plugin-global-shortcut`
/// con `global-hotkey`, la librería base que ese plugin envuelve.
pub struct Hotkeys {
    _manager: GlobalHotKeyManager,
    playpause_ids: HashSet<u32>,
    prev_ids: HashSet<u32>,
    next_ids: HashSet<u32>,
}

impl Hotkeys {
    pub fn register() -> Result<Self, Box<dyn std::error::Error>> {
        let manager = GlobalHotKeyManager::new()?;

        let playpause = [HotKey::new(None, Code::F7), HotKey::new(None, Code::MediaPlayPause)];
        let prev = [HotKey::new(None, Code::F6), HotKey::new(None, Code::MediaTrackPrevious)];
        let next = [HotKey::new(None, Code::F8), HotKey::new(None, Code::MediaTrackNext)];

        for hk in playpause.iter().chain(prev.iter()).chain(next.iter()) {
            let _ = manager.register(*hk);
        }

        Ok(Self {
            _manager: manager,
            playpause_ids: playpause.iter().map(|h| h.id()).collect(),
            prev_ids: prev.iter().map(|h| h.id()).collect(),
            next_ids: next.iter().map(|h| h.id()).collect(),
        })
    }

    /// Llamar una vez por frame: drena los eventos pendientes y los traduce a `AppEvent`.
    pub fn poll(&self, tx: &Sender<AppEvent>) {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state != HotKeyState::Pressed {
                continue;
            }
            if self.playpause_ids.contains(&event.id) {
                let _ = tx.send(AppEvent::MediaPlayPause);
            } else if self.prev_ids.contains(&event.id) {
                let _ = tx.send(AppEvent::MediaPrev);
            } else if self.next_ids.contains(&event.id) {
                let _ = tx.send(AppEvent::MediaNext);
            }
        }
    }
}
