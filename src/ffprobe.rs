use crate::error::{Result, VidInfoError};
use crate::format_helpers::{
    aspect_ratio, format_fps, frame_rate_mode, friendly_codec, friendly_container_with_path,
};
use crate::model::{
    AudioStream, Chapter, ContainerInfo, MediaInfo, SubtitleStream, VideoStream,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Cached ffprobe binary path.
static FFPROBE: OnceLock<PathBuf> = OnceLock::new();

/// Raw ffprobe JSON root.
#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    #[serde(default)]
    streams: Vec<RawStream>,
    #[serde(default)]
    chapters: Vec<RawChapter>,
    format: Option<RawFormat>,
}

#[derive(Debug, Deserialize)]
struct RawFormat {
    #[serde(default)]
    #[allow(dead_code)]
    filename: Option<String>,
    format_name: Option<String>,
    format_long_name: Option<String>,
    duration: Option<String>,
    size: Option<String>,
    bit_rate: Option<String>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct RawStream {
    index: u32,
    codec_name: Option<String>,
    codec_long_name: Option<String>,
    profile: Option<String>,
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    coded_width: Option<u32>,
    coded_height: Option<u32>,
    pix_fmt: Option<String>,
    sample_aspect_ratio: Option<String>,
    display_aspect_ratio: Option<String>,
    color_range: Option<String>,
    color_space: Option<String>,
    color_transfer: Option<String>,
    color_primaries: Option<String>,
    bits_per_raw_sample: Option<String>,
    bits_per_sample: Option<u32>,
    r_frame_rate: Option<String>,
    avg_frame_rate: Option<String>,
    duration: Option<String>,
    bit_rate: Option<String>,
    nb_frames: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u32>,
    channel_layout: Option<String>,
    sample_fmt: Option<String>,
    #[serde(default)]
    tags: HashMap<String, String>,
    #[serde(default)]
    disposition: HashMap<String, i32>,
    #[serde(default)]
    side_data_list: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct RawChapter {
    id: Option<i64>,
    start_time: Option<String>,
    end_time: Option<String>,
    #[serde(default)]
    tags: HashMap<String, String>,
}

/// Locate `ffprobe` on PATH or next to the current executable (sidecar). Cached.
pub fn find_ffprobe() -> Result<&'static Path> {
    if let Some(p) = FFPROBE.get() {
        return Ok(p.as_path());
    }
    let found = locate_ffprobe()?;
    let _ = FFPROBE.set(found);
    Ok(FFPROBE.get().expect("ffprobe path just set").as_path())
}

fn locate_ffprobe() -> Result<PathBuf> {
    if let Ok(path) = which("ffprobe") {
        return Ok(path);
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sidecar = dir.join("ffprobe");
            if is_executable(&sidecar) {
                return Ok(sidecar);
            }
            #[cfg(windows)]
            {
                let sidecar_exe = dir.join("ffprobe.exe");
                if sidecar_exe.is_file() {
                    return Ok(sidecar_exe);
                }
            }
        }
    }

    Err(VidInfoError::FfprobeMissing)
}

fn which(name: &str) -> std::result::Result<PathBuf, ()> {
    let path_var = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Ok(candidate);
        }
        #[cfg(windows)]
        {
            let candidate_exe = dir.join(format!("{name}.exe"));
            if candidate_exe.is_file() {
                return Ok(candidate_exe);
            }
        }
    }
    Err(())
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Run ffprobe and return normalized MediaInfo.
pub fn probe_file(path: &Path) -> Result<MediaInfo> {
    if !path.exists() {
        return Err(VidInfoError::NotFound(path.to_path_buf()));
    }
    if !path.is_file() {
        return Err(VidInfoError::InvalidMedia {
            path: path.to_path_buf(),
            reason: "path is not a regular file".into(),
        });
    }

    std::fs::File::open(path).map_err(|source| VidInfoError::Unreadable {
        path: path.to_path_buf(),
        source,
    })?;

    let ffprobe = find_ffprobe()?;
    // `-v error` keeps one useful line on failure without dumping noise
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-print_format",
            "json",
            "-show_format",
            "-show_streams",
            "-show_chapters",
        ])
        .arg(path)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                VidInfoError::FfprobeMissing
            } else {
                VidInfoError::FfprobeFailed {
                    path: path.to_path_buf(),
                    reason: e.to_string(),
                }
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = clean_ffprobe_error(&stderr, output.status.code());
        return Err(VidInfoError::InvalidMedia {
            path: path.to_path_buf(),
            reason,
        });
    }

    if output.stdout.is_empty() {
        return Err(VidInfoError::InvalidMedia {
            path: path.to_path_buf(),
            reason: "empty response from ffprobe".into(),
        });
    }

    let raw: FfprobeOutput = serde_json::from_slice(&output.stdout).map_err(|e| {
        VidInfoError::FfprobeFailed {
            path: path.to_path_buf(),
            reason: format!("invalid JSON from ffprobe: {e}"),
        }
    })?;

    normalize(path, raw)
}

