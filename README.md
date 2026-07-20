# vidinfo

Readable ffprobe for the terminal.

![vidinfo showing a single-file report and a folder comparison table](docs/demo.gif)

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

```bash
curl -fsSL https://raw.githubusercontent.com/Alyetama/vidinfo/main/install.sh | sh
```

That grabs the right binary for macOS or Linux (Intel or ARM) and drops it in `/usr/local/bin`, or `~/.local/bin` if that needs root. Set `VIDINFO_BIN_DIR` to put it somewhere else.

vidinfo reads files through [ffprobe](https://ffmpeg.org), so you need FFmpeg too. The installer says so if it is missing:

```bash
brew install ffmpeg        # macOS
sudo apt install ffmpeg    # Debian / Ubuntu
```

ffprobe can also sit next to the vidinfo binary as a sidecar instead of being on `PATH`.

<details>
<summary>From source</summary>

```bash
cargo install --git https://github.com/Alyetama/vidinfo
```

or, in a clone:

```bash
cargo build --release
./target/release/vidinfo --help
```

</details>

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
