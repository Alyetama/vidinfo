use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Normalized media inspection result for one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    pub path: PathBuf,
    pub file_name: String,
    pub container: ContainerInfo,
    pub video: Vec<VideoStream>,
    pub audio: Vec<AudioStream>,
    pub subtitles: Vec<SubtitleStream>,
    pub chapters: Vec<Chapter>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub format: String,
    pub format_long: Option<String>,
    pub size_bytes: u64,
    pub duration_secs: Option<f64>,
    pub bitrate_bps: Option<u64>,
    pub creation_time: Option<String>,
    pub encoder: Option<String>,
    pub video_count: usize,
    pub audio_count: usize,
    pub subtitle_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStream {
    pub index: u32,
    pub codec: String,
    pub codec_raw: String,
    pub profile: Option<String>,
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: String,
    pub frame_rate: Option<String>,
    pub frame_rate_mode: Option<String>,
    pub pixel_format: Option<String>,
    pub color_space: Option<String>,
    pub color_range: Option<String>,
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,
    pub bit_depth: Option<u32>,
    pub bitrate_bps: Option<u64>,
    pub rotation: Option<i32>,
    pub duration_secs: Option<f64>,
    pub frame_count: Option<u64>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStream {
    pub index: u32,
    pub codec: String,
    pub codec_raw: String,
    pub profile: Option<String>,
    pub sample_rate_hz: Option<u32>,
    pub channels: Option<u32>,
    pub channel_layout: Option<String>,
    pub bit_depth: Option<u32>,
    pub bitrate_bps: Option<u64>,
    pub language: Option<String>,
    pub duration_secs: Option<f64>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleStream {
    pub index: u32,
    pub codec: String,
    pub codec_raw: String,
    pub language: Option<String>,
    pub title: Option<String>,
    pub is_forced: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chapter {
    pub id: i64,
    pub start_secs: f64,
    pub end_secs: f64,
    pub title: Option<String>,
}

impl MediaInfo {
    pub fn primary_video(&self) -> Option<&VideoStream> {
        self.video.first()
    }

    pub fn primary_audio(&self) -> Option<&AudioStream> {
        self.audio.first()
    }
}