/// Turn ffprobe stderr into a short human reason.
fn clean_ffprobe_error(stderr: &str, code: Option<i32>) -> String {
    let line = stderr
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .unwrap_or("");

    if line.is_empty() {
        return format!(
            "unrecognized or unreadable media (ffprobe exit {})",
            code.unwrap_or(-1)
        );
    }

    // Strip common ffprobe prefixes / path noise
    let mut msg = line.to_string();
    for prefix in [
        "ffprobe version",
        "[error]",
        "Error ",
        "error ",
    ] {
        if let Some(rest) = msg.strip_prefix(prefix) {
            msg = rest.trim().to_string();
        }
    }

    // "Invalid data found when processing input" etc.
    if msg.contains("Invalid data found") || msg.contains("Invalid argument") {
        return "unrecognized or corrupt media data".into();
    }
    if msg.contains("Protocol not found") || msg.contains("No such file") {
        return msg;
    }
    if msg.len() > 160 {
        msg.truncate(157);
        msg.push_str("...");
    }
    msg
}

/// Parse ffprobe JSON bytes into MediaInfo (used by tests with fixtures).
pub fn parse_ffprobe_json(path: &Path, json: &[u8]) -> Result<MediaInfo> {
    let raw: FfprobeOutput = serde_json::from_slice(json)?;
    normalize(path, raw)
}

fn normalize(path: &Path, raw: FfprobeOutput) -> Result<MediaInfo> {
    let format = raw.format.ok_or_else(|| VidInfoError::InvalidMedia {
        path: path.to_path_buf(),
        reason: "no format information in ffprobe output".into(),
    })?;

    let format_name = format.format_name.unwrap_or_else(|| "unknown".into());
    if raw.streams.is_empty() && format.duration.is_none() {
        return Err(VidInfoError::InvalidMedia {
            path: path.to_path_buf(),
            reason: "no streams found".into(),
        });
    }

    let mut video = Vec::new();
    let mut audio = Vec::new();
    let mut subtitles = Vec::new();

    for s in &raw.streams {
        match s.codec_type.as_deref() {
            Some("video") => {
                let is_pic = s.disposition.get("attached_pic").copied().unwrap_or(0) != 0;
                if is_pic {
                    continue;
                }
                video.push(normalize_video(s));
            }
            Some("audio") => audio.push(normalize_audio(s)),
            Some("subtitle") => subtitles.push(normalize_subtitle(s)),
            _ => {}
        }
    }

    let chapters = raw
        .chapters
        .iter()
        .map(normalize_chapter)
        .collect::<Vec<_>>();

    let tags = &format.tags;
    let creation_time = tag_ci(tags, "creation_time");
    let encoder = tag_ci(tags, "encoder").or_else(|| tag_ci(tags, "encoding_tool"));

    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string());

    let size_bytes = parse_u64(format.size.as_deref()).unwrap_or_else(|| {
        std::fs::metadata(path)
            .map(|m| m.len())
            .unwrap_or(0)
    });

    Ok(MediaInfo {
        path: path.to_path_buf(),
        file_name,
        container: ContainerInfo {
            format: friendly_container_with_path(&format_name, Some(path)),
            format_long: format.format_long_name,
            size_bytes,
            duration_secs: parse_f64(format.duration.as_deref()),
            bitrate_bps: parse_u64(format.bit_rate.as_deref()),
            creation_time,
            encoder,
            video_count: video.len(),
            audio_count: audio.len(),
            subtitle_count: subtitles.len(),
        },
        video,
        audio,
        subtitles,
        chapters,
    })
}

