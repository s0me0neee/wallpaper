import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";
import { enableModernWindowStyle } from "@cloudworxx/tauri-plugin-mac-rounded-corners";
import { open } from "@tauri-apps/plugin-dialog";
import { initColumns, applyColumns, onColsChange } from "./zoom";
import { loadImages, currentDir, currentSort } from "./loader";
import { initKeyboard } from "./keyboard";
import { onActivate, flashApplied } from "./grid";
import type { AppConfig } from "./types";

let appConfig: AppConfig = {
  image_dir: null, order: "name", number_of_cols: 4, subdir: false,
  window_width: 720, window_height: 520, post_command: { cmds: [] },
  skip_set_wallpaper: false,
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
  const panel      = document.getElementById("settings-panel")!;
  const btn        = document.getElementById("settings-btn")!;
  const wInput     = document.getElementById("win-width")  as HTMLInputElement;
  const hInput     = document.getElementById("win-height") as HTMLInputElement;
  const skipToggle = document.getElementById("skip-set-wallpaper") as HTMLInputElement;

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

  let _sizeTimer: ReturnType<typeof setTimeout> | null = null;

  async function applySize(): Promise<void> {
    const w = Math.max(400, Math.min(2400, Number(wInput.value)));
    const h = Math.max(300, Math.min(1600, Number(hInput.value)));
    appConfig.window_width  = w;
    appConfig.window_height = h;
    saveConfig();
    const win = getCurrentWindow();
    await win.setSize(new LogicalSize(w, h));
    await win.center();
  }

  function scheduleSize(): void {
    if (_sizeTimer !== null) clearTimeout(_sizeTimer);
    _sizeTimer = setTimeout(() => { _sizeTimer = null; applySize(); }, 150);
  }

  function flushSize(): void {
    if (_sizeTimer !== null) { clearTimeout(_sizeTimer); _sizeTimer = null; }
    applySize();
  }

  wInput.addEventListener("input", scheduleSize);
  hInput.addEventListener("input", scheduleSize);
  wInput.addEventListener("change", flushSize);
  hInput.addEventListener("change", flushSize);

  skipToggle.addEventListener("change", () => {
    appConfig.skip_set_wallpaper = skipToggle.checked;
    saveConfig();
  });
}

window.addEventListener("DOMContentLoaded", async () => {
  console.log("[startup] DOMContentLoaded");
  if (navigator.userAgent.includes("Mac")) {
    document.documentElement.classList.add("macos");
  }
  // Delay show() by one rAF so WKWebView has committed its first paint before the
  // window is revealed — prevents the white compositor flash (tauri-apps/tauri#1564).
  const win = getCurrentWindow();
  requestAnimationFrame(() => {
    win.show().catch(() => {});
    invoke("focus_window").then(() => (document.activeElement as HTMLElement | null)?.blur()).catch(() => {});
  });

  enableModernWindowStyle({ cornerRadius: 12, offsetX: -12 }).catch(() => {});

  onActivate((path) => invoke("set_wallpaper", { path }).then(() => flashApplied(path)).catch(console.error));
  console.log("[startup] onActivate registered");

  initColumns();
  console.log("[startup] columns initialized");

  initKeyboard();
  console.log("[startup] keyboard handler registered");

  initSettings();
  console.log("[startup] settings initialized");

  document.getElementById("win-close")!.addEventListener("click", () => getCurrentWindow().close());
  document.getElementById("pick-dir")!.addEventListener("click", pickDirectory);

  document.getElementById("sort-by")!.addEventListener("change", (e) => {
    const sort = (e.target as HTMLSelectElement).value;
    appConfig.order = sort;
    saveConfig();
    (e.target as HTMLSelectElement).blur();
    loadImages(currentDir, sort);
  });

  onColsChange((n) => {
    appConfig.number_of_cols = n;
    saveConfig();
  });

  const [config, testDir] = await Promise.all([
    invoke<AppConfig>("get_config"),
    invoke<string | null>("get_startup_dir"),
  ]);
  appConfig = config;

  // Sync UI to saved config
  const sortEl = document.getElementById("sort-by") as HTMLSelectElement;
  sortEl.value = appConfig.order;
  (document.getElementById("win-width")  as HTMLInputElement).value = String(appConfig.window_width);
  (document.getElementById("win-height") as HTMLInputElement).value = String(appConfig.window_height);
  (document.getElementById("skip-set-wallpaper") as HTMLInputElement).checked = appConfig.skip_set_wallpaper;

  // Blur any form control that may have grabbed focus during config load
  (document.activeElement as HTMLElement | null)?.blur();
  invoke("focus_window").then(() => (document.activeElement as HTMLElement | null)?.blur()).catch(() => {});

  if (appConfig.number_of_cols !== 4) applyColumns(appConfig.number_of_cols);

  const startDir = testDir ?? appConfig.image_dir ?? undefined;
  loadImages(startDir, appConfig.order);
});
