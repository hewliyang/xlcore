# TS API conventions

The per-language fluent wrapper (Layer 3) is idiom-only — no semantics. To keep it
consistent and portable, every collection/accessor follows the rules below. When you
add a feature, audit the new method against this list before merging.

## Method verbs

Canonical CRUD verbs, fixed semantics:

| Verb | Returns | Meaning |
| --- | --- | --- |
| `list()` | `T[]` | All elements in scope. |
| `get(...)` | `T \| null` | One element (by id/ref) or the singleton settings object. `null` when absent. |
| `set(...)` | `T` | Upsert one element keyed by its identifier (replace if present, else create). |
| `add(...)` | `T` | Create a new element where duplicates are allowed (no upsert key), e.g. a merge or a note thread. |
| `remove(...)` | `T \| T[] \| null` | Delete by identifier. Returns what was removed (or `null`). |
| `clear(...)` | `T[]` | Bulk-delete/reset everything matching a scope (a ref, or "reset to default"). |

`set` vs `add`: use `set` when the element has a natural upsert key (a cell ref, a
name, an id) so calling twice replaces; use `add` when each call appends a distinct
element (merges, threaded notes).

### Domain verbs (allowed exceptions)

A few operations have no CRUD equivalent and keep a named verb. These are the *only*
sanctioned departures:

- `ChartCollection.update(id, patch)` / `PivotCollection.update(id, patch)` —
  in-place partial mutation that preserves unmodeled XML (distinct from `set`, which
  rebuilds the element).
- `PivotCollection.preview(patch)` — compute the grid without persisting.
- `ThreadedNotesCollection.reply(parentId, patch)` — append a reply to an existing
  thread (an `add` that targets a parent).
- `AutoFilterAccessor.setColumn` / `setColumnValues` / `setColumnTop10` /
  `setColumnCustom` / `removeColumn` — the per-column sub-API of a single autofilter.
  `set*` upsert a column criterion by `columnOffset`; the `Values`/`Top10`/`Custom`
  variants are typed sugar over `setColumn`.

Anything else must use a canonical verb.

## Class names

Two suffixes, picked by cardinality; no other suffix (`Api` is banned).

- **`<Concept>Collection`** — sheet-scoped collection of many elements
  (`MergeCollection`, `ChartCollection`, …).
- **`Workbook<Concept>`** — workbook-scoped collection
  (`WorkbookTables`, `WorkbookCharts`, `WorkbookDefinedNames`, …).
- **`<Concept>Accessor`** — a single element / settings object reached by get/set
  (`SheetFreezeAccessor`, `SheetPropertiesAccessor`, `WorkbookPropertiesAccessor`,
  `AutoFilterAccessor`, …). The `Accessor` suffix also avoids colliding with the
  like-named DTO (`SheetProperties`, `WorkbookProperties`, `CalcProperties`,
  `SheetPageSetup`).

The property the wrapper is exposed under (`ws.freeze`, `wb.properties`) is named for
the concept without the suffix.
