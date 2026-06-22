import type { Sheet } from "./types.js";

export interface ValidationDropdownOptions {
  getEditInput: () => HTMLInputElement;
  getSheet: () => Sheet;
  onAccept: (value: string) => void;
}

export interface ValidationDropdownHandle {
  open(cell: { r: number; c: number }, opts: { typed: boolean }): void;
  refresh(typed: boolean): void;
  handleKey(ev: KeyboardEvent): boolean;
  isOpen(): boolean;
  isActive(): boolean;
  hasOptions(): boolean;
  close(): void;
  reset(): void;
  destroy(): void;
}

export function createValidationDropdown(
  options: ValidationDropdownOptions,
): ValidationDropdownHandle {
  const menu = document.createElement("div");
  menu.style.cssText =
    "position:fixed;z-index:1100;display:none;background:#fff;border:1px solid #d4d4d8;border-radius:6px;" +
    "box-shadow:0 8px 24px rgba(15,23,42,0.18);padding:4px;min-width:120px;max-height:240px;overflow:auto;" +
    "font:13px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;";
  document.body.append(menu);

  let validationOptions: string[] | null = null;
  let validationFiltered: string[] = [];
  let validationActive = 0;
  let validationTyped = false;

  function validationListFor(cell: { r: number; c: number }): string[] | null {
    const sheet = options.getSheet();
    const dropdowns = sheet.validationDropdowns ?? [];
    const d = dropdowns.find((dd) => dd.r === cell.r && dd.c === cell.c);
    if (!d) return null;
    const lists = sheet.validationLists ?? [];
    return lists[d.list] ?? null;
  }

  function update(): void {
    if (!validationOptions) return close();
    const input = options.getEditInput();
    const q = validationTyped ? input.value.trim().toLowerCase() : "";
    const all = validationOptions;
    validationFiltered = q ? all.filter((o) => o.toLowerCase().includes(q)) : all.slice();
    if (validationFiltered.length === 0) return close();
    if (validationActive >= validationFiltered.length) validationActive = 0;
    render();
  }

  function itemCss(active: boolean): string {
    return `padding:4px 8px;cursor:pointer;border-radius:4px;white-space:nowrap;${
      active ? "background:#2563eb;color:#fff;" : "color:#111827;"
    }`;
  }

  function restyle(): void {
    Array.from(menu.children).forEach((child, i) => {
      (child as HTMLElement).style.cssText = itemCss(i === validationActive);
    });
  }

  function render(): void {
    const input = options.getEditInput();
    menu.replaceChildren();
    validationFiltered.forEach((value, i) => {
      const item = document.createElement("div");
      item.textContent = value;
      item.style.cssText = itemCss(i === validationActive);
      item.onmousedown = (ev) => {
        ev.preventDefault();
        accept(i);
      };
      item.onmouseenter = () => {
        validationActive = i;
        restyle();
      };
      menu.append(item);
    });
    const rect = input.getBoundingClientRect();
    menu.style.left = `${rect.left}px`;
    menu.style.top = `${rect.bottom + 2}px`;
    menu.style.minWidth = `${Math.max(120, rect.width)}px`;
    menu.style.display = "block";
  }

  function close(): void {
    validationActive = 0;
    validationFiltered = [];
    menu.style.display = "none";
  }

  function isOpen(): boolean {
    return menu.style.display !== "none";
  }

  function accept(index: number): void {
    const value = validationFiltered[index];
    if (value === undefined) return;
    options.onAccept(value);
  }

  function handleKey(ev: KeyboardEvent): boolean {
    if (!isOpen()) return false;
    const n = validationFiltered.length;
    if (n === 0) return false;
    if (ev.key === "ArrowDown") {
      ev.preventDefault();
      validationActive = (validationActive + 1) % n;
      render();
      return true;
    }
    if (ev.key === "ArrowUp") {
      ev.preventDefault();
      validationActive = (validationActive - 1 + n) % n;
      render();
      return true;
    }
    if (ev.key === "Enter" || ev.key === "Tab") {
      ev.preventDefault();
      accept(validationActive);
      return true;
    }
    if (ev.key === "Escape") {
      ev.preventDefault();
      close();
      return true;
    }
    return false;
  }

  return {
    open(cell, opts) {
      validationOptions = validationListFor(cell);
      validationTyped = opts.typed;
      validationActive = 0;
      if (validationOptions) update();
      else close();
    },
    refresh(typed) {
      validationTyped = typed;
      validationActive = 0;
      update();
    },
    handleKey,
    isOpen,
    isActive() {
      return validationOptions !== null;
    },
    hasOptions() {
      return validationOptions !== null;
    },
    close,
    reset() {
      validationOptions = null;
      close();
    },
    destroy() {
      close();
      menu.remove();
    },
  };
}
