use crate::events::{AppEvent, EventSender};
use global_hotkey::hotkey::{Code, HotKey};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use std::collections::HashSet;

/// Atajos globales (funcionan con la ventana minimizada): F6/F7/F8 y las
/// teclas multimedia estándar. Reemplaza a `tauri-plugin-global-shortcut`
/// con `global-hotkey`, la librería base que ese plugin envuelve.
///
/// Un hilo bloquea esperando eventos y los reenvía con `EventSender`, que
/// despierta a la UI: así la app no necesita repintar en reposo para
/// enterarse de una tecla.
pub struct Hotkeys {
    _manager: GlobalHotKeyManager,
}

impl Hotkeys {
    pub fn register(sender: EventSender) -> Result<Self, Box<dyn std::error::Error>> {
        let manager = GlobalHotKeyManager::new()?;

        let playpause = [HotKey::new(None, Code::F7), HotKey::new(None, Code::MediaPlayPause)];
        let prev = [HotKey::new(None, Code::F6), HotKey::new(None, Code::MediaTrackPrevious)];
        let next = [HotKey::new(None, Code::F8), HotKey::new(None, Code::MediaTrackNext)];

        for hk in playpause.iter().chain(prev.iter()).chain(next.iter()) {
            let _ = manager.register(*hk);
        }

        let ids = |keys: &[HotKey]| -> HashSet<u32> { keys.iter().map(|h| h.id()).collect() };
        let (playpause_ids, prev_ids, next_ids) = (ids(&playpause), ids(&prev), ids(&next));

        std::thread::spawn(move || {
            while let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                if event.state != HotKeyState::Pressed {
                    continue;
                }
                let app_event = if playpause_ids.contains(&event.id) {
                    AppEvent::MediaPlayPause
                } else if prev_ids.contains(&event.id) {
                    AppEvent::MediaPrev
                } else if next_ids.contains(&event.id) {
                    AppEvent::MediaNext
                } else {
                    continue;
                };
                let _ = sender.send(app_event);
            }
        });

        Ok(Self { _manager: manager })
    }
}
