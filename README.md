# wallpaper

A lightweight floating wallpaper picker built with [Tauri v2](https://tauri.app). Browse a directory of images in a compact thumbnail grid, pick one, and set it as your wallpaper.

## Features

- Floating, always-on-top window (720 × 520, borderless)
- Parallel thumbnail generation with disk cache — near-instant on repeat opens
- Sort by name, date, or file size (ascending/descending)
- Ctrl +/- to zoom the grid (2–8 columns)
- hjkl / arrow key navigation
- Config auto-saved — remembers last directory, sort order, and column count
- Esc or ✕ to close

## Requirements

- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) + [pnpm](https://pnpm.io)
- Tauri v2 system dependencies — see [Tauri prerequisites](https://tauri.app/start/prerequisites/)

On Linux, a compositor that supports transparent windows is recommended for rounded corners (Hyprland, KWin, Mutter, etc.).

## Development

```bash
pnpm install
pnpm tauri dev
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

The release binary is placed in `src-tauri/target/release/`.

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

Saved automatically to `~/.config/wallpaper/config.toml`:

```toml
image_dir = "/home/user/Pictures/wallpaper"
order = "name"
number_of_cols = 4
subdir = false
```

## Thumbnail Cache

Thumbnails are cached in `~/.cache/wallpaper/thumbnails/`. The cache is self-managing — stale entries (modified source file, entries older than 30 days, or total size over 200 MB) are cleaned up automatically on startup.

## Tech Stack

- **Backend** — Rust, Tauri v2, rayon, image crate (Lanczos3 resize)
- **Frontend** — TypeScript, Vite
