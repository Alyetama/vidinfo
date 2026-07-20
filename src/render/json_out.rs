use crate::model::MediaInfo;
use serde::Serialize;
use std::io::{self, Write};

/// Normalized JSON schema (not raw ffprobe field names).
#[derive(Serialize)]
struct JsonEnvelope<'a> {
    files: &'a [MediaInfo],
    errors: &'a [JsonError],
}

#[derive(Serialize)]
pub struct JsonError {
    pub path: String,
    pub error: String,
}

pub fn render_json(files: &[MediaInfo], errors: &[JsonError]) -> io::Result<()> {
    let env = JsonEnvelope { files, errors };
    let mut out = io::stdout().lock();
    serde_json::to_writer_pretty(&mut out, &env).map_err(io::Error::other)?;
    writeln!(out)?;
    Ok(())
}
