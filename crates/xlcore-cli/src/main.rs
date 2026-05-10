//! `xlcore` CLI.
//!
//! Subcommands:
//!   xlcore extract <in.xlsx> [-o layout.json]
//!     Emits the WorkbookLayout JSON.
//!
//!   xlcore preview <in.xlsx> [-o preview.html] [--renderer path/to/render.bundle.js]
//!     Emits a standalone HTML file: layout JSON inlined, renderer JS inlined.
//!     Open it in a browser; no server needed.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_help();
        std::process::exit(2);
    }

    match args[1].as_str() {
        "extract" => cmd_extract(&args[2..]),
        "preview" => cmd_preview(&args[2..]),
        "-h" | "--help" | "help" => {
            print_help();
            Ok(())
        }
        cmd => {
            eprintln!("unknown command: {cmd}\n");
            print_help();
            std::process::exit(2);
        }
    }
}

fn print_help() {
    eprintln!(
        "xlcore <command>\n\
         \n\
         Commands:\n  \
           extract <in.xlsx> [-o layout.json]            Extract WorkbookLayout JSON\n  \
           preview <in.xlsx> [-o preview.html] [--renderer R]  Bundle a standalone HTML preview"
    );
}

fn cmd_extract(args: &[String]) -> Result<()> {
    let (input, output) = parse_io_args(args, ".json")?;
    let layout = xlcore_export::extract(&input)?;
    let json = serde_json::to_string_pretty(&layout)?;
    fs::write(&output, json)?;
    println!(
        "extracted {} sheet(s) -> {}",
        layout.sheets.len(),
        output.display()
    );
    Ok(())
}

fn cmd_preview(args: &[String]) -> Result<()> {
    let mut renderer_path: Option<PathBuf> = None;
    let mut positional: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--renderer" => {
                i += 1;
                renderer_path = Some(PathBuf::from(
                    args.get(i).context("--renderer needs a path")?,
                ));
            }
            _ => positional.push(args[i].clone()),
        }
        i += 1;
    }

    let (input, output) = parse_io_args(&positional, ".html")?;
    let layout = xlcore_export::extract(&input)?;
    let layout_json = serde_json::to_string(&layout)?;

    // Find the renderer bundle.
    let renderer_path = renderer_path.or_else(default_renderer_path).context(
        "could not locate render-ts bundle; pass --renderer path/to/dist/browser.js \
             or build it first (cd render-ts && bun run build)",
    )?;
    let renderer_js = fs::read_to_string(&renderer_path)
        .with_context(|| format!("reading renderer bundle: {}", renderer_path.display()))?;

    // Gzip-compress the layout JSON and base64-encode it for embedding.
    // The browser decodes it via the native `DecompressionStream` API
    // (Chrome 80+, Safari 16.4+, Firefox 113+) — no JS dependency, and
    // typical workbook JSON shrinks 8–12× before base64 (~+33% overhead).
    let layout_gz = {
        use flate2::write::GzEncoder;
        use flate2::Compression;
        use std::io::Write;
        let mut enc = GzEncoder::new(Vec::new(), Compression::best());
        enc.write_all(layout_json.as_bytes())?;
        enc.finish()?
    };
    let layout_b64 = {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(&layout_gz)
    };

    let html = build_preview_html(&layout_b64, &renderer_js, &input);
    fs::write(&output, html)?;
    println!(
        "preview ({} sheet(s), {} B json -> {} B gz -> {} B b64, {} B renderer) -> {}",
        layout.sheets.len(),
        layout_json.len(),
        layout_gz.len(),
        layout_b64.len(),
        renderer_js.len(),
        output.display()
    );
    Ok(())
}

fn parse_io_args(args: &[String], default_ext: &str) -> Result<(PathBuf, PathBuf)> {
    let mut input: Option<PathBuf> = None;
    let mut output: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                i += 1;
                output = Some(PathBuf::from(args.get(i).context("-o needs a path")?));
            }
            s if !s.starts_with('-') => {
                if input.is_none() {
                    input = Some(PathBuf::from(s));
                } else {
                    bail!("unexpected positional argument: {s}");
                }
            }
            other => bail!("unknown flag: {other}"),
        }
        i += 1;
    }
    let input = input.context("missing <in.xlsx>")?;
    let output =
        output.unwrap_or_else(|| input.with_extension(default_ext.trim_start_matches('.')));
    Ok((input, output))
}

