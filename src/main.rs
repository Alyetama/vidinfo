use clap::Parser;
use owo_colors::OwoColorize;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use vidinfo::{
    find_ffprobe, probe_file, render_compact, render_compact_header, render_compare, render_json,
    render_report, resolve_inputs, sort_media, FieldFilter, JsonError, MediaInfo, SortField,
    VidInfoError,
};

#[derive(Debug, Parser)]
#[command(
    name = "vidinfo",
    version,
    about = "Friendly, well-organized video file inspector",
    long_about = "Inspect video/audio files with a clean terminal report. Shells out to ffprobe \
(FFmpeg) for accurate container/codec parsing.\n\nRequires ffprobe on PATH, or bundled as a \
sidecar next to this binary."
)]
struct Cli {
    /// One or more media files, directories, or glob patterns
    #[arg(required = true)]
    paths: Vec<String>,

    /// Recurse into subdirectories when a path is a directory
    #[arg(short, long)]
    recursive: bool,

    /// Full structured JSON (normalized schema) on stdout
    #[arg(long)]
    json: bool,

    /// One-line summary per file
    #[arg(short = 'c', long = "compact", visible_alias = "summary")]
    compact: bool,

    /// Side-by-side comparison table (default when multiple files succeed and neither --json nor --compact)
    #[arg(long)]
    compare: bool,

    /// Sort batch/compare table by field
    #[arg(long = "sort-by", value_enum)]
    sort_by: Option<SortField>,

    /// Comma-separated fields/sections to show (e.g. codec,resolution,audio)
    #[arg(long, value_name = "LIST")]
    fields: Option<String>,

    /// Force colored output
    #[arg(long, overrides_with = "no_color")]
    color: bool,

    /// Disable colored output
    #[arg(long)]
    no_color: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    // Reports go to stdout, diagnostics to stderr; each stream decides for itself so
    // `vidinfo clip.mp4 > out.txt` from a terminal writes no escape codes into the file.
    let color = want_color(&cli, io::stdout().is_terminal());
    let err_color = want_color(&cli, io::stderr().is_terminal());

    if let Err(e) = find_ffprobe() {
        eprint_err(err_color, &e.to_string());
        eprintln_plain(
            "  Install FFmpeg (includes ffprobe), e.g. `brew install ffmpeg` or `apt install ffmpeg`.",
        );
        return ExitCode::from(127);
    }

    let inputs = match resolve_inputs(&cli.paths, cli.recursive) {
        Ok(v) if v.is_empty() => {
            eprint_err(err_color, "no media files found");
            return ExitCode::FAILURE;
        }
        Ok(v) => v,
        Err(e) => {
            eprint_err(err_color, &e.to_string());
            return ExitCode::FAILURE;
        }
    };

    let filter = FieldFilter::from_csv(cli.fields.as_deref());

    let mut successes: Vec<MediaInfo> = Vec::new();
    let mut failures: Vec<(PathBuf, VidInfoError)> = Vec::new();

    for path in &inputs {
        match probe_file(path) {
            Ok(info) => successes.push(info),
            Err(e) => failures.push((path.clone(), e)),
        }
    }

    let has_fields = !filter.is_empty();

    // Mode priority: --json > --compact > --compare > auto.
    // Auto with several successful files: the compare table. `--fields` implies the
    // per-file report, since the table has a fixed set of columns.
    let auto_compare = !cli.json && !cli.compact && !has_fields && successes.len() > 1;
    let use_compare = (cli.compare || auto_compare) && successes.len() > 1;
    let use_compact = cli.compact && !cli.json;
    let use_report = !cli.json && !use_compare && !use_compact;

    let exit = if failures.is_empty() {
        ExitCode::SUCCESS
    } else if successes.is_empty() {
        ExitCode::FAILURE
    } else {
        ExitCode::from(2) // partial success
    };

    if cli.json {
        let errs: Vec<JsonError> = failures
            .iter()
            .map(|(p, e)| JsonError {
                path: p.display().to_string(),
                error: e.to_string(),
            })
            .collect();
        if let Err(e) = render_json(&successes, &errs) {
            return match write_failed(err_color, e) {
                Some(code) => code,
                None => exit,
            };
        }
        return exit;
    }

    // Sorting is about output order, so it applies to every rendered mode.
    if let Some(field) = cli.sort_by {
        sort_media(&mut successes, field);
    }

    let render_result = if use_compare {
        render_compare(&mut successes, None, color)
    } else if use_compact {
        render_compact_all(&successes)
    } else if use_report {
        render_all_reports(&successes, &filter, color)
    } else {
        Ok(())
    };

    if let Err(e) = render_result {
        if let Some(code) = write_failed(err_color, e) {
            return code;
        }
        return exit;
    }

    // Every VidInfoError already names the file it is about, so the path is not
    // repeated here.
    for (_path, err) in &failures {
        eprint_err(err_color, &err.to_string());
    }

    if !failures.is_empty() && !successes.is_empty() {
        if err_color {
            eprintln!(
                "\n{} {} ok, {} failed",
                "done:".yellow().bold(),
                successes.len(),
                failures.len()
            );
        } else {
            eprintln!(
                "\ndone: {} ok, {} failed",
                successes.len(),
                failures.len()
            );
        }
    }

    exit
}

fn render_compact_all(items: &[MediaInfo]) -> io::Result<()> {
    if items.is_empty() {
        return Ok(());
    }
    render_compact_header()?;
    for info in items {
        render_compact(info)?;
    }
    Ok(())
}

fn render_all_reports(items: &[MediaInfo], filter: &FieldFilter, color: bool) -> io::Result<()> {
    for (i, info) in items.iter().enumerate() {
        if i > 0 {
            let mut out = io::stdout().lock();
            writeln!(out, "{}", "─".repeat(60))?;
        }
        render_report(info, filter, color)?;
    }
    Ok(())
}

/// Report a write failure, unless it was a closed pipe (`vidinfo … | head`),
/// which is a normal way for the reader to stop and not an error worth printing.
/// Returns `Some(exit code)` when the caller should bail out with it.
fn write_failed(color: bool, e: io::Error) -> Option<ExitCode> {
    if e.kind() == io::ErrorKind::BrokenPipe {
        return None;
    }
    eprint_err(color, &e.to_string());
    Some(ExitCode::FAILURE)
}

fn want_color(cli: &Cli, stream_is_tty: bool) -> bool {
    if cli.no_color || std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if cli.color {
        return true;
    }
    stream_is_tty
}

fn eprint_err(color: bool, msg: &str) {
    let mut err = io::stderr().lock();
    if color {
        let _ = writeln!(err, "{} {msg}", "error:".red().bold());
    } else {
        let _ = writeln!(err, "error: {msg}");
    }
}

fn eprintln_plain(msg: &str) {
    let _ = writeln!(io::stderr().lock(), "{msg}");
}
