/// Format a duration in seconds as `HH:MM:SS` or `HH:MM:SS.mmm` when fractional.
pub fn format_duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.0 {
        return "—".to_string();
    }

    let total_ms = (seconds * 1000.0).round() as u64;
    let ms = total_ms % 1000;
    let total_secs = total_ms / 1000;
    let s = total_secs % 60;
    let total_mins = total_secs / 60;
    let m = total_mins % 60;
    let h = total_mins / 60;

    if ms == 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{h:02}:{m:02}:{s:02}.{ms:03}")
    }
}

/// Format a bitrate in bits/sec as human-readable kbps or Mbps.
pub fn format_bitrate(bps: u64) -> String {
    if bps == 0 {
        return "—".to_string();
    }
    let kbps = bps as f64 / 1000.0;
    if kbps >= 1000.0 {
        format!("{:.2} Mbps", kbps / 1000.0)
    } else {
        format!("{:.0} kbps", kbps)
    }
}

/// Format a file size in bytes as B / KB / MB / GB.
pub fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.2} GB", b / GB)
    } else if b >= MB {
        format!("{:.2} MB", b / MB)
    } else if b >= KB {
        format!("{:.1} KB", b / KB)
    } else {
        format!("{bytes} B")
    }
}

/// Format frame rate from a rational string like `"30000/1001"` or `"30/1"`.
pub fn format_fps(rate: &str) -> Option<String> {
    let (num, den) = parse_rational(rate)?;
    if den == 0.0 || num == 0.0 {
        return None;
    }
    let fps = num / den;
    if (fps - fps.round()).abs() < 0.01 {
        Some(format!("{:.0} fps", fps.round()))
    } else {
        Some(format!("{fps:.3} fps"))
    }
}

/// Parse `"num/den"` or a plain float string into (num, den).
pub fn parse_rational(s: &str) -> Option<(f64, f64)> {
    let s = s.trim();
    if s.is_empty() || s == "0/0" {
        return None;
    }
    if let Some((a, b)) = s.split_once('/') {
        let num: f64 = a.parse().ok()?;
        let den: f64 = b.parse().ok()?;
        Some((num, den))
    } else {
        let v: f64 = s.parse().ok()?;
        Some((v, 1.0))
    }
}

/// Detect constant vs variable frame rate from r_frame_rate vs avg_frame_rate.
pub fn frame_rate_mode(r_frame_rate: &str, avg_frame_rate: &str) -> Option<&'static str> {
    let r = parse_rational(r_frame_rate)?;
    let a = parse_rational(avg_frame_rate)?;
    if r.1 == 0.0 || a.1 == 0.0 {
        return None;
    }
    let rf = r.0 / r.1;
    let af = a.0 / a.1;
    if rf == 0.0 || af == 0.0 {
        return None;
    }
    // Allow small floating tolerance
    if (rf - af).abs() / rf.max(af) < 0.02 {
        Some("CFR")
    } else {
        Some("VFR")
    }
}

/// Compute display aspect ratio string from width/height (and optional SAR).
pub fn aspect_ratio(width: u32, height: u32, sample_aspect: Option<&str>) -> String {
    if width == 0 || height == 0 {
        return "—".to_string();
    }

    let mut w = width as f64;
    let h = height as f64;

    if let Some(sar) = sample_aspect {
        if let Some((sn, sd)) = parse_rational(sar) {
            if sn > 0.0 && sd > 0.0 && (sn - sd).abs() > f64::EPSILON {
                w *= sn / sd;
            }
        }
    }

    let g = gcd((w.round() as u64).max(1), (h.round() as u64).max(1));
    let rw = (w.round() as u64) / g;
    let rh = (h.round() as u64) / g;

    // Prefer common labels
    let ratio = w / h;
    let label = if (ratio - 16.0 / 9.0).abs() < 0.02 {
        "16:9"
    } else if (ratio - 4.0 / 3.0).abs() < 0.02 {
        "4:3"
    } else if (ratio - 21.0 / 9.0).abs() < 0.03 {
        "21:9"
    } else if (ratio - 1.0).abs() < 0.02 {
        "1:1"
    } else if rw < 100 && rh < 100 {
        return format!("{rw}:{rh}");
    } else {
        return format!("{ratio:.3}:1");
    };

    label.to_string()
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a.max(1)
}

/// Map common ffprobe format_name values to short friendly labels.
/// `path_hint` (file name/extension) disambiguates overlapping format lists
/// like `matroska,webm` and `mov,mp4,...`.
pub fn friendly_container(format_name: &str) -> String {
    friendly_container_with_path(format_name, None)
}

