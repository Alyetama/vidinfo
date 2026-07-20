use crate::format_helpers::{format_duration, format_size};
use crate::model::MediaInfo;
use crate::render::{audio_codec_str, resolution_str, video_codec_str};
use std::io::{self, Write};

/// Single-line summary per file.
pub fn render_compact(info: &MediaInfo) -> io::Result<()> {
    let mut out = io::stdout().lock();
    render_compact_to(info, &mut out)
}

pub fn render_compact_to<W: Write>(info: &MediaInfo, out: &mut W) -> io::Result<()> {
    let dur = info
        .container
        .duration_secs
        .map(format_duration)
        .unwrap_or_else(|| "—".into());
    let res = resolution_str(info);
    let vcodec = video_codec_str(info);
    let acodec = audio_codec_str(info);
    let size = format_size(info.container.size_bytes);

    writeln!(
        out,
        "{:<40}  {:>12}  {:>11}  {:<16}  {:<10}  {:>8}",
        truncate(&info.file_name, 40),
        dur,
        res,
        truncate(&vcodec, 16),
        truncate(&acodec, 10),
        size
    )
}

pub fn render_compact_header() -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(
        out,
        "{:<40}  {:>12}  {:>11}  {:<16}  {:<10}  {:>8}",
        "FILE", "DURATION", "RESOLUTION", "VIDEO", "AUDIO", "SIZE"
    )?;
    writeln!(
        out,
        "{}",
        "-".repeat(40 + 2 + 12 + 2 + 11 + 2 + 16 + 2 + 10 + 2 + 8)
    )
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
    t.push('…');
    t
}
