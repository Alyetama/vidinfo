use crate::format_helpers::{format_bitrate, format_duration, format_size};
use crate::model::MediaInfo;
use crate::render::FieldFilter;
use owo_colors::OwoColorize;
use std::io::{self, Write};

pub fn render_report(info: &MediaInfo, filter: &FieldFilter, color: bool) -> io::Result<()> {
    let mut out = io::stdout().lock();
    render_report_to(info, filter, color, &mut out)
}

pub fn render_report_to<W: Write>(
    info: &MediaInfo,
    filter: &FieldFilter,
    color: bool,
    out: &mut W,
) -> io::Result<()> {
    writeln!(out)?;
    write_header(out, &info.file_name, color)?;
    writeln!(out, "  {}", dim(&info.path.display().to_string(), color))?;
    writeln!(out)?;

    if filter.any_section(&["container", "format", "size", "duration", "encoder", "creation_time"])
    {
        write_section(out, "Container", color)?;
        let c = &info.container;

        if filter.allow("format") {
            kv(out, "Format", &c.format, color)?;
        }
        if filter.allow("size") {
            kv(out, "Size", &format_size(c.size_bytes), color)?;
        }
        if filter.allow("duration") {
            let d = c
                .duration_secs
                .map(format_duration)
                .unwrap_or_else(|| "—".into());
            kv(out, "Duration", &d, color)?;
        }
        if filter.allow("bitrate") || filter.allow("container") {
            let br = c
                .bitrate_bps
                .map(format_bitrate)
                .unwrap_or_else(|| "—".into());
            kv(out, "Bitrate", &br, color)?;
        }
        if filter.allow("creation_time") {
            if let Some(ref t) = c.creation_time {
                kv(out, "Created", t, color)?;
            }
        }
        if filter.allow("encoder") {
            if let Some(ref e) = c.encoder {
                kv(out, "Encoder", e, color)?;
            }
        }
        if filter.allow("container") {
            kv(
                out,
                "Streams",
                &format!(
                    "{} video, {} audio, {} subtitle",
                    c.video_count, c.audio_count, c.subtitle_count
                ),
                color,
            )?;
        }
        writeln!(out)?;
    }

    if filter.any_section(&[
        "video",
        "codec",
        "resolution",
        "fps",
        "pixel_format",
        "color",
        "rotation",
        "bitrate",
    ]) && !info.video.is_empty()
    {
        for (i, v) in info.video.iter().enumerate() {
            let title = if info.video.len() == 1 {
                "Video".to_string()
            } else {
                format!("Video #{} (stream {})", i + 1, v.index)
            };
            write_section(out, &title, color)?;

            if filter.allow("codec") || filter.allow("video") {
                kv(out, "Codec", &v.codec, color)?;
            }
            if filter.allow("resolution") || filter.allow("video") {
                kv(
                    out,
                    "Resolution",
                    &format!("{}x{} ({})", v.width, v.height, v.aspect_ratio),
                    color,
                )?;
            }
            if filter.allow("fps") || filter.allow("video") {
                let mut fps = v.frame_rate.clone().unwrap_or_else(|| "—".into());
                if let Some(ref mode) = v.frame_rate_mode {
                    fps = format!("{fps} [{mode}]");
                }
                kv(out, "Frame rate", &fps, color)?;
            }
            if filter.allow("pixel_format") || filter.allow("video") {
                if let Some(ref pf) = v.pixel_format {
                    kv(out, "Pixel format", pf, color)?;
                }
            }
            if filter.allow("color") || filter.allow("video") {
                let color_bits: Vec<String> = [
                    v.color_space.as_deref(),
                    v.color_range.as_deref(),
                    v.color_transfer.as_deref(),
                    v.color_primaries.as_deref(),
                ]
                .into_iter()
                .flatten()
                .map(str::to_string)
                .collect();
                if !color_bits.is_empty() {
                    kv(out, "Color", &color_bits.join(" / "), color)?;
                }
            }
            if filter.allow("video") {
                if let Some(bd) = v.bit_depth {
                    kv(out, "Bit depth", &format!("{bd}-bit"), color)?;
                }
                if let Some(br) = v.bitrate_bps {
                    kv(out, "Bitrate", &format_bitrate(br), color)?;
                }
            }
            if filter.allow("rotation") || filter.allow("video") {
                if let Some(rot) = v.rotation {
                    kv(out, "Rotation", &format!("{rot}°"), color)?;
                }
            }
            if filter.allow("video") {
                if let Some(d) = v.duration_secs {
                    let container_d = info.container.duration_secs;
                    let show = container_d.map(|cd| (cd - d).abs() > 0.05).unwrap_or(true);
                    if show {
                        kv(out, "Duration", &format_duration(d), color)?;
                    }
                }
                if let Some(fc) = v.frame_count {
                    kv(out, "Frames", &fc.to_string(), color)?;
                }
                if let Some(ref lang) = v.language {
                    kv(out, "Language", lang, color)?;
                }
            }
            writeln!(out)?;
        }
    }

    if filter.any_section(&["audio", "codec", "sample_rate", "channels", "language", "bitrate"])
        && !info.audio.is_empty()
    {
        for (i, a) in info.audio.iter().enumerate() {
            let title = if info.audio.len() == 1 {
                "Audio".to_string()
            } else {
                format!("Audio #{} (stream {})", i + 1, a.index)
            };
            write_section(out, &title, color)?;

            if filter.allow("codec") || filter.allow("audio") {
                kv(out, "Codec", &a.codec, color)?;
            }
            if filter.allow("sample_rate") || filter.allow("audio") {
                if let Some(sr) = a.sample_rate_hz {
                    kv(out, "Sample rate", &format!("{sr} Hz"), color)?;
                }
            }
            if filter.allow("channels") || filter.allow("audio") {
                if let Some(ref layout) = a.channel_layout {
                    // Avoid "stereo (2 ch)" redundancy for mono/stereo labels
                    let text = match (layout.as_str(), a.channels) {
                        ("mono" | "stereo", _) => layout.clone(),
                        (_, Some(c)) if !layout.contains(char::is_numeric) => {
                            format!("{layout} ({c} ch)")
                        }
                        _ => layout.clone(),
                    };
                    kv(out, "Channels", &text, color)?;
                } else if let Some(c) = a.channels {
                    kv(out, "Channels", &format!("{c} ch"), color)?;
                }
            }
            if filter.allow("audio") {
                if let Some(bd) = a.bit_depth {
                    kv(out, "Bit depth", &format!("{bd}-bit"), color)?;
                }
                if let Some(br) = a.bitrate_bps {
                    kv(out, "Bitrate", &format_bitrate(br), color)?;
                }
            }
            if filter.allow("language") || filter.allow("audio") {
                if let Some(ref lang) = a.language {
                    kv(out, "Language", lang, color)?;
                }
            }
            if filter.allow("audio") {
                if let Some(ref t) = a.title {
                    kv(out, "Title", t, color)?;
                }
            }
            writeln!(out)?;
        }
    }

    if filter.any_section(&["subtitles", "subs", "sub"]) && !info.subtitles.is_empty() {
        write_section(out, "Subtitles", color)?;
        for s in &info.subtitles {
            let mut parts = vec![format!("#{} {}", s.index, s.codec)];
            if let Some(ref lang) = s.language {
                parts.push(lang.clone());
            }
            if let Some(ref t) = s.title {
                parts.push(format!("\"{t}\""));
            }
            let mut flags = Vec::new();
            if s.is_default {
                flags.push("default");
            }
            if s.is_forced {
                flags.push("forced");
            }
            if !flags.is_empty() {
                parts.push(format!("[{}]", flags.join(", ")));
            }
            writeln!(out, "  • {}", parts.join("  "))?;
        }
        writeln!(out)?;
    }

    if filter.any_section(&["chapters", "chapter"]) && !info.chapters.is_empty() {
        write_section(out, "Chapters", color)?;
        for ch in &info.chapters {
            let title = ch.title.as_deref().unwrap_or("(untitled)");
            writeln!(
                out,
                "  {:>3}. {} – {}  {}",
                ch.id,
                format_duration(ch.start_secs),
                format_duration(ch.end_secs),
                title
            )?;
        }
        writeln!(out)?;
    }

    Ok(())
}

fn write_header<W: Write>(out: &mut W, name: &str, color: bool) -> io::Result<()> {
    if color {
        writeln!(out, "{}", name.bold().cyan())
    } else {
        writeln!(out, "{name}")
    }
}

fn write_section<W: Write>(out: &mut W, title: &str, color: bool) -> io::Result<()> {
    if color {
        writeln!(out, "{}", title.bold().yellow())
    } else {
        writeln!(out, "── {title} ──")
    }
}

fn kv<W: Write>(out: &mut W, key: &str, value: &str, color: bool) -> io::Result<()> {
    // Pad before styling: ANSI escapes count toward `{:<14}` width, so padding a
    // styled string shifts every colored column out of alignment.
    let padded = format!("{key:<14}");
    if color {
        writeln!(out, "  {} {}", padded.dimmed(), value)
    } else {
        writeln!(out, "  {padded} {value}")
    }
}

fn dim(s: &str, color: bool) -> String {
    if color {
        s.dimmed().to_string()
    } else {
        s.to_string()
    }
}
