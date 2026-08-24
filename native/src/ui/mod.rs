pub mod artists_grid;
pub mod fullscreen;
pub mod header;
pub mod lyrics_panel;
pub mod player_bar;
pub mod progress;
pub mod sidebar;
mod size;
pub mod song_table;
pub mod transport;
pub mod visualizers;
pub mod volume;

pub use size::Size;

pub fn format_time(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}
