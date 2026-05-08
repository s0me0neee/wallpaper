# wall

A lightweight floating wallpaper picker built with [Tauri v2](https://tauri.app). Browse a directory of images in a compact thumbnail grid, pick one, and set it as your wallpaper — or set one directly from the command line.

## Features

- Floating, always-on-top window (720 × 520, borderless)
- Parallel thumbnail generation with disk cache — near-instant on repeat opens
- Sort by name, date, or file size (ascending/descending)
- Ctrl +/- to zoom the grid (2–8 columns)
- hjkl / arrow key navigation
- Config auto-saved — remembers last directory, sort order, and column count
- Post-command hooks — run shell commands (or send a native notification) after every wallpaper change
- CLI mode — set a wallpaper from the terminal without opening the GUI
- Esc or ✕ to close

## Install

Download the latest release for your platform from the [Releases](../../releases) page:

- **macOS** — `.dmg` (arm64 or x86_64)
- **Linux** — `.deb` / `.AppImage`
- **Windows** — `.msi` / `.exe`

## CLI

```bash
wall /path/to/image.jpg     # set wallpaper, no GUI
wall -v /path/to/image.jpg  # verbose (debug) logging
wall -q /path/to/image.jpg  # quiet (errors only)
wall --help                 # print usage
wall --version              # print version
wall                        # open the GUI picker
```

On macOS the binary lives inside the app bundle. To use it as a command, symlink it:

```bash
ln -sf /Applications/wall.app/Contents/MacOS/wallpaper ~/.local/bin/wall
```

## Requirements

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) + [pnpm](https://pnpm.io)
- Tauri v2 system dependencies — see [Tauri prerequisites](https://tauri.app/start/prerequisites/)

On Linux, a compositor that supports transparent windows is recommended for rounded corners (Hyprland, KWin, Mutter, etc.).

## Development

```bash
pnpm install
pnpm tauri dev        # hot-reload; logs at trace level by default
pnpm tauri dev 2>&1   # pipe Rust output to terminal
```

Useful dev flags:

```bash
TEST=1  pnpm tauri dev   # auto-load ~/Pictures/wallpaper on startup
BENCH=1 pnpm tauri dev   # run thumbnail benchmarks and exit
```

## Build

```bash
pnpm tauri build
```

The release binary is placed in `src-tauri/target/release/` as `wall`. Releases for all platforms are built and published automatically via GitHub Actions on every `v*` tag push.

## Benchmarks

```bash
cd src-tauri
cargo bench
```

Runs a two-phase benchmark: a one-shot stats report (mean, p25/p50/p75, img/s for sequential and parallel generation), then criterion's rigorous confidence-interval measurements. An HTML report is written to `src-tauri/target/criterion/report/index.html`.

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `h` / `←` | Select left |
| `l` / `→` | Select right |
| `k` / `↑` | Select up |
| `j` / `↓` | Select down |
| `Ctrl` + `+` | More columns (zoom in) |
| `Ctrl` + `-` | Fewer columns (zoom out) |
| `Esc` | Close |

## Config

Saved automatically to `~/.config/wallpaper/config.toml` (on macOS, falls back to `~/Library/Application Support/wallpaper/config.toml` if `~/.config` doesn't exist):

```toml
image_dir = "/home/user/Pictures/wallpaper"
order = "name"
number_of_cols = 4
subdir = false

[post_command]
cmds = [
  "notify-send 'Wallpaper changed' '${{wallpaper}}'",
  "${{notify 'Wallpaper changed'}}",
]
```

`${{wallpaper}}` is replaced with the absolute path of the newly set image. `${{notify 'message'}}` sends a native OS notification without spawning a shell.

## Thumbnail Cache

Thumbnails are cached in `~/.cache/wallpaper/thumbnails/`. The cache is self-managing — stale entries (modified source file, entries older than 30 days, or total size over 200 MB) are evicted automatically on startup.

## Tech Stack

- **Backend** — Rust, Tauri v2, rayon, image crate (Lanczos3 resize), duct
- **Frontend** — TypeScript, Vite
- **Plugins** — tauri-plugin-cli, tauri-plugin-dialog, tauri-plugin-log, tauri-plugin-notification, tauri-plugin-opener
