// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { createAutocompletePopover } from "./autocompletePopover.js";

function makeInput(value: string): HTMLInputElement {
  const input = document.createElement("input");
  document.body.append(input);
  input.value = value;
  input.setSelectionRange(value.length, value.length);
  return input;
}

const names = ["SUM", "SUMIF", "AVERAGE"];

afterEach(() => {
  document.body.replaceChildren();
});

describe("createAutocompletePopover", () => {
  it("opens on a function prefix", () => {
    const handle = createAutocompletePopover({
      getFunctionNames: () => names,
      onAccept: () => {},
    });
    const input = makeInput("=SU");
    handle.update(input);
    expect(handle.isOpen()).toBe(true);
    handle.destroy();
  });

  it("does not open without a matching prefix", () => {
    const handle = createAutocompletePopover({
      getFunctionNames: () => names,
      onAccept: () => {},
    });
    const input = makeInput("=ZZ");
    handle.update(input);
    expect(handle.isOpen()).toBe(false);
    handle.destroy();
  });

  it("accepts via arrow + enter", () => {
    const onAccept = vi.fn();
    const handle = createAutocompletePopover({
      getFunctionNames: () => names,
      onAccept,
    });
    const input = makeInput("=SU");
    handle.update(input);
    handle.handleKey(new KeyboardEvent("keydown", { key: "ArrowDown" }));
    handle.handleKey(new KeyboardEvent("keydown", { key: "Enter" }));
    expect(input.value).toBe("=SUMIF(");
    expect(onAccept).toHaveBeenCalledWith(input);
    expect(handle.isOpen()).toBe(false);
    handle.destroy();
  });

  it("closes", () => {
    const handle = createAutocompletePopover({
      getFunctionNames: () => names,
      onAccept: () => {},
    });
    const input = makeInput("=SU");
    handle.update(input);
    expect(handle.isOpen()).toBe(true);
    handle.close();
    expect(handle.isOpen()).toBe(false);
    handle.destroy();
  });
});
