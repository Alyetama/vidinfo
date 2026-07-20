use crate::format_helpers::{format_bitrate, format_duration, format_size};
use crate::model::MediaInfo;
use crate::render::{audio_codec_str, resolution_str, video_codec_str};
use clap::ValueEnum;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, Color, ContentArrangement, Table};
use std::cmp::Ordering;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SortField {
    Name,
    Duration,
    Size,
    Resolution,
    Bitrate,
    Codec,
}

pub fn render_compare(
    items: &mut [MediaInfo],
    sort_by: Option<SortField>,
    color: bool,
) -> io::Result<()> {
    if let Some(field) = sort_by {
        sort_media(items, field);
    }

    let mut table = Table::new();
    // comfy-table only colors when stdout is a TTY; mirror our own decision so
    // `--no-color` / `NO_COLOR` strip the header styling and `--color` keeps it
    // when piping.
    if color {
        table.enforce_styling();
    } else {
        table.force_no_tty();
    }
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic)
        .set_header(vec![
            header("File"),
            header("Duration"),
            header("Resolution"),
            header("Video"),
            header("Audio"),
            header("Bitrate"),
            header("Size"),
        ]);

    for info in items.iter() {
        let dur = info
            .container
            .duration_secs
            .map(format_duration)
            .unwrap_or_else(|| "—".into());
        let br = info
            .container
            .bitrate_bps
            .map(format_bitrate)
            .unwrap_or_else(|| "—".into());

        table.add_row(vec![
            Cell::new(&info.file_name),
            Cell::new(dur),
            Cell::new(resolution_str(info)),
            Cell::new(video_codec_str(info)),
            Cell::new(audio_codec_str(info)),
            Cell::new(br),
            Cell::new(format_size(info.container.size_bytes)),
        ]);
    }

    let mut out = io::stdout().lock();
    writeln!(out, "{table}")
}

pub fn sort_media(items: &mut [MediaInfo], field: SortField) {
    items.sort_by(|a, b| compare_by(a, b, field));
}

pub fn compare_by(a: &MediaInfo, b: &MediaInfo, field: SortField) -> Ordering {
    match field {
        SortField::Name => a.file_name.cmp(&b.file_name),
        SortField::Duration => cmp_opt_f64(a.container.duration_secs, b.container.duration_secs),
        SortField::Size => a.container.size_bytes.cmp(&b.container.size_bytes),
        SortField::Resolution => pixel_count(a).cmp(&pixel_count(b)),
        SortField::Bitrate => cmp_opt_u64(a.container.bitrate_bps, b.container.bitrate_bps),
        SortField::Codec => video_codec_str(a).cmp(&video_codec_str(b)),
    }
}

fn header(s: &str) -> Cell {
    Cell::new(s)
        .add_attribute(Attribute::Bold)
        .fg(Color::Cyan)
}

fn pixel_count(info: &MediaInfo) -> u64 {
    info.primary_video()
        .map(|v| v.width as u64 * v.height as u64)
        .unwrap_or(0)
}

fn cmp_opt_f64(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn cmp_opt_u64(a: Option<u64>, b: Option<u64>) -> Ordering {
    match (a, b) {
        (Some(x), Some(y)) => x.cmp(&y),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}