pub fn friendly_container_with_path(format_name: &str, path: Option<&std::path::Path>) -> String {
    let lower = format_name.to_ascii_lowercase();
    let ext = path
        .and_then(|p| p.extension())
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());

    // ffprobe reports a list of candidate formats for containers that share a
    // demuxer ("mov,mp4,m4a,3gp,3g2,mj2"). The extension picks between them, but
    // only when ffprobe agrees it is one of the candidates — a `.mp4` file that
    // really holds Matroska should still be reported as MKV.
    if let Some(ref e) = ext {
        let (demuxer, label) = match e.as_str() {
            "mp4" | "m4v" => ("mp4", "MP4"),
            "mov" => ("mov", "MOV"),
            "m4a" => ("m4a", "M4A"),
            "mkv" => ("matroska", "MKV"),
            "webm" => ("webm", "WebM"),
            "avi" => ("avi", "AVI"),
            "wmv" => ("asf", "WMV"),
            "flv" => ("flv", "FLV"),
            "ts" | "m2ts" | "mts" => ("mpegts", "MPEG-TS"),
            "mp3" => ("mp3", "MP3"),
            "flac" => ("flac", "FLAC"),
            "wav" => ("wav", "WAV"),
            "ogg" | "ogv" | "oga" => ("ogg", "Ogg"),
            "opus" => ("ogg", "Opus"),
            _ => ("", ""),
        };
        if !demuxer.is_empty() && lower.split(',').any(|f| f.trim() == demuxer) {
            return label.into();
        }
    }

    let primary = lower
        .split(',')
        .next()
        .unwrap_or(&lower)
        .trim();

    match primary {
        "mov" | "mp4" | "m4a" | "3gp" | "3g2" | "mj2" => {
            if lower.contains("mp4") {
                "MP4".into()
            } else {
                "MOV".into()
            }
        }
        "matroska" | "webm" => {
            // ffprobe lists both for either container; default MKV unless only webm
            if primary == "webm" || (lower.contains("webm") && !lower.contains("matroska")) {
                "WebM".into()
            } else {
                "MKV".into()
            }
        }
        "avi" => "AVI".into(),
        "mpegts" | "mpeg" => "MPEG-TS".into(),
        "flv" => "FLV".into(),
        "ogg" => "Ogg".into(),
        "wav" => "WAV".into(),
        "mp3" => "MP3".into(),
        "flac" => "FLAC".into(),
        "asf" => "ASF/WMV".into(),
        other => other.to_ascii_uppercase(),
    }
}

/// Friendly codec display name.
pub fn friendly_codec(name: &str, profile: Option<&str>) -> String {
    let lower = name.to_ascii_lowercase();
    let base = match lower.as_str() {
        "h264" | "avc" => "H.264".to_string(),
        "hevc" | "h265" => "H.265/HEVC".to_string(),
        "av1" => "AV1".to_string(),
        "vp9" => "VP9".to_string(),
        "vp8" => "VP8".to_string(),
        "mpeg4" => "MPEG-4".to_string(),
        "mpeg2video" => "MPEG-2".to_string(),
        "prores" => "ProRes".to_string(),
        "ffv1" => "FFV1".to_string(),
        "aac" => "AAC".to_string(),
        "mp3" => "MP3".to_string(),
        "opus" => "Opus".to_string(),
        "vorbis" => "Vorbis".to_string(),
        "flac" => "FLAC".to_string(),
        "ac3" => "AC-3".to_string(),
        "eac3" => "E-AC-3".to_string(),
        "dts" => "DTS".to_string(),
        "pcm_s16le" => "PCM 16-bit".to_string(),
        "pcm_s24le" => "PCM 24-bit".to_string(),
        "truehd" => "TrueHD".to_string(),
        "subrip" | "srt" => "SRT".to_string(),
        "ass" | "ssa" => "ASS".to_string(),
        "hdmv_pgs_subtitle" => "PGS".to_string(),
        "dvd_subtitle" => "VobSub".to_string(),
        "mov_text" => "Timed Text".to_string(),
        "webvtt" => "WebVTT".to_string(),
        other => other.to_string(),
    };

    match profile {
        Some(p) if !p.is_empty() && p != "unknown" => format!("{base} {p}"),
        _ => base,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn duration_whole_seconds() {
        assert_eq!(format_duration(0.0), "00:00:00");
        assert_eq!(format_duration(65.0), "00:01:05");
        assert_eq!(format_duration(3661.0), "01:01:01");
    }

    #[test]
    fn duration_fractional() {
        assert_eq!(format_duration(13.265), "00:00:13.265");
        assert_eq!(format_duration(90.5), "00:01:30.500");
    }

    #[test]
    fn duration_invalid() {
        assert_eq!(format_duration(-1.0), "—");
        assert_eq!(format_duration(f64::NAN), "—");
    }

    #[test]
    fn bitrate_units() {
        assert_eq!(format_bitrate(0), "—");
        assert_eq!(format_bitrate(128_000), "128 kbps");
        assert_eq!(format_bitrate(5_857_540), "5.86 Mbps");
    }

    #[test]
    fn size_units() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(2048), "2.0 KB");
        assert_eq!(format_size(1_196_650), "1.14 MB");
        assert_eq!(format_size(2_147_483_648), "2.00 GB");
    }

    #[test]
    fn fps_formatting() {
        assert_eq!(format_fps("30/1").as_deref(), Some("30 fps"));
        assert_eq!(format_fps("30000/1001").as_deref(), Some("29.970 fps"));
        assert_eq!(format_fps("0/0"), None);
    }

    #[test]
    fn cfr_vs_vfr() {
        assert_eq!(frame_rate_mode("30/1", "30/1"), Some("CFR"));
        assert_eq!(frame_rate_mode("60000/1001", "80600/2653"), Some("VFR"));
    }

    #[test]
    fn aspect_common() {
        assert_eq!(aspect_ratio(1920, 1080, None), "16:9");
        assert_eq!(aspect_ratio(640, 480, None), "4:3");
        assert_eq!(aspect_ratio(1080, 1080, None), "1:1");
    }

    #[test]
    fn friendly_names() {
        assert_eq!(friendly_container("mov,mp4,m4a,3gp,3g2,mj2"), "MP4");
        assert_eq!(friendly_container("matroska,webm"), "MKV");
        assert_eq!(
            friendly_container_with_path(
                "matroska,webm",
                Some(std::path::Path::new("clip.webm"))
            ),
            "WebM"
        );
        // A misnamed file is reported by what it actually is, not what it claims.
        assert_eq!(
            friendly_container_with_path(
                "matroska,webm",
                Some(std::path::Path::new("mislabeled.mp4"))
            ),
            "MKV"
        );
        assert_eq!(friendly_codec("h264", Some("High")), "H.264 High");
        assert_eq!(friendly_codec("aac", Some("LC")), "AAC LC");
    }
}
