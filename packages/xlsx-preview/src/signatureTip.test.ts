// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { createSignatureTip } from "./signatureTip.js";

function makeInput(value: string, caret = value.length): HTMLInputElement {
  const input = document.createElement("input");
  document.body.append(input);
  input.value = value;
  input.setSelectionRange(caret, caret);
  return input;
}

function tipEl(): HTMLDivElement {
  return Array.from(document.body.querySelectorAll<HTMLDivElement>("div")).find(
    (el) => el.style.zIndex === "1099",
  )!;
}

afterEach(() => {
  document.body.replaceChildren();
});

describe("createSignatureTip", () => {
  it("shows for a known function with arg highlight", () => {
    const handle = createSignatureTip({ isBlocked: () => false });
    const input = makeInput("=SUM(1,");
    handle.update(input);
    const tip = tipEl();
    expect(tip.style.display).toBe("block");
    expect(tip.textContent).toContain("SUM");
    const highlighted = Array.from(tip.querySelectorAll("span")).find(
      (el) => el.style.fontWeight === "700",
    );
    expect(highlighted?.textContent).toBe("number2");
    handle.destroy();
  });

  it("hides when caret is outside a call", () => {
    const handle = createSignatureTip({ isBlocked: () => false });
    const input = makeInput("=1+2");
    handle.update(input);
    expect(tipEl().style.display).toBe("none");
    handle.destroy();
  });

  it("hides when isBlocked", () => {
    const handle = createSignatureTip({ isBlocked: () => true });
    const input = makeInput("=SUM(1,");
    handle.update(input);
    expect(tipEl().style.display).toBe("none");
    handle.destroy();
  });
});
