import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { initColumns, applyColumns, onColsChange } from "./zoom";
import { loadImages, currentDir, currentSort } from "./loader";
import { initKeyboard } from "./keyboard";
import type { AppConfig } from "./types";

let appConfig: AppConfig = { image_dir: null, order: "name", number_of_cols: 4, subdir: false };

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

window.addEventListener("DOMContentLoaded", async () => {
  initColumns();
  initKeyboard();

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

  // Load config, then decide startup directory (TEST env var takes priority)
  appConfig = await invoke<AppConfig>("get_config");

  // Sync sort-by dropdown to saved value
  const sortEl = document.getElementById("sort-by") as HTMLSelectElement;
  sortEl.value = appConfig.order;

  // Apply saved column count
  if (appConfig.number_of_cols !== 4) {
    applyColumns(appConfig.number_of_cols);
  }

  // TEST env var overrides saved dir
  const testDir = await invoke<string | null>("get_startup_dir");
  const startDir = testDir ?? appConfig.image_dir ?? undefined;
  loadImages(startDir, appConfig.order);
});
