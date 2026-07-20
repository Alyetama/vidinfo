mod compact;
mod compare;
mod report;

pub use compact::{render_compact, render_compact_header};
pub use compare::{compare_by, render_compare, sort_media, SortField};
pub use json_out::render_json;
pub use report::render_report;

pub mod json_out;

use crate::model::MediaInfo;

/// Which top-level sections / field groups to show.
#[derive(Debug, Clone, Default)]
pub struct FieldFilter {
    /// If empty, show everything.
    pub selected: Vec<String>,
}

impl FieldFilter {
    pub fn from_csv(csv: Option<&str>) -> Self {
        let selected = csv
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_ascii_lowercase())
                    .filter(|p| !p.is_empty())
                    .collect()
            })
            .unwrap_or_default();
        Self { selected }
    }

    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    pub fn allow(&self, key: &str) -> bool {
        if self.selected.is_empty() {
            return true;
        }
        let key = key.to_ascii_lowercase();
        self.selected.iter().any(|s| match s.as_str() {
            "all" => true,
            // exact field
            sel if sel == key => true,
            // section aliases expand to their fields
            "video" => matches!(
                key.as_str(),
                "video"
                    | "codec"
                    | "resolution"
                    | "fps"
                    | "pixel_format"
                    | "color"
                    | "rotation"
                    | "bitrate"
            ),
            "audio" => matches!(
                key.as_str(),
                "audio" | "codec" | "sample_rate" | "channels" | "language" | "bitrate"
            ),
            "container" => matches!(
                key.as_str(),
                "container"
                    | "format"
                    | "size"
                    | "duration"
                    | "encoder"
                    | "creation_time"
                    | "bitrate"
            ),
            "subs" | "sub" | "subtitles" => key == "subtitles" || key == "subs" || key == "sub",
            "chapters" | "chapter" => key == "chapters" || key == "chapter",
            _ => false,
        })
    }

    pub fn any_section(&self, sections: &[&str]) -> bool {
        if self.selected.is_empty() {
            return true;
        }
        sections.iter().any(|s| self.allow(s))
    }
}

pub fn resolution_str(info: &MediaInfo) -> String {
    info.primary_video()
        .map(|v| format!("{}x{}", v.width, v.height))
        .unwrap_or_else(|| "—".into())
}

pub fn video_codec_str(info: &MediaInfo) -> String {
    info.primary_video()
        .map(|v| v.codec.clone())
        .unwrap_or_else(|| "—".into())
}

pub fn audio_codec_str(info: &MediaInfo) -> String {
    info.primary_audio()
        .map(|a| a.codec.clone())
        .unwrap_or_else(|| "—".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_exact_not_prefix() {
        let f = FieldFilter::from_csv(Some("codec"));
        assert!(f.allow("codec"));
        assert!(!f.allow("container"));
        assert!(!f.allow("color"));
        assert!(!f.allow("channels"));
        assert!(!f.allow("chapters"));
    }

    #[test]
    fn fields_section_video() {
        let f = FieldFilter::from_csv(Some("video"));
        assert!(f.allow("codec"));
        assert!(f.allow("resolution"));
        assert!(f.allow("fps"));
        assert!(!f.allow("sample_rate"));
    }

    #[test]
    fn fields_short_c_no_match() {
        let f = FieldFilter::from_csv(Some("c"));
        assert!(!f.allow("codec"));
        assert!(!f.allow("container"));
    }
}