fn normalize_video(s: &RawStream) -> VideoStream {
    let codec_raw = s
        .codec_name
        .clone()
        .or_else(|| s.codec_long_name.clone())
        .unwrap_or_else(|| "unknown".into());
    let profile = s.profile.clone().filter(|p| p != "unknown");
    let width = s.width.or(s.coded_width).unwrap_or(0);
    let height = s.height.or(s.coded_height).unwrap_or(0);

    let ar = s
        .display_aspect_ratio
        .as_deref()
        .filter(|d| *d != "0:1" && *d != "N/A")
        .map(|d| d.to_string())
        .unwrap_or_else(|| aspect_ratio(width, height, s.sample_aspect_ratio.as_deref()));

    let r_rate = s.r_frame_rate.as_deref().unwrap_or("0/0");
    let avg_rate = s.avg_frame_rate.as_deref().unwrap_or("0/0");
    let frame_rate = format_fps(avg_rate)
        .or_else(|| format_fps(r_rate))
        .or_else(|| s.avg_frame_rate.clone())
        .or_else(|| s.r_frame_rate.clone());

    let rotation = extract_rotation(s);

    VideoStream {
        index: s.index,
        codec: friendly_codec(&codec_raw, profile.as_deref()),
        codec_raw,
        profile,
        width,
        height,
        aspect_ratio: ar,
        frame_rate,
        frame_rate_mode: frame_rate_mode(r_rate, avg_rate).map(str::to_string),
        pixel_format: s.pix_fmt.clone(),
        color_space: s.color_space.clone(),
        color_range: s.color_range.clone(),
        color_transfer: s.color_transfer.clone(),
        color_primaries: s.color_primaries.clone(),
        // ffprobe writes bits_per_raw_sample as "0" when it does not know, so fall
        // through to the pixel format instead of reporting "0-bit".
        bit_depth: parse_u32(s.bits_per_raw_sample.as_deref())
            .filter(|&b| b > 0)
            .or_else(|| bit_depth_from_pix_fmt(s.pix_fmt.as_deref())),
        bitrate_bps: parse_u64(s.bit_rate.as_deref()),
        rotation,
        duration_secs: parse_f64(s.duration.as_deref()),
        frame_count: parse_u64(s.nb_frames.as_deref()),
        language: language_tag(&s.tags),
    }
}

fn normalize_audio(s: &RawStream) -> AudioStream {
    let codec_raw = s
        .codec_name
        .clone()
        .or_else(|| s.codec_long_name.clone())
        .unwrap_or_else(|| "unknown".into());
    let profile = s.profile.clone().filter(|p| p != "unknown");

    let bit_depth = s.bits_per_sample.filter(|&b| b > 0).or_else(|| {
        let raw = s.codec_name.as_deref().unwrap_or("");
        if raw.starts_with("pcm_") || raw == "flac" || raw == "alac" {
            bit_depth_from_sample_fmt(s.sample_fmt.as_deref())
        } else {
            None
        }
    });

    AudioStream {
        index: s.index,
        codec: friendly_codec(&codec_raw, profile.as_deref()),
        codec_raw,
        profile,
        sample_rate_hz: parse_u32(s.sample_rate.as_deref()),
        channels: s.channels,
        channel_layout: s.channel_layout.clone().or_else(|| {
            s.channels.map(|c| match c {
                1 => "mono".into(),
                2 => "stereo".into(),
                n => format!("{n} channels"),
            })
        }),
        bit_depth,
        bitrate_bps: parse_u64(s.bit_rate.as_deref()),
        language: language_tag(&s.tags),
        duration_secs: parse_f64(s.duration.as_deref()),
        title: tag_ci(&s.tags, "title"),
    }
}

fn normalize_subtitle(s: &RawStream) -> SubtitleStream {
    let codec_raw = s
        .codec_name
        .clone()
        .or_else(|| s.codec_long_name.clone())
        .unwrap_or_else(|| "unknown".into());

    SubtitleStream {
        index: s.index,
        codec: friendly_codec(&codec_raw, None),
        codec_raw,
        language: language_tag(&s.tags),
        title: tag_ci(&s.tags, "title"),
        is_forced: s.disposition.get("forced").copied().unwrap_or(0) != 0,
        is_default: s.disposition.get("default").copied().unwrap_or(0) != 0,
    }
}

fn normalize_chapter(c: &RawChapter) -> Chapter {
    Chapter {
        id: c.id.unwrap_or(0),
        start_secs: parse_f64(c.start_time.as_deref()).unwrap_or(0.0),
        end_secs: parse_f64(c.end_time.as_deref()).unwrap_or(0.0),
        title: tag_ci(&c.tags, "title"),
    }
}

fn extract_rotation(s: &RawStream) -> Option<i32> {
    if let Some(r) = tag_ci(&s.tags, "rotate") {
        if let Ok(v) = r.parse::<i32>() {
            if v != 0 {
                return Some(v);
            }
        }
    }

    for side in &s.side_data_list {
        if let Some(rot) = side.get("rotation") {
            if let Some(v) = rot.as_i64() {
                if v != 0 {
                    return Some(v as i32);
                }
            }
            if let Some(v) = rot.as_f64() {
                let i = v.round() as i32;
                if i != 0 {
                    return Some(i);
                }
            }
        }
    }
    None
}

fn language_tag(tags: &HashMap<String, String>) -> Option<String> {
    tag_ci(tags, "language").filter(|l| {
        let lower = l.to_ascii_lowercase();
        lower != "und" && lower != "unk" && lower != "unknown"
    })
}

