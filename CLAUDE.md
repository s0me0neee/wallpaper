# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

A cross-platform desktop wallpaper viewer built with **Tauri v2** (Rust backend + TypeScript/Vite frontend). Uses `pnpm` for frontend package management.

## Commands

```bash
# Development
pnpm tauri dev        # Run app with hot reload (starts Vite dev server + Rust backend)

# Build
pnpm tauri build      # Package the app for distribution

# Frontend only
pnpm dev              # Vite dev server on port 1420
pnpm build            # TypeScript compile + Vite bundle

# Rust backend (run from src-tauri/)
cargo build
cargo test
cargo check
```

## Architecture

The project follows the standard Tauri hybrid architecture:

- **`src/`** — Frontend (TypeScript + Vite). `main.ts` is the entry point; calls Rust commands via `invoke()` from `@tauri-apps/api`.
- **`src-tauri/src/`** — Rust backend. `lib.rs` sets up the Tauri app and command handlers; `fs.rs` contains image file system operations exposed as Tauri commands (`#[tauri::command]`). `main.rs` is just a thin wrapper.
- **`src-tauri/capabilities/default.json`** — Tauri v2 capability-based permissions for the main window. Any new plugin permissions must be added here.
- **`src-tauri/tauri.conf.json`** — App config: window size (800×600), dev server port (1420), CSP, bundle targets, and lifecycle hooks (`beforeDevCommand`, `beforeBuildCommand`).

### IPC Pattern

Frontend calls Rust via `invoke('command_name', { args })`. Rust functions annotated with `#[tauri::command]` must be registered in the `generate_handler![]` macro in `lib.rs` to be callable from the frontend.

### Key Dependencies

- `dirs` (Rust) — cross-platform home/config directory resolution
- `tauri-plugin-opener` — opening files/URLs on the host OS
- `serde` / `serde_json` — data serialization across the IPC boundary
