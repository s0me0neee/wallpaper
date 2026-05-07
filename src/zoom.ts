export const MIN_COLS = 2;
export const MAX_COLS = 8;
export let cols = 4;

let _onColsChange: ((n: number) => void) | null = null;
export function onColsChange(cb: (n: number) => void): void { _onColsChange = cb; }

export function applyColumns(n: number): void {
  const next = Math.max(MIN_COLS, Math.min(MAX_COLS, n));
  if (next === cols) return;
  const zoomIn = next < cols;
  cols = next;

  document.documentElement.style.setProperty("--cols", String(cols));
  document.documentElement.style.setProperty("--row-height", `${Math.round(560 / cols)}px`);

  const grid = document.getElementById("grid")!;
  grid.classList.remove("zoom-in", "zoom-out");
  void grid.offsetWidth; // restart animation
  grid.classList.add(zoomIn ? "zoom-in" : "zoom-out");
  setTimeout(() => grid.classList.remove("zoom-in", "zoom-out"), 250);

  _onColsChange?.(cols);
}

export function initColumns(): void {
  document.documentElement.style.setProperty("--cols", String(cols));
  document.documentElement.style.setProperty("--row-height", `${Math.round(560 / cols)}px`);
}
