use crate::error::{Result, VidInfoError};
use glob::glob;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "flv", "mpeg", "mpg", "m2ts", "mts", "ts",
    "vob", "ogv", "3gp", "3g2", "f4v", "asf", "rm", "rmvb", "divx", "xvid", "mp3", "m4a", "aac",
    "flac", "wav", "ogg", "opus", "wma", "aiff", "aif",
];

/// Resolve CLI path arguments into a de-duplicated list of media files.
///
/// - Explicit file paths are kept as-is (even without a known media extension).
/// - Directories and globs only include known media extensions.
pub fn resolve_inputs(paths: &[String], recursive: bool) -> Result<Vec<PathBuf>> {
    let mut found = BTreeSet::new();
    let mut glob_errors: Vec<VidInfoError> = Vec::new();

    for raw in paths {
        if looks_like_glob(raw) {
            // A pattern matching nothing is only fatal when nothing else matched
            // either: `vidinfo *.mp4 *.mkv` should still work in a folder that
            // happens to hold no .mkv files.
            if let Err(e) = expand_glob(raw, &mut found) {
                glob_errors.push(e);
            }
            continue;
        }

        let path = PathBuf::from(raw);
        if !path.exists() {
            return Err(VidInfoError::NotFound(path));
        }

        if path.is_dir() {
            collect_dir(&path, recursive, &mut found)?;
        } else {
            // Explicit file: always include (user named it)
            found.insert(path);
        }
    }

    if found.is_empty() {
        if let Some(e) = glob_errors.into_iter().next() {
            return Err(e);
        }
    }

    Ok(found.into_iter().collect())
}

fn looks_like_glob(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[')
}

fn expand_glob(pattern: &str, out: &mut BTreeSet<PathBuf>) -> Result<()> {
    let entries = glob(pattern).map_err(|e| {
        VidInfoError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("invalid glob pattern '{pattern}': {e}"),
        ))
    })?;

    let mut any_match = false;
    let mut any_media = false;
    for entry in entries {
        match entry {
            Ok(p) if p.is_file() => {
                any_match = true;
                if is_media_extension(&p) {
                    any_media = true;
                    out.insert(p);
                }
            }
            Ok(_) => {}
            Err(e) => {
                return Err(VidInfoError::Io(std::io::Error::other(e.to_string())));
            }
        }
    }

    if !any_match {
        return Err(VidInfoError::NotFound(PathBuf::from(pattern)));
    }
    if !any_media {
        return Err(VidInfoError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("glob '{pattern}' matched files but none look like media"),
        )));
    }
    Ok(())
}

fn collect_dir(dir: &Path, recursive: bool, out: &mut BTreeSet<PathBuf>) -> Result<()> {
    let rd = std::fs::read_dir(dir).map_err(|source| VidInfoError::Unreadable {
        path: dir.to_path_buf(),
        source,
    })?;

    for entry in rd {
        let entry = entry.map_err(|source| VidInfoError::Unreadable {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        // `file_type()` does not follow symlinks, so a directory link pointing at an
        // ancestor cannot send the walk into an endless loop.
        let is_dir = entry
            .file_type()
            .map(|t| t.is_dir())
            .unwrap_or_else(|_| path.is_dir());
        if is_dir {
            if recursive {
                collect_dir(&path, true, out)?;
            }
        } else if is_media_extension(&path) {
            out.insert(path);
        }
    }
    Ok(())
}

fn is_media_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| {
            MEDIA_EXTENSIONS
                .iter()
                .any(|ext| e.eq_ignore_ascii_case(ext))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("vidinfo_test_{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn collects_media_from_dir() {
        let dir = tmp_dir();
        fs::write(dir.join("a.mp4"), b"x").unwrap();
        fs::write(dir.join("b.txt"), b"x").unwrap();
        fs::write(dir.join("c.MKV"), b"x").unwrap();

        let files = resolve_inputs(&[dir.to_string_lossy().into()], false).unwrap();
        let names: Vec<_> = files
            .iter()
            .filter_map(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
            .collect();
        assert!(names.contains(&"a.mp4".into()));
        assert!(names.contains(&"c.MKV".into()));
        assert!(!names.iter().any(|n| n == "b.txt"));

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn glob_skips_non_media() {
        let dir = tmp_dir();
        fs::write(dir.join("a.mp4"), b"x").unwrap();
        fs::write(dir.join("note.txt"), b"x").unwrap();
        let pattern = format!("{}/*", dir.display());
        let files = resolve_inputs(&[pattern], false).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].extension().unwrap() == "mp4");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn explicit_non_media_path_kept() {
        let dir = tmp_dir();
        let f = dir.join("readme.txt");
        fs::write(&f, b"x").unwrap();
        let files = resolve_inputs(&[f.to_string_lossy().into()], false).unwrap();
        assert_eq!(files.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_path_errors() {
        let err = resolve_inputs(&["/no/such/vidinfo_file_zzz.mp4".into()], false).unwrap_err();
        assert!(matches!(err, VidInfoError::NotFound(_)));
    }
}
