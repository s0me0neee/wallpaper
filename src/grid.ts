import type { ImageEntry } from "./types";

let selectedIndex = -1;

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

export function appendThumb(entry: ImageEntry, selectIt = false): void {
  const grid = document.getElementById("grid")!;
  const item = document.createElement("div");
  item.className = "thumb-item";

  const filename = entry.path.split("/").pop() ?? "";

  const img = document.createElement("img");
  img.src = entry.thumbnail;
  img.alt = filename;
  img.title = entry.path;

  const label = document.createElement("span");
  label.className = "thumb-name";
  label.textContent = filename;

  item.appendChild(img);
  item.appendChild(label);
  item.addEventListener("click", () => setSelected(getItems().indexOf(item)));
  grid.appendChild(item);

  if (selectIt) setSelected(getItems().length - 1);
}
