#!/bin/sh
# vidinfo installer.
#   curl -fsSL https://raw.githubusercontent.com/Alyetama/vidinfo/main/install.sh | sh
#
# Downloads the release binary for this machine and puts it on your PATH.
# Override the install location with VIDINFO_BIN_DIR, e.g.
#   curl -fsSL .../install.sh | VIDINFO_BIN_DIR="$HOME/bin" sh
set -eu

REPO="Alyetama/vidinfo"

say() { printf '%s\n' "$*"; }
die() { printf 'error: %s\n' "$*" >&2; exit 1; }

command -v tar >/dev/null 2>&1 || die "this installer needs 'tar'"

if command -v curl >/dev/null 2>&1; then
    fetch_to() { curl -fsSL "$1" -o "$2"; }
elif command -v wget >/dev/null 2>&1; then
    fetch_to() { wget -qO "$2" "$1"; }
else
    die "this installer needs curl or wget"
fi

os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
    Darwin) os_part="apple-darwin" ;;
    Linux)  os_part="unknown-linux-gnu" ;;
    *) die "no prebuilt binary for $os; build from source with: cargo install --git https://github.com/$REPO" ;;
esac

case "$arch" in
    x86_64 | amd64) arch_part="x86_64" ;;
    arm64 | aarch64) arch_part="aarch64" ;;
    *) die "no prebuilt binary for $arch; build from source with: cargo install --git https://github.com/$REPO" ;;
esac

target="${arch_part}-${os_part}"
asset="vidinfo-${target}.tar.gz"
url="https://github.com/$REPO/releases/latest/download/$asset"

# Prefer a directory that is already on PATH and writable without sudo.
if [ -n "${VIDINFO_BIN_DIR:-}" ]; then
    bin_dir="$VIDINFO_BIN_DIR"
elif [ -w /usr/local/bin ]; then
    bin_dir="/usr/local/bin"
else
    bin_dir="$HOME/.local/bin"
fi
mkdir -p "$bin_dir" || die "cannot create $bin_dir"
[ -w "$bin_dir" ] || die "cannot write to $bin_dir (set VIDINFO_BIN_DIR to a directory you own)"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT INT TERM

say "Downloading vidinfo for $target"
fetch_to "$url" "$tmp/$asset" || die "download failed: $url"

tar -xzf "$tmp/$asset" -C "$tmp" || die "could not unpack $asset"
[ -f "$tmp/vidinfo" ] || die "archive did not contain a vidinfo binary"

chmod +x "$tmp/vidinfo"
mv "$tmp/vidinfo" "$bin_dir/vidinfo" || die "could not install to $bin_dir"

say "Installed $bin_dir/vidinfo"

case ":$PATH:" in
    *":$bin_dir:"*) ;;
    *) say "Add it to your PATH:  export PATH=\"$bin_dir:\$PATH\"" ;;
esac

if ! command -v ffprobe >/dev/null 2>&1; then
    say ""
    say "vidinfo reads files through ffprobe, which is not installed yet:"
    if [ "$os" = "Darwin" ]; then
        say "  brew install ffmpeg"
    else
        say "  sudo apt install ffmpeg   # or your distro's package manager"
    fi
fi
