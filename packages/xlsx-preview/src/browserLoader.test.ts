import { afterEach, describe, expect, test } from "vitest";
import { loadWorkbookFromArrayBufferWithReport } from "./browserLoader.js";
import type { LoadReport } from "./errors.js";
import type { WorkbookLayout } from "./types.js";

const originalWorker = globalThis.Worker;

afterEach(() => {
  globalThis.Worker = originalWorker;
});

describe("browser loader", () => {
  test("does not transfer and detach the caller's ArrayBuffer", async () => {
    const input = new Uint8Array([0x50, 0x4b, 0x03, 0x04]).buffer;
    let observedPayloadBytes: ArrayBuffer | undefined;
    let observedTransfer: Transferable[] | undefined;

    class FakeWorker {
      onmessage: ((event: MessageEvent) => void) | null = null;
      onerror: ((event: ErrorEvent) => void) | null = null;

      postMessage(message: unknown, transfer?: Transferable[]): void {
        const payload = message as { bytes: ArrayBuffer };
        observedPayloadBytes = payload.bytes;
        observedTransfer = transfer;
        queueMicrotask(() => {
          this.onmessage?.({
            data: {
              type: "loaded",
              layout: { sheets: [] } satisfies Partial<WorkbookLayout>,
              report: { fixes: [], warnings: [] } satisfies LoadReport,
            },
          } as MessageEvent);
        });
      }

      terminate(): void {}
    }

    globalThis.Worker = FakeWorker as unknown as typeof Worker;

    await loadWorkbookFromArrayBufferWithReport(input, { format: "xlsx" });

    expect(observedPayloadBytes).toBeInstanceOf(ArrayBuffer);
    expect(observedPayloadBytes).not.toBe(input);
    expect(observedTransfer).toEqual([observedPayloadBytes]);
    expect(input.byteLength).toBe(4);
  });
});
