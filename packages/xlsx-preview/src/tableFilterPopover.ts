import type { WorkbookLayout } from "./types.js";

export interface TableFilterContext {
  field: string;
  columnOffset: number;
  rangeRef: string;
}

export interface TableFilterController {
  items(ctx: TableFilterContext): string[] | Promise<string[]>;

  activeValues(ctx: TableFilterContext): string[] | Promise<string[]>;

  setFilter(
    ctx: TableFilterContext & { values: string[] },
  ): WorkbookLayout | void | Promise<WorkbookLayout | void>;

  setSort(
    ctx: TableFilterContext & { descending: boolean | null },
  ): WorkbookLayout | void | Promise<WorkbookLayout | void>;
}

export interface TableFilterPopoverHandle {
  open(
    ctx: TableFilterContext,
    anchor: { left: number; top: number; right: number; bottom: number },
  ): void;
  close(): void;
  destroy(): void;
}

export function createTableFilterPopover(
  controller: TableFilterController,
  onChange: (layout: WorkbookLayout | void) => void,
): TableFilterPopoverHandle {
  let scrim: HTMLDivElement | null = null;
  let menu: HTMLDivElement | null = null;

  function close() {
    scrim?.remove();
    menu?.remove();
    scrim = null;
    menu = null;
  }

  async function render(ctx: TableFilterContext) {
    if (!menu) return;
    const items = await Promise.resolve(controller.items(ctx));
    if (!menu) return;
    const active = await Promise.resolve(controller.activeValues(ctx));
    if (!menu) return;
    const kept = new Set(active.length === 0 ? items : active);
    menu.replaceChildren();

    const header = document.createElement("div");
    header.textContent = ctx.field;
    header.style.cssText =
      "font-weight:600;padding:2px 6px 6px;border-bottom:1px solid #eee;margin-bottom:4px;";
    menu.append(header);

    const sortAsc = document.createElement("button");
    sortAsc.type = "button";
    sortAsc.textContent = "Sort A→Z";
    const sortDesc = document.createElement("button");
    sortDesc.type = "button";
    sortDesc.textContent = "Sort Z→A";
    for (const btn of [sortAsc, sortDesc]) {
      btn.style.cssText =
        "display:block;width:100%;text-align:left;background:none;border:none;padding:4px 6px;" +
        "cursor:pointer;font:inherit;border-radius:4px;";
      btn.onmouseenter = () => (btn.style.background = "#f1f5f9");
      btn.onmouseleave = () => (btn.style.background = "none");
    }
    sortAsc.onclick = async () => {
      onChange(await Promise.resolve(controller.setSort({ ...ctx, descending: false })));
      close();
    };
    sortDesc.onclick = async () => {
      onChange(await Promise.resolve(controller.setSort({ ...ctx, descending: true })));
      close();
    };
    menu.append(sortAsc, sortDesc);

    const clear = document.createElement("a");
    clear.textContent = "Clear filter";
    clear.href = "#";
    clear.style.cssText =
      "display:block;padding:4px 6px;margin:2px 0;color:#2563eb;cursor:pointer;text-decoration:none;";
    clear.onclick = async (e) => {
      e.preventDefault();
      onChange(await Promise.resolve(controller.setFilter({ ...ctx, values: [] })));
      void render(ctx);
    };
    menu.append(clear);

    const sep = document.createElement("div");
    sep.style.cssText = "border-top:1px solid #eee;margin:4px 0;";
    menu.append(sep);

    if (items.length === 0) {
      const empty = document.createElement("div");
      empty.textContent = "No items";
      empty.style.cssText = "color:#9ca3af;padding:4px 6px;";
      menu.append(empty);
      return;
    }

    const allChecked = items.every((value) => kept.has(value));
    const selectAllLabel = document.createElement("label");
    selectAllLabel.style.cssText =
      "display:flex;align-items:center;gap:6px;padding:3px 6px;cursor:pointer;font-weight:600;";
    const selectAllCb = document.createElement("input");
    selectAllCb.type = "checkbox";
    selectAllCb.checked = allChecked;
    selectAllCb.onchange = async () => {
      const values = selectAllCb.checked ? [] : ["\0"];
      const layout = await Promise.resolve(controller.setFilter({ ...ctx, values }));
      onChange(layout);
      void render(ctx);
    };
    const selectAllSpan = document.createElement("span");
    selectAllSpan.textContent = allChecked ? "Unselect all" : "Select all";
    selectAllLabel.append(selectAllCb, selectAllSpan);
    menu.append(selectAllLabel);

    const selSep = document.createElement("div");
    selSep.style.cssText = "border-top:1px solid #eee;margin:4px 0;";
    menu.append(selSep);

    for (const value of items) {
      const label = document.createElement("label");
      label.style.cssText =
        "display:flex;align-items:center;gap:6px;padding:3px 6px;cursor:pointer;";
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = kept.has(value);
      cb.onchange = async () => {
        const next = new Set(kept);
        if (cb.checked) next.add(value);
        else next.delete(value);
        const values = next.size === items.length ? [] : [...next];
        const layout = await Promise.resolve(controller.setFilter({ ...ctx, values }));
        onChange(layout);
        void render(ctx);
      };
      const span = document.createElement("span");
      span.textContent = value;
      label.append(cb, span);
      menu.append(label);
    }
  }

  function open(
    ctx: TableFilterContext,
    anchor: { left: number; top: number; right: number; bottom: number },
  ) {
    close();
    scrim = document.createElement("div");
    scrim.style.cssText = "position:fixed;inset:0;z-index:1000;";
    scrim.onclick = close;

    menu = document.createElement("div");
    menu.style.cssText =
      "position:fixed;z-index:1001;background:#fff;border:1px solid #d4d4d8;border-radius:6px;" +
      "box-shadow:0 8px 24px rgba(15,23,42,0.18);padding:6px;min-width:160px;max-height:280px;" +
      "overflow:auto;font:13px system-ui,-apple-system,Segoe UI,Roboto,sans-serif;";
    menu.style.left = `${anchor.left}px`;
    menu.style.top = `${anchor.bottom + 4}px`;

    document.body.append(scrim, menu);
    void render(ctx);
  }

  return {
    open,
    close,
    destroy: close,
  };
}
