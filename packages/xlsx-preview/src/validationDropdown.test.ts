// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { createValidationDropdown } from "./validationDropdown.js";
import type { Sheet } from "./types.js";

function makeSheet(): Sheet {
  return {
    validationDropdowns: [{ r: 1, c: 1, list: 0 }],
    validationLists: [["Apple", "Banana", "Cherry"]],
  } as unknown as Sheet;
}

function menuEl(): HTMLDivElement {
  return Array.from(document.body.querySelectorAll<HTMLDivElement>("div")).find(
    (el) => el.style.zIndex === "1100",
  )!;
}

afterEach(() => {
  document.body.replaceChildren();
});

describe("createValidationDropdown", () => {
  it("opens and lists options for a cell", () => {
    const input = document.createElement("input");
    document.body.append(input);
    const handle = createValidationDropdown({
      getEditInput: () => input,
      getSheet: makeSheet,
      onAccept: () => {},
    });
    handle.open({ r: 1, c: 1 }, { typed: false });
    const menu = menuEl();
    expect(menu.style.display).toBe("block");
    expect(menu.textContent).toBe("AppleBananaCherry");
    expect(handle.hasOptions()).toBe(true);
    handle.destroy();
  });

  it("filters by typed text", () => {
    const input = document.createElement("input");
    document.body.append(input);
    const handle = createValidationDropdown({
      getEditInput: () => input,
      getSheet: makeSheet,
      onAccept: () => {},
    });
    handle.open({ r: 1, c: 1 }, { typed: false });
    input.value = "an";
    handle.refresh(true);
    expect(menuEl().textContent).toBe("Banana");
    handle.destroy();
  });

  it("accepts highlighted option on Enter via onAccept", () => {
    const input = document.createElement("input");
    document.body.append(input);
    let accepted: string | null = null;
    const handle = createValidationDropdown({
      getEditInput: () => input,
      getSheet: makeSheet,
      onAccept: (value) => {
        accepted = value;
      },
    });
    handle.open({ r: 1, c: 1 }, { typed: false });
    const ev = new KeyboardEvent("keydown", { key: "Enter" });
    expect(handle.handleKey(ev)).toBe(true);
    expect(accepted).toBe("Apple");
    handle.destroy();
  });

  it("accepts a clicked option without the hover rebuild dropping it", () => {
    const input = document.createElement("input");
    document.body.append(input);
    let accepted: string | null = null;
    const handle = createValidationDropdown({
      getEditInput: () => input,
      getSheet: makeSheet,
      onAccept: (value) => {
        accepted = value;
      },
    });
    handle.open({ r: 1, c: 1 }, { typed: false });
    const banana = Array.from(menuEl().children).find((c) => c.textContent === "Banana")!;
    banana.dispatchEvent(new MouseEvent("mouseenter"));
    expect(banana.isConnected).toBe(true);
    banana.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true }));
    expect(accepted).toBe("Banana");
    handle.destroy();
  });

  it("returns null options for a cell without a dropdown", () => {
    const input = document.createElement("input");
    document.body.append(input);
    const handle = createValidationDropdown({
      getEditInput: () => input,
      getSheet: makeSheet,
      onAccept: () => {},
    });
    handle.open({ r: 5, c: 5 }, { typed: false });
    expect(handle.hasOptions()).toBe(false);
    expect(menuEl().style.display).toBe("none");
    handle.destroy();
  });
});
