use glam::Vec4;
use styx::{components::Text, Font};

/// Returns a progress bar with a given width and fullness as a string.
///
/// # Arguments
///
/// * `width` - the width of the progress bar in characters
/// * `fullness` - the percentage of the progress bar filled, clamped to the range [0.0, 1.0]
pub fn progress_bar_string(width: usize, mut fullness: f32) -> String {
    if fullness < 0.0 {
        fullness = 0.0;
    } else if fullness > 1.0 {
        fullness = 1.0;
    }
    let pbstr = "\u{25A0}".repeat((fullness * width as f32).floor() as usize);
    let pbwid = "-".repeat(width - (fullness * width as f32).floor() as usize);
    let progress_bar = format!("[{}{}]", pbstr, pbwid);
    progress_bar
}
/// Returns a progress bar with a given width and fullness as a Text.
///
/// # Arguments
///
/// * `width` - the width of the progress bar in characters
/// * `fullness` - the percentage of the progress bar filled, clamped to the range [0.0, 1.0]
pub fn progress_bar_text(
    width: usize,
    fullness: f32,
    colour: Vec4,
    font_size: f32,
    font: Font,
) -> Text {
    Text {
        text: progress_bar_string(width, fullness),
        font: font.into(),
        font_size,
        colour,
    }
}
