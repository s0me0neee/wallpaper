import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";

interface ImageEntry {
  path: string;
  thumbnail: string;
}

interface LoadDone {
  loaded: number;
  skipped: number;
}

// ── column / zoom state ────────────────────────────────────────────────────
const MIN_COLS = 2;
const MAX_COLS = 8;
let cols = 4;

function applyColumns(n: number) {
  cols = Math.max(MIN_COLS, Math.min(MAX_COLS, n));
  document.documentElement.style.setProperty("--cols", String(cols));
  // Fewer columns → taller rows so more image detail is visible.
  const rowHeight = Math.round(800 / cols);
  document.documentElement.style.setProperty("--row-height", `${rowHeight}px`);
}

window.addEventListener("keydown", (e) => {
  if (!e.ctrlKey) return;
  if (e.key === "+" || e.key === "=") {
    e.preventDefault();
    applyColumns(cols - 1); // fewer cols → larger thumbnails (zoom in)
  } else if (e.key === "-") {
    e.preventDefault();
    applyColumns(cols + 1); // more cols → smaller thumbnails (zoom out)
  }
});

// ── event listeners ────────────────────────────────────────────────────────
let unlisteners: UnlistenFn[] = [];

function stopListening() {
  unlisteners.forEach((fn) => fn());
  unlisteners = [];
}

function appendThumb(entry: ImageEntry) {
  const grid = document.getElementById("grid")!;
  const item = document.createElement("div");
  item.className = "thumb-item";
  const img = document.createElement("img");
  img.src = entry.thumbnail;
  img.alt = entry.path.split("/").pop() ?? "";
  img.title = entry.path;
  item.appendChild(img);
  grid.appendChild(item);
}

// ── loading ────────────────────────────────────────────────────────────────
async function loadImages(dir?: string) {
  stopListening();

  const grid = document.getElementById("grid")!;
  const label = document.getElementById("current-dir")!;
  grid.innerHTML = '<p class="status">Loading...</p>';
  let first = true;

  const unlistenThumb = await listen<ImageEntry>("thumbnail", (e) => {
    if (first) {
      grid.innerHTML = "";
      if (!dir) {
        const p = e.payload.path;
        label.textContent = p.substring(0, p.lastIndexOf("/"));
      }
      first = false;
    }
    appendThumb(e.payload);
  });

  const unlistenDone = await listen<LoadDone>("load-done", (e) => {
    stopListening();
    if (first) grid.innerHTML = '<p class="status">No images found.</p>';
    log(`Done — ${e.payload.loaded} loaded, ${e.payload.skipped} skipped`);
  });

  unlisteners = [unlistenThumb, unlistenDone];

  try {
    await invoke("start_load_images", { dir: dir ?? null });
  } catch (e) {
    stopListening();
    grid.innerHTML = `<p class="status error">Error: ${e}</p>`;
  }
}

function log(msg: string) {
  console.log(`[wallpaper] ${msg}`);
}

// ── toolbar ────────────────────────────────────────────────────────────────
async function pickDirectory() {
  const selected = await open({ directory: true, multiple: false });
  if (typeof selected === "string") {
    document.getElementById("current-dir")!.textContent = selected;
    loadImages(selected);
  }
}

window.addEventListener("DOMContentLoaded", () => {
  applyColumns(cols); // set initial --row-height
  document.getElementById("pick-dir")!.addEventListener("click", pickDirectory);
  loadImages();
});
