import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { appendThumb, resetSelection, setSelected } from "./grid";
import type { ImageEntry, LoadDone } from "./types";

export let currentDir: string | undefined;
export let currentSort = "name";

let unlisteners: UnlistenFn[] = [];

export function stopListening(): void {
  unlisteners.forEach((fn) => fn());
  unlisteners = [];
}

export async function loadImages(dir?: string, sort = currentSort): Promise<void> {
  console.log("[loader] loadImages start, dir:", dir ?? "(none)", "sort:", sort);
  stopListening();
  currentDir = dir;
  currentSort = sort;
  resetSelection();

  const grid = document.getElementById("grid")!;
  const dirLabel = document.getElementById("current-dir")!;
  grid.innerHTML = '<p class="status">Loading...</p>';
  let first = true;

  const unlistenThumb = await listen<ImageEntry>("thumbnail", (e) => {
    if (first) {
      grid.innerHTML = "";
      if (!dir) {
        const p = e.payload.path;
        dirLabel.textContent = p.substring(0, p.lastIndexOf("/"));
      }
      first = false;
      appendThumb(e.payload, false, currentSort);
    } else {
      appendThumb(e.payload, false, currentSort);
    }
  });

  const unlistenDone = await listen<LoadDone>("load-done", (e) => {
    stopListening();
    if (first) {
      grid.innerHTML = '<p class="status">No images found.</p>';
    } else {
      setSelected(0);
      invoke("focus_window").then(() => {
        console.log("[focus] focus_window resolved, activeElement:", document.activeElement?.tagName);
      }).catch(console.error);
    }
    console.log(`[wallpaper] Done — ${e.payload.loaded} loaded, ${e.payload.skipped} skipped`);
  });

  unlisteners = [unlistenThumb, unlistenDone];

  try {
    await invoke("start_load_images", { dir: dir ?? null, sortBy: sort });
  } catch (err) {
    stopListening();
    grid.innerHTML = `<p class="status error">Error: ${err}</p>`;
  }
}
