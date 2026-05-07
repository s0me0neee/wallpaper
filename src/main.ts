import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { initColumns } from "./zoom";
import { loadImages, currentDir, currentSort } from "./loader";
import { initKeyboard } from "./keyboard";

async function pickDirectory(): Promise<void> {
  const selected = await open({ directory: true, multiple: false });
  if (typeof selected === "string") {
    document.getElementById("current-dir")!.textContent = selected;
    loadImages(selected, currentSort);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  initColumns();
  initKeyboard();

  document.getElementById("pick-dir")!.addEventListener("click", pickDirectory);

  document.getElementById("sort-by")!.addEventListener("change", (e) => {
    loadImages(currentDir, (e.target as HTMLSelectElement).value);
  });

  const startDir = await invoke<string | null>("get_startup_dir");
  loadImages(startDir ?? undefined, currentSort);
});