fn default_renderer_path() -> Option<PathBuf> {
    // Walk up from CWD looking for render-ts/dist/browser.js.
    let mut cur = std::env::current_dir().ok()?;
    for _ in 0..6 {
        let candidate = cur.join("render-ts/dist/browser.js");
        if candidate.exists() {
            return Some(candidate);
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn build_preview_html(layout_b64: &str, renderer_js: &str, source: &Path) -> String {
    let title = source
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "workbook".into());

    // Base64 alphabet contains no `<`, so no `</script>` escaping is
    // possible inside the payload — embed verbatim.
    let safe_layout = layout_b64;

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>{title} — xlcore preview</title>
<style>
  html, body {{ margin: 0; padding: 0; background: #f4f4f5; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }}
  header {{ padding: 8px 16px; background: #1f2937; color: #f9fafb; font-size: 13px; display: flex; gap: 16px; align-items: center; }}
  header b {{ color: #fff; }}
  #tabs {{ display: flex; gap: 2px; padding: 0 8px; background: #e5e7eb; }}
  #tabs button {{ background: #fff; border: 1px solid #d1d5db; border-bottom: none; padding: 6px 14px; cursor: pointer; font: inherit; font-size: 12px; }}
  #tabs button.active {{ background: #f4f4f5; font-weight: 600; }}
  #zoom {{ margin-left: auto; display: flex; gap: 4px; align-items: center; padding-right: 8px; }}
  #zoom button {{ background: #fff; border: 1px solid #d1d5db; padding: 4px 10px; cursor: pointer; font: inherit; font-size: 12px; border-radius: 4px; }}
  #zoom span {{ font-size: 12px; min-width: 42px; text-align: center; color: #374151; }}
  #namebox {{ font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 12px; padding: 4px 10px; background: #fff; border: 1px solid #d1d5db; border-radius: 4px; min-width: 70px; color: #111827; }}
  /* Stage is the scroll container. The spacer inside it gives the
     scrollbars their range; the canvas is sized to the visible viewport
     and follows the scroll position via transform/translate so we only
     ever paint what's on screen. */
  #stage {{ overflow: auto; height: calc(100vh - 70px); position: relative; background: #f4f4f5; }}
  #spacer {{ position: relative; }}
  #sheet {{ position: sticky; top: 0; left: 0; background: #fff; display: block; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
</style>
</head>
<body>
<header><b>xlcore preview</b><span>{title}</span></header>
<div id="tabs"><div id="zoom"><div id="namebox">A1</div><button id="zo">−</button><span id="zl">100%</span><button id="zi">+</button></div></div>
<div id="stage"><div id="spacer"><canvas id="sheet"></canvas></div></div>

<script id="layout" type="application/octet-stream;base64,gzip">{safe_layout}</script>
<script>
{renderer_js}
</script>
<script>
(async function () {{
  // Decode the gzip+base64-embedded layout. `DecompressionStream` is
  // native in all modern browsers; we avoid bundling pako/zlib in JS.
  async function loadLayout() {{
    const b64 = document.getElementById('layout').textContent.trim();
    const bin = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
    const ds = new DecompressionStream('gzip');
    const stream = new Blob([bin]).stream().pipeThrough(ds);
    const text = await new Response(stream).text();
    return JSON.parse(text);
  }}
  const layout = await loadLayout();
  // Inflate the per-sheet columnar blobs (b64 → Uint32/Int32/Float32
  // typed-array views, plus a row-index lookup map). After this call
  // the wire `cells` / `rowMeta` strings are dropped and the renderer
  // reads from `sheet.decodedCells` / `sheet.decodedRowMeta`.
  window.xlcoreDecodeLayout(layout);
  const canvas = document.getElementById('sheet');
  const stage = document.getElementById('stage');
  const spacer = document.getElementById('spacer');
  const tabs = document.getElementById('tabs');
  const zo = document.getElementById('zo');
  const zi = document.getElementById('zi');
  const zl = document.getElementById('zl');
  const namebox = document.getElementById('namebox');
  // Honor `xl/workbook.xml`'s `<workbookView activeTab="N"/>` so
  // workbooks that ship with a non-first sheet selected (pivot
  // demos, dashboards) open on the right tab. Falls back to 0 when
  // out-of-range or absent.
  const _at = layout.activeSheetIndex;
  let active = (typeof _at === 'number' && _at >= 0 && _at < layout.sheets.length) ? _at : 0;
  let zoom = 1;
  // Per-sheet column/row size overrides, keyed by sheet index. Each entry is
  // a {{ col, row }} pair of Maps. Resizing only the visible sheet keeps the
  // others' sizing pristine and lets users "reset" by reloading.
  const overridesBySheet = layout.sheets.map(() => ({{ col: new Map(), row: new Map() }}));
  // Active cell is also per-sheet so switching tabs preserves selection.
  const activeBySheet = layout.sheets.map(() => ({{ r: 1, c: 1 }}));
  const selectionBySheet = layout.sheets.map(() => ({{ r1: 1, c1: 1, r2: 1, c2: 1 }}));
  let interactHandle = null;

  function colLabel(n) {{
    let s = '';
    while (n > 0) {{ const r = (n - 1) % 26; s = String.fromCharCode(65 + r) + s; n = Math.floor((n - 1) / 26); }}
    return s;
  }}
  function updateNameBox() {{
    const a = activeBySheet[active];
    const s = selectionBySheet[active];
    if (!a) {{ namebox.textContent = ''; return; }}
    // Excel convention: the name box shows the anchor address even for
    // multi-cell selections. "5R x 3C" style summaries are reserved for
    // mid-drag, which we don't support yet.
    if (s && (s.r1 !== s.r2 || s.c1 !== s.c2)) {{
      const rows = s.r2 - s.r1 + 1;
      const cols = s.c2 - s.c1 + 1;
      namebox.textContent = `${{colLabel(a.c)}}${{a.r}}  (${{rows}}R×${{cols}}C)`;
    }} else {{
      namebox.textContent = colLabel(a.c) + a.r;
    }}
  }}

  // Virtual sheet extent. We extend well past the used range so the user
  // can scroll into empty space and the grid keeps painting like Excel.
  // Default-sized rows past `maxRow` cost nothing per redraw — the
  // renderer's visible-range logic only iterates the rows on screen.
  const VIRTUAL_EXTRA_COLS = 50;     // up to ~3000 px past last used col
  const VIRTUAL_EXTRA_ROWS = 1000;   // up to ~18000 px past last used row

  // Compute the virtualized sheet's logical pixel size for spacer sizing.
  // Mirrors buildGrid's accounting (col widths + 44 px row-header gutter).
  function virtualSize(sheet, ov) {{
    const dw = sheet.defaultColWidthPx || 64;
    const dh = sheet.defaultRowHeightPx || 18;
    const HEADER_W = 44, HEADER_H = 22;
    const maxCol = Math.min(16384, Math.max(sheet.maxCol + 2, sheet.maxCol + VIRTUAL_EXTRA_COLS));
    const maxRow = Math.min(1048576, Math.max(sheet.maxRow + 5, sheet.maxRow + VIRTUAL_EXTRA_ROWS));
    // Width: sum of col widths.
    let w = HEADER_W;
    const colWidths = new Map();
    for (const c of sheet.cols) {{
      for (let i = c.min; i <= c.max; i++) colWidths.set(i, c.hidden ? 0 : c.widthPx);
    }}
    if (ov && ov.col) for (const [c, v] of ov.col) colWidths.set(c, Math.max(0, v));
    for (let c = 1; c <= maxCol; c++) w += colWidths.get(c) ?? dw;
    // Height. Iterate the columnar row-meta blob — sheet.rows no
    // longer exists in the wire format; row metadata lives in typed
    // arrays decoded by xlcoreDecodeLayout.
    let h = HEADER_H;
    const rowHeights = new Map();
    window.xlcoreIterRows(sheet, (row) => {{
      if (row.hidden) rowHeights.set(row.index, 0);
      else if (row.heightPx !== undefined) rowHeights.set(row.index, row.heightPx);
    }});
    if (ov && ov.row) for (const [r, v] of ov.row) rowHeights.set(r, Math.max(0, v));
    for (let r = 1; r <= maxRow; r++) h += rowHeights.get(r) ?? dh;
    return {{ w, h }};
  }}

  // Mailbox the renderer reads on each frame to figure out which slice of
  // the sheet to paint.
  let viewport = {{ x: 0, y: 0, w: 0, h: 0 }};
  function recomputeViewport() {{
    // Logical (pre-zoom) viewport. CSS dimensions of stage ÷ zoom — we never
    // ask the canvas to be larger than the visible area.
    viewport = {{
      x: stage.scrollLeft / zoom,
      y: stage.scrollTop / zoom,
      w: stage.clientWidth / zoom,
      h: stage.clientHeight / zoom,
    }};
  }}

  function updateSpacerSize() {{
    const sheet = layout.sheets[active];
    const ov = overridesBySheet[active];
    const vs = virtualSize(sheet, ov);
    spacer.style.width = (vs.w * zoom) + 'px';
    spacer.style.height = (vs.h * zoom) + 'px';
  }}

  function draw() {{
    const sheet = layout.sheets[active];
    const ov = overridesBySheet[active];
    recomputeViewport();
    window.xlcoreRender(canvas, sheet, layout, {{
      scale: window.devicePixelRatio || 1,
      zoom,
      colOverrides: ov.col,
      rowOverrides: ov.row,
      activeCell: activeBySheet[active],
      selection: selectionBySheet[active],
      viewport,
    }});
    updateNameBox();
  }}

  // Coalesce scroll-driven redraws into one per animation frame so we
  // never queue up more work than the screen can paint. This is what makes
  // dragging the scrollbar feel buttery on big sheets.
  let rafPending = false;
  function scheduleDraw() {{
    if (rafPending) return;
    rafPending = true;
    requestAnimationFrame(() => {{ rafPending = false; draw(); }});
  }}

  stage.addEventListener('scroll', scheduleDraw, {{ passive: true }});
  // Image drawings decode asynchronously; the renderer fires this event
  // when a previously-missing image is ready, so the next paint includes it.
  window.addEventListener('xlcore-image-ready', scheduleDraw);
  const ro = new ResizeObserver(() => {{ updateSpacerSize(); scheduleDraw(); }});
  ro.observe(stage);

  function attachForActive() {{
    if (interactHandle) interactHandle.destroy();
    const ov = overridesBySheet[active];
    interactHandle = window.xlcoreAttachInteractivity(canvas, {{
      getSheet: () => layout.sheets[active],
      getLayout: () => layout,
      zoom: {{
        get: () => zoom,
        set: (v) => {{
          zoom = v;
          zl.textContent = Math.round(zoom * 100) + '%';
          updateSpacerSize();
        }},
      }},
      colOverrides: ov.col,
      rowOverrides: ov.row,
      activeCell: {{ get: () => activeBySheet[active], set: (v) => {{ activeBySheet[active] = v; }} }},
      selection: {{ get: () => selectionBySheet[active], set: (v) => {{ selectionBySheet[active] = v; }} }},
      scrollContainer: stage,
      getViewport: () => viewport,
      redraw: scheduleDraw,
    }});
  }}
  layout.sheets.forEach((s, i) => {{
    const b = document.createElement('button');
    b.textContent = s.name;
    b.onclick = () => {{ active = i; stage.scrollTop = 0; stage.scrollLeft = 0; attachForActive(); rerender(); }};
    tabs.insertBefore(b, document.getElementById('zoom'));
  }});
  function rerender() {{
    Array.from(tabs.children).forEach((b) => {{
      if (b.id === 'zoom') return;
      const i = Array.from(tabs.children).filter(x => x.id !== 'zoom').indexOf(b);
      b.classList.toggle('active', i === active);
    }});
    zl.textContent = Math.round(zoom * 100) + '%';
    updateSpacerSize();
    draw();
  }}
  zi.onclick = () => {{ zoom = Math.min(4, +(zoom + 0.25).toFixed(2)); updateSpacerSize(); rerender(); }};
  zo.onclick = () => {{ zoom = Math.max(0.25, +(zoom - 0.25).toFixed(2)); updateSpacerSize(); rerender(); }};

  // Re-render when DPR changes (browser zoom in/out, monitor switch).
  let lastDpr = window.devicePixelRatio || 1;
  function watchDpr() {{
    const m = window.matchMedia(`(resolution: ${{lastDpr}}dppx)`);
    const handler = () => {{
      if ((window.devicePixelRatio || 1) !== lastDpr) {{
        lastDpr = window.devicePixelRatio || 1;
        draw();
      }}
      watchDpr(); // matchMedia is one-shot per breakpoint; chain it
    }};
    m.addEventListener('change', handler, {{ once: true }});
  }}
  watchDpr();

  attachForActive();
  rerender();
}})();
</script>
</body>
</html>
"#
    )
}
