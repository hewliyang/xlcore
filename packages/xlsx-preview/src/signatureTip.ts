import { lookupSignature, signatureAt, type FunctionSignature } from "./formulaSignature.js";

export interface SignatureTipOptions {
  isBlocked: () => boolean;
}

export interface SignatureTipHandle {
  update(input: HTMLInputElement): void;
  hide(): void;
  scheduleClose(): void;
  destroy(): void;
}

export function createSignatureTip(options: SignatureTipOptions): SignatureTipHandle {
  const tip = document.createElement("div");
  tip.style.cssText =
    "position:fixed;z-index:1099;display:none;background:#fff;border:1px solid #d4d4d8;border-radius:4px;" +
    "box-shadow:0 4px 12px rgba(15,23,42,0.12);padding:6px 10px;max-width:480px;" +
    "font:12px ui-monospace,SFMono-Regular,Menlo,monospace;color:#111827;";
  document.body.append(tip);

  let blurTimer: ReturnType<typeof setTimeout> | null = null;

  function hide(): void {
    tip.style.display = "none";
    tip.replaceChildren();
  }

  function scheduleClose(): void {
    if (blurTimer !== null) clearTimeout(blurTimer);
    blurTimer = setTimeout(() => hide(), 120);
  }

  function render(input: HTMLInputElement, sig: FunctionSignature, argIndex: number): void {
    tip.replaceChildren();

    const sigLine = document.createElement("div");
    sigLine.style.cssText = "margin:0 0 6px 0;line-height:1.4;";

    const nameSpan = document.createElement("span");
    nameSpan.textContent = sig.name;
    nameSpan.style.fontWeight = "600";
    sigLine.append(nameSpan);

    const openParen = document.createElement("span");
    openParen.textContent = "(";
    sigLine.append(openParen);

    const highlightIndex =
      sig.args.length === 0
        ? -1
        : argIndex >= sig.args.length - 1 && sig.args[sig.args.length - 1] === "..."
          ? sig.args.length - 1
          : Math.min(argIndex, sig.args.length - 1);

    sig.args.forEach((arg, i) => {
      if (i > 0) {
        const comma = document.createElement("span");
        comma.textContent = ", ";
        sigLine.append(comma);
      }
      const argSpan = document.createElement("span");
      argSpan.textContent = arg;
      if (i === highlightIndex) {
        argSpan.style.cssText =
          "font-weight:700;background:#fef9c3;padding:0 2px;border-radius:2px;";
      }
      sigLine.append(argSpan);
    });

    const closeParen = document.createElement("span");
    closeParen.textContent = ")";
    sigLine.append(closeParen);
    tip.append(sigLine);

    const summaryLabel = document.createElement("div");
    summaryLabel.textContent = "Summary";
    summaryLabel.style.cssText =
      "font:600 11px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#6b7280;margin:0 0 2px 0;";
    tip.append(summaryLabel);

    const summaryText = document.createElement("div");
    summaryText.textContent = sig.summary;
    summaryText.style.cssText =
      "font:12px -apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;color:#374151;line-height:1.35;";
    tip.append(summaryText);

    const rect = input.getBoundingClientRect();
    tip.style.left = `${rect.left}px`;
    tip.style.top = `${rect.bottom + 2}px`;
    tip.style.display = "block";
  }

  function update(input: HTMLInputElement): void {
    if (options.isBlocked()) return hide();
    const caret = input.selectionStart;
    if (caret === null) return hide();
    const ctx = signatureAt(input.value, caret);
    if (!ctx) return hide();
    const sig = lookupSignature(ctx.name);
    if (!sig) return hide();
    render(input, sig, ctx.argIndex);
  }

  return {
    update,
    hide,
    scheduleClose,
    destroy() {
      hide();
      tip.remove();
    },
  };
}
