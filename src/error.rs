use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VidInfoError {
    #[error("file not found: {0}")]
    NotFound(PathBuf),

    #[error("cannot read file: {path}: {source}")]
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("not a valid media file: {path}: {reason}")]
    InvalidMedia { path: PathBuf, reason: String },

    #[error("ffprobe not found on PATH (install FFmpeg or place ffprobe next to this binary)")]
    FfprobeMissing,

    #[error("ffprobe failed for {path}: {reason}")]
    FfprobeFailed { path: PathBuf, reason: String },

    #[error("failed to parse ffprobe JSON: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, VidInfoError>;
