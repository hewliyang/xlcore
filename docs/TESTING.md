# testing & validation

Three layers, smallest first.

## 1. Rust unit tests

```bash
cargo test --workspace
```

Today: A1-roundtrip test in `xlcore-io`, theme color resolvers
(`scrgb_byte`, `hsl_to_rgb`, prstClr table) in `xlcore-export`.
Snapshot tests on the JSON output (cargo-insta) are listed in
[open work](#open-work).

The TypeScript renderer ships unit tests too:

```bash
pnpm --filter @hewliyang/xlsx-preview test
```

Covers `numfmt.ts` (date/time/fraction/scientific/sections),
`render.ts` HLS-tint math, hidden-cell overflow, outline-gutter
placement.

## 2. End-to-end CLI smoke

```bash
cargo build --release
F=tests/fixtures/kitchensink/kitchensink.xlsx
./target/release/xlcore extract "$F" -o /tmp/k.json
./target/release/xlcore preview "$F" -o /tmp/preview.html
open /tmp/preview.html
```

What to spot-check in `k.json`:

- Every formatted cell's `styleIndex` points at a real `cellXfs` entry.
- `conditionalFormats[].rules[].colorScale.stops` has 2+ entries with colors.
- `drawings[].chart.series[].values` is **non-empty** even when the source
  workbook didn't pre-cache numCache values — the resolver in
  `xlcore-export/src/lib.rs::resolve_chart_refs` should fill them.
- `freeze`, `merges`, `cols` look right.

## 3. Visual fidelity vs HSX (final boss) + Walnut (peer)

We treat HSX (SpreadJS, the OAI artifact-tool's Office-grade renderer) as
ground truth and Walnut (OAI's reference canvas renderer) as the same-class
benchmark. Workflow is manual and each step is independently re-runnable —
no orchestration script (we tried, the walnut server hangs unattended).

### a. ours

After step 2 above, in your browser inspector or via your screenshot tool of
choice, capture `/tmp/preview.html`. If you have `browser-harness`:

```bash
uv run browser-harness <<'PY'
goto("file:///tmp/preview.html")
wait_for_load()
import time; time.sleep(0.6)
screenshot("/tmp/xlcore-ours.png")
PY
```

### b. HSX (final boss)

```bash
hsx screenshot tests/fixtures/kitchensink/kitchensink.xlsx -o /tmp/xlcore-hsx.png
```

### c. Walnut (OAI canvas)

```bash
# in one shell, leave running:
cd /Users/m1a1/Developer/oai-artifact-tools/examples/browser-workbook-preview
SAMPLE_XLSX=$PWD/tests/fixtures/kitchensink/kitchensink.xlsx node server.mjs

# in another:
uv run browser-harness <<'PY'
goto("http://127.0.0.1:4177/?sample=1")
wait_for_load()
import time; time.sleep(3)   # walnut's wasm boot is slow
screenshot("/tmp/xlcore-walnut.png")
PY

# kill the server when you're done:
lsof -i :4177 -t | xargs kill
```

### d. composite for eyeballing

Optional, but nice for diffing many turns:

```python
from PIL import Image, ImageDraw, ImageFont
imgs = [
    ("Ours",   "/tmp/xlcore-ours.png"),
    ("Walnut", "/tmp/xlcore-walnut.png"),
    ("HSX",    "/tmp/xlcore-hsx.png"),
]
W, banner = 2000, 60
loaded = [(t, Image.open(p).convert("RGB")) for t, p in imgs]
scaled = [(t, im.resize((W, int(im.height * W / im.width)))) for t, im in loaded]
out_h = banner * len(scaled) + sum(im.height for _, im in scaled)
canvas = Image.new("RGB", (W, out_h), "#fff")
draw = ImageDraw.Draw(canvas)
f = ImageFont.truetype("/System/Library/Fonts/Helvetica.ttc", 32)
y = 0
for t, im in scaled:
    draw.rectangle([0, y, W, y + banner], fill="#1f2937")
    draw.text((20, y + 14), t, fill="white", font=f)
    y += banner; canvas.paste(im, (0, y)); y += im.height
canvas.save("/tmp/xlcore-compare.png")
```

### what to eyeball when shipping a render change

- White text on the dark header bar (theme-color / FFFFFF resolution).
- CF color scale on `B2:E5` (green → red gradient, no off-by-one rows).
- "Sorted by Q4 desc" / "FILTER Q4>120" overflow into empty neighbors.
- Chart: title centered, theme accent series colors (blue/orange/gray/yellow,
  in order), axis ticks formatted as `$200` not `200`, legend at bottom.
- No stray thick black lines on freeze borders (was a 2px black bar regression
  before we made the indicator subtle).
- At app-zoom 200% (`+` twice in our preview UI) text re-shapes crisply
  rather than upscaling as bitmap.

## footguns

### `pnpm exec xlsx-preview` resolves to the **global** install, not the workspace

The workspace package `@hewliyang/xlsx-preview` exposes a `xlsx-preview`
bin. If you've ever `npm i -g @hewliyang/xlsx-preview` (or it landed via
some other global install), `pnpm exec xlsx-preview` — even from inside
`packages/xlsx-preview/` — silently resolves to
`~/.nvm/.../bin/xlsx-preview` → the globally-installed `dist/cli.js`.
That bin ships its own bundled wasm and renderer code, frozen at the
published version. Every fix you make locally — extractor, schema,
renderer — is invisible to that binary.

Symptoms: "my change doesn't show up," "this combo chart used to work
and now it's bar-only," "the dispUnits caption isn't painting even
though the unit test passes." The wasm version of the data the
global binary surfaces predates whatever schema field your renderer
is now reading, so the field is silently `undefined` and your code
path dead-ends.

Diagnose:

```bash
which xlsx-preview
#   /Users/you/.nvm/versions/node/vN/bin/xlsx-preview
readlink "$(which xlsx-preview)"
#   ../lib/node_modules/@hewliyang/xlsx-preview/dist/cli.js   ← NOT YOUR CODE
```

Fixes:

- **For local CLI testing, always invoke the workspace build directly:**

  ```bash
  node packages/xlsx-preview/dist/cli.js <args>
  ```

  Confirms you're hitting the freshly-built `dist/` and the freshly-built
  `crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm`. Pair with
  `pnpm --filter @hewliyang/xlsx-preview build` (TS only) or
  `pnpm --filter @hewliyang/xlsx-preview run build:release` (full,
  including the wasm rebuild) before each run when you've changed
  Rust extractor code.

- Don't trust `pnpm exec` / `npx` / bare `xlsx-preview` for workspace
  verification — they'll happily run the wrong binary against your
  local fixtures and produce convincingly stale screenshots.

Related: the wasm bundle (`crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm`)
is a *separate* build artifact from the TS bundle. `pnpm --filter ...
build` only rebuilds TS; you need `build:release` (or `build:wasm`) to
pick up extractor changes. The wasm's mtime is the canonical check:

```bash
stat -f "%Sm %N" crates/xlcore-wasm/pkg/xlcore_wasm_bg.wasm
```

If it predates your last extractor edit, the renderer is reading stale
data. (`hsx`'s output is unaffected since it doesn't go through our
wasm at all.)

## fixtures

Live in `tests/fixtures/` (source-controlled). See
[`tests/fixtures/README.md`](../tests/fixtures/README.md) for the table
of what each one covers and how to add new ones.

Quickstart:

```bash
# One-shot rebuild of a fixture:
bash tests/fixtures/kitchensink/build.sh
bash tests/fixtures/themes/build-custom-theme-accent.sh

# Render ours / hsx for any fixture:
F=tests/fixtures/themes/custom-theme-accent.xlsx
./target/release/xlcore preview "$F" -o /tmp/preview.html
hsx screenshot "$F" -o /tmp/xlcore-hsx.png
```

## open work

- [ ] `cargo-insta` snapshot test on the WorkbookLayout JSON for every
      committed fixture.
- ~~Pixel-diff snapshot test against `*.hsx.png`.~~ **Punted.** Tried
      and dropped — subpixel deltas across font stacks + DPI swamp
      imagehash tolerances, and the manual `hsx screenshot` eyeball
      pass in step 3 above catches regressions for less effort.
- [ ] CI guard: `cargo test … export_bindings && git diff --exit-code
      packages/xlsx-preview/src/schema/` to catch silent schema drift.
