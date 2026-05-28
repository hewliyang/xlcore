import { execFile } from "node:child_process";
import { mkdir, readFile, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";
import { describe, expect, test } from "vitest";

const execFileAsync = promisify(execFile);
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(packageRoot, "../..");
const cli = resolve(packageRoot, "dist/cli.js");

async function runCli(args: string[]) {
  try {
    const result = await execFileAsync(process.execPath, [cli, ...args], {
      cwd: packageRoot,
      maxBuffer: 1024 * 1024,
    });
    return { status: 0, stdout: result.stdout, stderr: result.stderr };
  } catch (error) {
    const err = error as {
      code?: number;
      stdout?: string;
      stderr?: string;
    };
    return {
      status: err.code ?? 1,
      stdout: err.stdout ?? "",
      stderr: err.stderr ?? "",
    };
  }
}

describe("built cli", () => {
  test("prints info for csv inputs using format detection", async () => {
    const csv = resolve(repoRoot, "tests/fixtures/csv/basic.csv");

    const result = await runCli([csv, "--info"]);

    expect(result.status).toBe(0);
    const info = JSON.parse(result.stdout);
    expect(info.format).toBe("csv");
    expect(info.sheets).toMatchObject([{ name: "basic", maxRow: 5, maxCol: 4 }]);
  });

  test("rejects xlsx-only sheet options for csv inputs", async () => {
    const csv = resolve(repoRoot, "tests/fixtures/csv/basic.csv");

    const result = await runCli([csv, "--sheet", "Sheet1", "--output", "/tmp/cli-nope.png"]);

    expect(result.status).toBe(1);
    expect(result.stderr).toContain("--sheet / --sheet-index only apply to .xlsx inputs");
  });

  test("renders parquet from the command line", async () => {
    const parquet = resolve(repoRoot, "tests/fixtures/parquet/primitives.parquet");
    const outputDir = resolve(tmpdir(), "xlsx-preview-cli-test");
    const output = resolve(outputDir, "primitives.png");
    await mkdir(outputDir, { recursive: true });

    const result = await runCli([parquet, "--output", output, "--max-rows", "2", "--verbose"]);

    expect(result.status).toBe(0);
    const payload = JSON.parse(result.stdout);
    expect(payload).toMatchObject({ format: "parquet", output });
    expect(result.stderr).toContain("parquet truncated");
    expect((await stat(output)).size).toBeGreaterThan(1000);
    expect((await readFile(output)).subarray(0, 8)).toEqual(
      Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    );
  });

  test("strict mode fails on csv truncation warnings", async () => {
    const csv = resolve(repoRoot, "tests/fixtures/csv/basic.csv");
    const output = resolve(tmpdir(), "xlsx-preview-cli-test", "strict.csv.png");

    const result = await runCli([csv, "--output", output, "--max-rows", "2", "--strict"]);

    expect(result.status).toBe(2);
    expect(result.stderr).toContain("--strict failed");
    expect(result.stderr).toContain("1 warning(s)");
  });
});
