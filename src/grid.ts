import type { ImageEntry } from "./types";

let selectedIndex = -1;
let _onActivate: ((path: string) => void) | null = null;

export function onActivate(cb: (path: string) => void): void { _onActivate = cb; }
export function triggerActivate(): void {
  const items = getItems();
  if (selectedIndex < 0 || selectedIndex >= items.length) return;
  const path = items[selectedIndex].querySelector("img")?.title;
  if (path) _onActivate?.(path);
}

export function getSelectedIndex(): number {
  return selectedIndex;
}

export function getItems(): HTMLElement[] {
  return Array.from(document.querySelectorAll<HTMLElement>(".thumb-item"));
}

export function resetSelection(): void {
  selectedIndex = -1;
}

export function setSelected(index: number): void {
  const items = getItems();
  if (items.length === 0) return;

  if (selectedIndex >= 0 && selectedIndex < items.length) {
    items[selectedIndex].classList.remove("selected");
  }

  selectedIndex = Math.max(0, Math.min(index, items.length - 1));
  items[selectedIndex].classList.add("selected");
  items[selectedIndex].scrollIntoView({ behavior: "smooth", block: "nearest" });
}

function formatMeta(entry: ImageEntry, sort: string): string {
  switch (sort) {
    case "date":
    case "date_old":
      return new Date(entry.modified * 1000).toLocaleDateString(undefined, {
        year: "numeric", month: "short", day: "numeric",
      });
    case "size":
    case "size_asc": {
      const mb = entry.size / (1024 * 1024);
      if (mb >= 1) return `${mb.toFixed(1)} MB`;
      const kb = entry.size / 1024;
      if (kb >= 1) return `${kb.toFixed(0)} KB`;
      return `${entry.size} B`;
    }
    default:
      return "";
  }
}

export function appendThumb(entry: ImageEntry, selectIt = false, sort = "name"): void {
  const grid = document.getElementById("grid")!;
  const item = document.createElement("div");
  item.className = "thumb-item";
  item.dataset.index = String(entry.index);

  const filename = entry.path.split("/").pop() ?? "";

  const img = document.createElement("img");
  img.src = entry.thumbnail;
  img.alt = filename;
  img.title = entry.path;

  const label = document.createElement("span");
  label.className = "thumb-name";
  label.textContent = filename;

  const metaText = formatMeta(entry, sort);
  const meta = document.createElement("span");
  meta.className = "thumb-meta";
  meta.textContent = metaText;

  item.appendChild(img);
  item.appendChild(label);
  if (metaText) item.appendChild(meta);

  item.addEventListener("click", () => setSelected(getItems().indexOf(item)));
  item.addEventListener("dblclick", () => {
    setSelected(getItems().indexOf(item));
    if (entry.path) _onActivate?.(entry.path);
  });

  // Insert at the correct sorted position rather than appending blindly.
  const items = getItems();
  const after = items.find((el) => Number(el.dataset.index) > entry.index) ?? null;
  grid.insertBefore(item, after);

  if (selectIt) setSelected(0);
}
