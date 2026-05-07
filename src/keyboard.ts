import { getCurrentWindow } from "@tauri-apps/api/window";
import { applyColumns, cols } from "./zoom";
import { getItems, getSelectedIndex, setSelected } from "./grid";

export function initKeyboard(): void {
  window.addEventListener("keydown", (e) => {
    if (e.key === "Escape") { getCurrentWindow().close(); return; }

    // Zoom: Ctrl +/-
    if (e.ctrlKey) {
      if (e.key === "+" || e.key === "=") { e.preventDefault(); applyColumns(cols - 1); }
      else if (e.key === "-")             { e.preventDefault(); applyColumns(cols + 1); }
      return;
    }

    // Skip navigation when typing in a form control
    if ((e.target as HTMLElement).matches("input, select, textarea")) return;

    const items = getItems();
    if (items.length === 0) return;

    const cur = getSelectedIndex() < 0 ? 0 : getSelectedIndex();
    let next = cur;

    switch (e.key) {
      case "h": case "ArrowLeft":  next = cur - 1;    break;
      case "l": case "ArrowRight": next = cur + 1;    break;
      case "k": case "ArrowUp":    next = cur - cols; break;
      case "j": case "ArrowDown":  next = cur + cols; break;
      default: return;
    }

    e.preventDefault();
    setSelected(Math.max(0, Math.min(next, items.length - 1)));
  });
}
