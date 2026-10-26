# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

A cross-platform desktop wallpaper viewer/picker built with **Tauri v2** (Rust backend + TypeScript/Vite frontend). Displays a thumbnail grid from a directory of images. Runs as a small floating window, always on top. Also ships a CLI mode: passing an image path as an argument sets the wallpaper and exits without opening the GUI. The executable is named `wall` (`productName` in `tauri.conf.json`).

## Commands

```bash
# Development
pnpm tauri dev             # hot-reload dev server + Rust backend
pnpm tauri dev 2>&1        # show Rust log output in terminal

# Build
pnpm tauri build           # release build — binary at src-tauri/target/release/wall

# Frontend only
pnpm dev                   # Vite dev server on port 1420
pnpm build                 # TypeScript compile + Vite bundle

# Rust (run from src-tauri/)
cargo check                # fast type-check
cargo test                 # run unit tests
cargo bench                # criterion thumbnail benchmarks

# Dev env vars
TEST=1 pnpm tauri dev      # auto-load ~/Pictures/wallpaper on startup
BENCH=1 pnpm tauri dev     # run criterion benchmarks and exit (no GUI)
```

## Architecture

Standard Tauri v2 hybrid: Rust backend streams data to a TypeScript frontend via IPC events.

### Rust — `src-tauri/src/`

| File           | Responsibility                                                                                                                                                                                                                                                     |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `lib.rs`       | App entry point — `check_early_flags()` handles `--help`/`--version` before Tauri starts; pre-parses verbosity flags, loads config, starts cleanup thread, registers plugins and commands; detects CLI mode in `setup` and exits early if an image path was passed |
| `config.rs`    | Load `~/.config/wallpaper/conf.lua` via mlua; extracts `Setting` fields from the returned table; registers `post_command` function in Lua globals; macOS falls back to `~/Library/Application Support` if `~/.config` doesn't exist                              |
| `commands.rs`  | Tauri command handlers exposed to the frontend; `set_wallpaper` applies the wallpaper then hands off to `hook::spawn_post_command`                                                                                                                                 |
| `hook.rs`      | Post-command runtime: registers the Lua `wall.run` helper, resolves the login shell's `PATH`/locale for Finder launches, runs `_wall_post_command` on a background thread                                                                                          |
| `scanner.rs`   | Reads a directory, filters image files, sorts by name/date/size                                                                                                                                                                                                    |
| `thumbnail.rs` | Generate JPEG thumbnails (Lanczos3, 1000×600, q92); disk cache in `~/.cache/wallpaper/thumbnails/`; cache cleanup on startup                                                                                                                                       |
| `types.rs`     | Shared serde types: `ImageEntry`, `LoadDone`, `SortBy`                                                                                                                                                                                                             |
| `test.rs`      | `TEST`/`BENCH` env var handling for dev                                                                                                                                                                                                                            |

**CLI mode** — `check_early_flags()` in `lib.rs` handles `-h`/`--help` and `-V`/`--version` by printing and exiting before the Tauri builder runs — no window flash. `verbosity_level()` then counts `-v`/`--verbose` occurrences. In the `setup` callback, `app.cli().matches()` is called; if an `image` arg is present, `wp::set_from_path` is called and the process exits — the window never appears.

On macOS the app bundle binary is at `/Applications/wall.app/Contents/MacOS/wallpaper`. Symlink it to use `wall` as a shell command (the system `wall` utility at `/usr/bin/wall` would otherwise take precedence).

**Thumbnail cache** filenames encode path hash + mtime + dimensions + quality so stale entries are automatically bypassed. `cleanup()` runs on startup in a background thread and evicts: duplicate versions of the same image, entries older than 30 days, and total size over 200 MB.

**Parallelism** — thumbnail generation uses a capped rayon thread pool (`clamp(2, 6)` threads). Rayon is used instead of tokio because decoding is CPU-bound, not I/O-bound.

**Post-commands** — if `post_command` is defined as a function in `conf.lua`, `commands::set_wallpaper` calls `hook::spawn_post_command`, which runs it on a background thread with the absolute image path as the sole argument. The Lua runtime (`mlua::Lua`) is shared via a `Mutex<Lua>` Tauri state, so hooks run one at a time; a queued hook is skipped if a newer pick arrived (`LATEST` counter). Lua errors are logged at error level.

