pub mod fullscreen;
pub mod integration_prompt;
pub mod gallery;
pub mod lyrics_view;
pub mod progress;
pub mod screens;
pub mod shell;
mod size;
pub mod backdrop;
pub mod crossfade;
pub mod textures;
pub mod transport;
pub mod visualizers;
pub mod widgets;
pub mod volume;


pub use size::Size;

pub fn format_time(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    format!("{mins}:{secs:02}")
}
