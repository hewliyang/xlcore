import type { WorkbookLayout } from "./types.js";

export interface PivotFilterContext {
  pivot: string;
  field: string;
  axis: "row" | "column";
}

export interface PivotFilterController {
  items(ctx: PivotFilterContext): string[] | Promise<string[]>;

  hiddenValues(ctx: PivotFilterContext): string[] | Promise<string[]>;

  setHidden(
    ctx: PivotFilterContext & { hidden: string[] },
  ): WorkbookLayout | void | Promise<WorkbookLayout | void>;
}

export interface PivotFilterPopoverHandle {
  open(
    ctx: PivotFilterContext,
    anchor: { left: number; top: number; right: number; bottom: number },
  ): void;
  close(): void;
  destroy(): void;
}

export function createPivotFilterPopover(
  controller: PivotFilterController,
  onChange: (layout: WorkbookLayout | void) => void,
): PivotFilterPopoverHandle {
  let scrim: HTMLDivElement | null = null;
  let menu: HTMLDivElement | null = null;

  function close() {
    scrim?.remove();
    menu?.remove();
    scrim = null;
    menu = null;
  }

  async function render(ctx: PivotFilterContext) {
    if (!menu) return;
    const items = await Promise.resolve(controller.items(ctx));
    if (!menu) return;
    const hidden = new Set(await Promise.resolve(controller.hiddenValues(ctx)));
    if (!menu) return;
    menu.replaceChildren();

    const header = document.createElement("div");
    header.textContent = ctx.field;
    header.style.cssText =
      "font-weight:600;padding:2px 6px 6px;border-bottom:1px solid #eee;margin-bottom:4px;";
    menu.append(header);

    if (items.length === 0) {
      const empty = document.createElement("div");
      empty.textContent = "No items";
      empty.style.cssText = "color:#9ca3af;padding:4px 6px;";
      menu.append(empty);
      return;
    }

    for (const value of items) {
      const label = document.createElement("label");
      label.style.cssText =
        "display:flex;align-items:center;gap:6px;padding:3px 6px;cursor:pointer;";
      const cb = document.createElement("input");
      cb.type = "checkbox";
      cb.checked = !hidden.has(value);
      cb.onchange = async () => {
        const next = new Set(await Promise.resolve(controller.hiddenValues(ctx)));
        if (cb.checked) next.delete(value);
        else next.add(value);
        const layout = await Promise.resolve(controller.setHidden({ ...ctx, hidden: [...next] }));
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
    ctx: PivotFilterContext,
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
