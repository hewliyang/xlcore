# csv fixtures

Tiny hand-written CSV files that exercise the `xlcore-tabular` CSV adapter.
Each file is small enough to inspect by eye; the matching snapshot test
lives in `packages/xlsx-preview/src/tabular.test.ts`.

| File | What it covers |
|---|---|
| `basic.csv` | Mixed types (strings / ints / floats / bools), header row, a couple of empty cells. The default sanity check. |
| `semicolon.csv` | European-style `;`-delimited file. Catches regressions in the delimiter sniff. |
| `leading-zeros.csv` | ZIP codes / phone-number-shaped tokens that must stay strings (Excel-import gotcha — we don't want `00123` silently becoming `123`). |
| `ragged.csv` | Rows with fewer/more columns than the header; verifies `max_col` extends to the widest row. |

Adding a new fixture: drop the `.csv` in here, add a row to the table in
`packages/xlsx-preview/src/tabular.test.ts`, and re-run
`pnpm --filter @hewliyang/xlsx-preview test`.
