import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { initColumns, applyColumns, onColsChange } from "./zoom";
import { loadImages, currentDir, currentSort } from "./loader";
import { initKeyboard } from "./keyboard";
import type { AppConfig } from "./types";

let appConfig: AppConfig = {
  image_dir: null, order: "name", number_of_cols: 4, subdir: false,
  window_width: 720, window_height: 520,
};

function saveConfig(): void {
  invoke("save_config", { setting: appConfig }).catch((e) => console.warn("save_config:", e));
}

async function pickDirectory(): Promise<void> {
  const selected = await open({ directory: true, multiple: false });
  if (typeof selected === "string") {
    document.getElementById("current-dir")!.textContent = selected;
    appConfig.image_dir = selected;
    saveConfig();
    loadImages(selected, currentSort);
  }
}

function initSettings(): void {
  const panel   = document.getElementById("settings-panel")!;
  const btn     = document.getElementById("settings-btn")!;
  const wInput  = document.getElementById("win-width")  as HTMLInputElement;
  const hInput  = document.getElementById("win-height") as HTMLInputElement;

  btn.addEventListener("click", (e) => {
    e.stopPropagation();
    panel.classList.toggle("hidden");
  });

  // Close panel when clicking outside
  document.addEventListener("click", (e) => {
    if (!panel.contains(e.target as Node) && e.target !== btn) {
      panel.classList.add("hidden");
    }
  });

  async function applySize(): Promise<void> {
    const w = Math.max(400, Math.min(2400, Number(wInput.value)));
    const h = Math.max(300, Math.min(1600, Number(hInput.value)));
    appConfig.window_width  = w;
    appConfig.window_height = h;
    const win = getCurrentWindow();
    await win.setSize(new LogicalSize(w, h));
    await win.center();
    saveConfig();
  }

  wInput.addEventListener("change", applySize);
  hInput.addEventListener("change", applySize);
}

window.addEventListener("DOMContentLoaded", async () => {
  initColumns();
  initKeyboard();
  initSettings();

  document.getElementById("win-close")!.addEventListener("click", () => getCurrentWindow().close());
  document.getElementById("pick-dir")!.addEventListener("click", pickDirectory);

  document.getElementById("sort-by")!.addEventListener("change", (e) => {
    const sort = (e.target as HTMLSelectElement).value;
    appConfig.order = sort;
    saveConfig();
    loadImages(currentDir, sort);
  });

  onColsChange((n) => {
    appConfig.number_of_cols = n;
    saveConfig();
  });

  appConfig = await invoke<AppConfig>("get_config");

  // Sync UI to saved config
  const sortEl = document.getElementById("sort-by") as HTMLSelectElement;
  sortEl.value = appConfig.order;
  (document.getElementById("win-width")  as HTMLInputElement).value = String(appConfig.window_width);
  (document.getElementById("win-height") as HTMLInputElement).value = String(appConfig.window_height);

  if (appConfig.number_of_cols !== 4) applyColumns(appConfig.number_of_cols);

  const testDir  = await invoke<string | null>("get_startup_dir");
  const startDir = testDir ?? appConfig.image_dir ?? undefined;
  loadImages(startDir, appConfig.order);
});
