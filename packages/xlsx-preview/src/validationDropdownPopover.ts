export interface ValidationDropdownPopoverHandle {
  open(
    options: string[],
    selected: string | null,
    anchor: { left: number; top: number; right: number; bottom: number },
  ): void;
  close(): void;
  destroy(): void;
}

export function createValidationDropdownPopover(
  onSelect: (value: string) => void,
): ValidationDropdownPopoverHandle {
  let scrim: HTMLDivElement | null = null;
  let menu: HTMLDivElement | null = null;

  function close() {
    scrim?.remove();
    menu?.remove();
    scrim = null;
    menu = null;
  }

  function open(
    options: string[],
    selected: string | null,
    anchor: { left: number; top: number; right: number; bottom: number },
  ) {
    close();
    scrim = document.createElement("div");
    scrim.style.cssText = "position:fixed;inset:0;z-index:1000;";
    scrim.onclick = close;

    menu = document.createElement("div");
    menu.style.cssText =
      "position:fixed;z-index:1001;background:#fff;border:1px solid #d4d4d8;border-radius:6px;" +
      "box-shadow:0 8px 24px rgba(15,23,42,0.18);padding:4px;min-width:120px;max-height:280px;" +
      "overflow:auto;font:13px system-ui,-apple-system,Segoe UI,Roboto,sans-serif;";
    menu.style.left = `${anchor.left}px`;
    menu.style.top = `${anchor.bottom + 4}px`;
    menu.style.minWidth = `${Math.max(120, anchor.right - anchor.left)}px`;

    if (options.length === 0) {
      const empty = document.createElement("div");
      empty.textContent = "No items";
      empty.style.cssText = "color:#9ca3af;padding:6px 8px;";
      menu.append(empty);
    }

    for (const value of options) {
      const item = document.createElement("button");
      item.type = "button";
      item.textContent = value;
      const active = value === selected;
      item.style.cssText =
        "display:block;width:100%;text-align:left;border:none;padding:5px 8px;cursor:pointer;" +
        `font:inherit;border-radius:4px;background:${active ? "#eef2ff" : "none"};`;
      item.onmouseenter = () => (item.style.background = "#f1f5f9");
      item.onmouseleave = () => (item.style.background = active ? "#eef2ff" : "none");
      item.onclick = () => {
        onSelect(value);
        close();
      };
      menu.append(item);
    }

    document.body.append(scrim, menu);
  }

  return { open, close, destroy: close };
}
