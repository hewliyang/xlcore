import { autocompleteState, type AutocompleteState } from "./formulaAutocomplete.js";

export interface AutocompletePopoverOptions {
  getFunctionNames: () => string[];
  onAccept: (input: HTMLInputElement) => void;
}

export interface AutocompletePopoverHandle {
  update(input: HTMLInputElement): void;
  handleKey(ev: KeyboardEvent): boolean;
  isOpen(): boolean;
  close(): void;
  scheduleClose(): void;
  destroy(): void;
}

export function createAutocompletePopover(
  options: AutocompletePopoverOptions,
): AutocompletePopoverHandle {
  const menu = document.createElement("div");
  menu.style.cssText =
    "position:fixed;z-index:1100;display:none;background:#fff;border:1px solid #d4d4d8;border-radius:6px;" +
    "box-shadow:0 8px 24px rgba(15,23,42,0.18);padding:4px;min-width:140px;max-height:240px;overflow:auto;" +
    "font:12px ui-monospace,SFMono-Regular,Menlo,monospace;";
  document.body.append(menu);

  let autocompleteFor: HTMLInputElement | null = null;
  let autocompleteData: AutocompleteState | null = null;
  let autocompleteActive = 0;
  let blurTimer: ReturnType<typeof setTimeout> | null = null;

  function isOpen(): boolean {
    return autocompleteData !== null && menu.style.display !== "none";
  }

  function close(): void {
    autocompleteFor = null;
    autocompleteData = null;
    autocompleteActive = 0;
    menu.style.display = "none";
  }

  function scheduleClose(): void {
    if (blurTimer !== null) clearTimeout(blurTimer);
    blurTimer = setTimeout(() => close(), 120);
  }

  function render(input: HTMLInputElement): void {
    const state = autocompleteData;
    if (!state) return;
    menu.replaceChildren();
    state.matches.forEach((name, i) => {
      const item = document.createElement("div");
      item.textContent = name;
      const active = i === autocompleteActive;
      item.style.cssText = `padding:3px 8px;cursor:pointer;border-radius:4px;${
        active ? "background:#2563eb;color:#fff;" : "color:#111827;"
      }`;
      item.onmousedown = (ev) => {
        ev.preventDefault();
        accept(i);
      };
      item.onmouseenter = () => {
        autocompleteActive = i;
        render(input);
      };
      menu.append(item);
    });
    const rect = input.getBoundingClientRect();
    menu.style.left = `${rect.left}px`;
    menu.style.top = `${rect.bottom + 2}px`;
    menu.style.display = "block";
  }

  function update(input: HTMLInputElement): void {
    const caret = input.selectionStart;
    if (caret === null) return close();
    const state = autocompleteState(input.value, caret, options.getFunctionNames());
    if (!state) return close();
    autocompleteFor = input;
    autocompleteData = state;
    if (autocompleteActive >= state.matches.length) autocompleteActive = 0;
    render(input);
  }

  function accept(index: number): void {
    const state = autocompleteData;
    const input = autocompleteFor;
    if (!state || !input) return;
    const name = state.matches[index];
    if (!name) return;
    const insert = `${name}(`;
    const value = input.value.slice(0, state.start) + insert + input.value.slice(state.end);
    input.value = value;
    const caret = state.start + insert.length;
    input.setSelectionRange(caret, caret);
    close();
    options.onAccept(input);
  }

  function handleKey(ev: KeyboardEvent): boolean {
    if (!isOpen()) return false;
    const state = autocompleteData!;
    const input = autocompleteFor;
    if (ev.key === "ArrowDown") {
      ev.preventDefault();
      autocompleteActive = (autocompleteActive + 1) % state.matches.length;
      if (input) render(input);
      return true;
    }
    if (ev.key === "ArrowUp") {
      ev.preventDefault();
      autocompleteActive = (autocompleteActive - 1 + state.matches.length) % state.matches.length;
      if (input) render(input);
      return true;
    }
    if (ev.key === "Enter" || ev.key === "Tab") {
      ev.preventDefault();
      accept(autocompleteActive);
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
    update,
    handleKey,
    isOpen,
    close,
    scheduleClose,
    destroy() {
      close();
      menu.remove();
    },
  };
}