fn tag_ci(tags: &HashMap<String, String>, key: &str) -> Option<String> {
    tags.get(key)
        .cloned()
        .or_else(|| {
            tags.iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v.clone())
        })
        .filter(|v| !v.is_empty())
}

fn parse_f64(s: Option<&str>) -> Option<f64> {
    s.and_then(|v| v.parse().ok())
        .filter(|v: &f64| v.is_finite())
}

fn parse_u64(s: Option<&str>) -> Option<u64> {
    s.and_then(|v| v.parse().ok())
}

fn parse_u32(s: Option<&str>) -> Option<u32> {
    s.and_then(|v| v.parse().ok())
}

fn bit_depth_from_pix_fmt(pix: Option<&str>) -> Option<u32> {
    let p = pix?;
    if p.contains("p10") || p.contains("10le") || p.contains("10be") {
        Some(10)
    } else if p.contains("p12") || p.contains("12le") {
        Some(12)
    } else if p.contains("p16") || p.contains("16le") {
        Some(16)
    } else if p.contains("p9") {
        Some(9)
    } else if p.starts_with("yuv") || p.starts_with("nv") || p == "rgb24" || p == "bgr24" {
        Some(8)
    } else {
        None
    }
}

fn bit_depth_from_sample_fmt(fmt: Option<&str>) -> Option<u32> {
    match fmt? {
        "u8" | "u8p" => Some(8),
        "s16" | "s16p" => Some(16),
        "s32" | "s32p" | "flt" | "fltp" => Some(32),
        "s64" | "s64p" | "dbl" | "dblp" => Some(64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::path::Path;

    const SAMPLE_MP4: &str = include_str!("../tests/fixtures/sample_mp4.json");
    const SAMPLE_MKV: &str = include_str!("../tests/fixtures/sample_mkv.json");

    #[test]
    fn parse_sample_mp4() {
        let info = parse_ffprobe_json(Path::new("b.mp4"), SAMPLE_MP4.as_bytes()).unwrap();
        assert_eq!(info.container.format, "MP4");
        assert_eq!(info.container.video_count, 1);
        assert_eq!(info.container.audio_count, 1);
        assert_eq!(info.container.subtitle_count, 0);
        assert!((info.container.duration_secs.unwrap() - 13.265).abs() < 0.001);
        assert_eq!(info.container.size_bytes, 1_196_650);

        let v = info.primary_video().unwrap();
        assert_eq!(v.codec, "H.264 High");
        assert_eq!(v.width, 1280);
        assert_eq!(v.height, 592);
        assert_eq!(v.pixel_format.as_deref(), Some("yuv420p"));
        assert_eq!(v.color_space.as_deref(), Some("bt709"));
        assert_eq!(v.bit_depth, Some(8));
        assert_eq!(v.frame_count, Some(403));
        assert_eq!(v.frame_rate_mode.as_deref(), Some("VFR"));

        let a = info.primary_audio().unwrap();
        assert_eq!(a.codec, "AAC LC");
        assert_eq!(a.sample_rate_hz, Some(44100));
        assert_eq!(a.channel_layout.as_deref(), Some("stereo"));
        assert_eq!(a.bitrate_bps, Some(128299));
    }

    #[test]
    fn parse_sample_mkv_with_subs_chapters() {
        let info = parse_ffprobe_json(Path::new("movie.mkv"), SAMPLE_MKV.as_bytes()).unwrap();
        assert_eq!(info.container.format, "MKV");
        assert_eq!(info.video.len(), 1);
        assert_eq!(info.audio.len(), 2);
        assert_eq!(info.subtitles.len(), 2);
        assert_eq!(info.chapters.len(), 2);

        assert_eq!(info.audio[0].language.as_deref(), Some("eng"));
        assert_eq!(info.audio[1].language.as_deref(), Some("jpn"));
        assert_eq!(info.subtitles[0].codec, "SRT");
        assert_eq!(info.chapters[0].title.as_deref(), Some("Opening"));
        assert_eq!(info.video[0].rotation, Some(90));
    }

    #[test]
    fn rejects_empty_streams() {
        let json = br#"{"streams":[],"chapters":[],"format":{"format_name":"mp4"}}"#;
        let err = parse_ffprobe_json(Path::new("x.mp4"), json).unwrap_err();
        assert!(matches!(err, VidInfoError::InvalidMedia { .. }));
    }

    #[test]
    fn cleans_ffprobe_errors() {
        assert_eq!(
            clean_ffprobe_error("Invalid data found when processing input\n", Some(1)),
            "unrecognized or corrupt media data"
        );
        assert!(clean_ffprobe_error("", Some(1)).contains("unreadable media"));
    }
}
