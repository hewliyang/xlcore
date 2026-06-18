# Backlog: Data validation (list) dropdowns in xlsx-preview

Goal: render Excel-style dropdown arrows on cells that carry a `list`-type data
validation, in the canvas previewer. Visual (read-only) scope. Mirror the
existing AutoFilter arrow feature end-to-end.

## Architecture notes (read before starting)

- Two independent pipelines:
  - `crates/xlcore-api` = OOXML editing (DV read/write already exists in
    `crates/xlcore-api/src/data_validation.rs`). DO NOT touch for rendering.
  - `crates/xlcore-export` = builds the lightweight preview model that the
    canvas renders. ts-rs emits TS into `packages/xlsx-preview/src/schema/`.
- The model to mirror is the AutoFilter dropdown button:
  - extract:  `crates/xlcore-export/src/table_filter.rs` (`extract`, `push_range`, dedup via `HashSet<(u32,u32)>`)
  - schema:   `crates/xlcore-export/src/schema.rs` (`struct TableFilterArrow`, field `Sheet.table_filter_arrows`)
  - wire:     `crates/xlcore-export/src/lib.rs:171` (`sheet.table_filter_arrows = table_filter::extract(...)`); init `Vec::new()` in `crates/xlcore-export/src/sheet.rs` (~line 221)
  - TS schema (generated): `packages/xlsx-preview/src/schema/Sheet.ts`, `TableFilterArrow.ts`
  - draw:     `packages/xlsx-preview/src/sheetChrome.ts` (`drawFilterArrows`, helpers `filterArrowRect`, `FILTER_ARROW_BOX_W/H`); called from `packages/xlsx-preview/src/render.ts:115`
- The worksheet has `ws.data_validations` (same field the API uses, see
  `crates/xlcore-api/src/data_validation.rs:20`). Each `dataValidation` has a
  `type` (want `list`), `sequence_of_references` (sqref, may be multiple ranges
  like "B2:B6 D2"), and `formula1`.
- Render schema is intentionally minimal/obfuscated. Do NOT reuse api-schema types.
- ts-rs `schema/*.ts` are generated ("Do not edit manually") — change Rust then
  rebuild wasm (`pnpm build:wasm` from packages/xlsx-preview, or
  `pnpm --filter @hewliyang/xlsx-preview build:wasm`) which runs wasm-pack and
  regenerates the TS schema.

## Repo conventions (AGENTS.md)

- No comments/docstrings in code.
- Terse changelog, one-liners.
- Use `fd` not `find`.
- Conventional commits.
- Test e2e: build .xlsx fixture -> render via xlsx-preview CLI.

Fixture already created: `packages/xlsx-preview/tests/fixtures/data-validation-list.xlsx`
(B2:B6 list "Open,Closed,Pending"; D2 list "Yes,No").

## Items

### TODO

### Shipped

- Item 2 — TS: `drawValidationArrows` in `sheetChrome.ts` mirrors `drawFilterArrows`, iterates `sheet.validationDropdowns ?? []`, called per pane in `render.ts`. E2e render of fixture shows arrows on B2:B6 + D2.

- Item 1 — Rust: `data_validation::extract` pulls list-type DV cells into render schema (`Sheet.validation_dropdowns` / `ValidationDropdown {r,c}`); ts-rs regen + `cargo test -p xlcore-export` green. Indexing is 1-based (B2..B6 + D2 => (2,2)..(6,2),(2,4)).

- Item 3 — Interactive dropdowns (Phase 2): `ValidationDropdown` carries a `list` index into deduped `Sheet.validationLists` (inline `"a,b,c"` + same-sheet range refs resolved in Rust; respects inverted `showDropDown` suppression). `interact.ts` hit-tests arrows + `onValidationPick`; previewer opens `validationDropdownPopover` and commits the pick via `celledit`. Verified e2e in-browser (B5→Pending, D2→No write to the correct cells with distinct lists).