`hook::register` adds a global `wall` table before `conf.lua` loads. `wall.run({prog, args...})` spawns via `duct` without a shell, with null stdin, a 30 s timeout, and the env from `hook::shell_env()`. It returns stdout, or raises a Lua error naming the program with its exit status and stderr. `shell_env()` is resolved once (warmed up at startup): when stdin isn't a terminal (Finder/Dock launch), it reads `PATH`/`LANG`/`LC_*` from `$SHELL -ilc` and falls back to `LANG=en_US.UTF-8` if no locale is set.

### TypeScript — `src/`

| File          | Responsibility                                                                |
| ------------- | ----------------------------------------------------------------------------- |
| `main.ts`     | Entry point — wires DOM events, loads config, initialises modules             |
| `loader.ts`   | Invokes `start_load_images`, listens for `thumbnail` / `load-done` events     |
| `grid.ts`     | Renders thumbnails, manages selection state                                   |
| `zoom.ts`     | Column count + row height CSS variables, zoom animations, col-change callback |
| `keyboard.ts` | hjkl / arrow key navigation, Ctrl+/- zoom, Esc to close                       |
| `types.ts`    | TypeScript interfaces mirroring Rust serde types                              |

### IPC Pattern

- **Commands** (`invoke`): request/response — `start_load_images`, `get_config`, `save_config`, `get_startup_dir`, `set_wallpaper`
- **Events** (`listen`): streaming — `thumbnail` (one per image), `load-done` (final count)

Commands must be registered in `generate_handler![]` in `lib.rs` **and** listed in `src-tauri/capabilities/default.json`.

### CLI Plugin Config

The CLI schema is defined in `tauri.conf.json` under `plugins.cli`:

- Positional arg `image` (index 1, optional) — image path to set as wallpaper
- Flag `-v` / `--verbose` (multi-occurrence) — increases log verbosity; one occurrence → debug, two or more → trace
- Flag `-q` / `--quiet` — suppresses output (error level only)

`-h`/`--help` and `-V`/`--version` are handled by `check_early_flags()` before the Tauri plugin sees the args — they are not declared in the `tauri.conf.json` schema.

### Config

Loaded from `~/.config/wallpaper/conf.lua` (macOS: XDG path preferred, `~/Library/Application Support` fallback). The file must return a Lua table:

```lua
return {
    image_dir      = "/home/user/Pictures/wallpaper",
    order          = "name",   -- name | name_desc | date | date_old | size | size_asc
    number_of_cols = 4,
    subdir         = false,
    window_width   = 720,
    window_height  = 520,
    skip_set_wallpaper = false,

    post_command = function(path)
        wall.run({ "wal", "-q", "-n", "-s", "-i", path })
    end,
}
```

All fields are optional and fall back to defaults. `save_config` (called from the frontend when the user changes directory, sort order, or column count) updates the **in-memory** state only — the Lua file is user-managed and never written back by the app.

### Key Dependencies (Rust)

| Crate                       | Purpose                                                              |
| --------------------------- | -------------------------------------------------------------------- |
| `image 0.25`                | Image decode + Lanczos3 resize (jpeg/png/webp/gif/bmp features only) |
| `rayon`                     | CPU-parallel thumbnail generation                                    |
| `duct`                      | Spawn post-command programs with captured output and a timeout       |
| `base64 0.22`               | Encode thumbnail bytes as data URLs for IPC                          |
| `dirs`                      | Cross-platform config/cache/pictures directory paths                 |
| `thiserror`                 | Error types in `config.rs`                                           |
| `mlua 0.11` (lua54+vendored) | Embedded Lua 5.4 runtime; loads `conf.lua` and calls `post_command` |
| `wp` (`wallpaper` crate v3) | Set the desktop wallpaper                                            |
| `tauri-plugin-cli`          | CLI argument parsing; schema in `tauri.conf.json`                    |
| `tauri-plugin-dialog`       | Native directory picker                                              |
| `tauri-plugin-log`          | Coloured, timestamped terminal logging                               |
| `criterion` (dev)           | Benchmark harness for `benches/thumbnail_bench.rs`                   |

### Window

Default 720 × 520 (configurable via `window_width`/`window_height` in `conf.lua`), borderless (`decorations: false`), transparent, always on top. Custom titlebar with `data-tauri-drag-region` for dragging. Capabilities: `core:default`, `opener:default`, `dialog:default`, `core:window:allow-close`, `core:window:allow-minimize`, `core:window:allow-set-size`, `core:window:allow-set-focus`, `core:window:allow-show`, `core:window:allow-start-dragging`, `core:window:allow-is-fullscreen`, `core:window:allow-is-maximized`, `core:event:allow-listen`, `cli:default`.
