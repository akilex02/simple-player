pub mod artists_grid;
pub mod header;
pub mod sidebar;
pub mod song_table;

pub fn format_time(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}
