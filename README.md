# vidinfo

Readable ffprobe for the terminal.

A small Rust binary that prints media details in labeled sections: container, video, audio, subs, chapters. For folders you get one-line summaries or a comparison table. For scripts, normalized JSON. The parsing is ffprobe's, so the numbers match what FFmpeg sees.

```bash
vidinfo clip.mp4
```

```
clip.mp4

Container
  Format         MP4
  Size           1.14 MB
  Duration       00:00:13.265
  Bitrate        722 kbps

Video
  Codec          H.264 High
  Resolution     1280x720 (16:9)
  Frame rate     29.970 fps [CFR]
  …

Audio
  Codec          AAC LC
  Sample rate    48000 Hz
  Channels       stereo
```

## Install

You need [ffprobe](https://ffmpeg.org) on `PATH`, or next to the binary as a sidecar.

```bash
# macOS / Linux (from source)
cargo install --path .

# ffprobe
brew install ffmpeg   # macOS
sudo apt install ffmpeg
```

```bash
cargo build --release
./target/release/vidinfo --help
```

## Quick use

| Goal | Command |
|------|---------|
| One file | `vidinfo clip.mp4` |
| Folder summary | `vidinfo --compact ./clips/` |
| Compare clips | `vidinfo a.mp4 b.mp4 c.mkv` |
| JSON for scripts | `vidinfo --json clip.mp4 \| jq .` |
| Specific fields | `vidinfo --fields codec,resolution clip.mp4` |
| Recursive dir | `vidinfo -r ~/Movies/` |

```bash
vidinfo --compare --sort-by size ./exports/
vidinfo --fields audio,subtitles movie.mkv
vidinfo "./vacation/*.mp4"
```

## Flags

| Flag | Description |
|------|-------------|
| `PATHS…` | Files, directories, or globs |
| `-r, --recursive` | Walk subdirectories |
| `-c, --compact` | One line per file (`--summary`) |
| `--compare` | Side-by-side table |
| `--sort-by` | `name`, `duration`, `size`, `resolution`, `bitrate`, or `codec` |
| `--json` | Normalized JSON (stable field names, not raw ffprobe keys) |
| `--fields <list>` | e.g. `codec,resolution,audio` (applies to the per-file report) |
| `--color` / `--no-color` | Force ANSI on or off (`NO_COLOR` is honored) |

Exit codes: `0` ok, `1` all failed, `2` partial, `127` ffprobe missing.

If one file in a batch fails, the rest still run.

## How it works

```
file -> ffprobe JSON -> MediaInfo model -> report | compact | compare | json
```

Inspection only. No playback, thumbnails, or transcoding.

## Dev

```bash
cargo test
cargo build --release
```

Tests load JSON fixtures. You do not need sample video files.

## License

[MIT](LICENSE)

---

[Website](https://Alyetama.github.io/vidinfo)
