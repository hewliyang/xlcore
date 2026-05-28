# parquet fixtures

Tiny parquet files exercising the `xlcore-tabular` parquet adapter. Each
fixture is reproducibly built from
[`crates/xlcore-tabular/examples/build_parquet_fixtures.rs`](../../../crates/xlcore-tabular/examples/build_parquet_fixtures.rs).
Rebuild with:

```bash
cargo run -p xlcore-tabular --features parquet --example build_parquet_fixtures
```

| File | What it covers |
|---|---|
| `primitives.parquet` | `Utf8 / Int64 / Float64 / Boolean`, including a `null` in the float column. The smoke check — if this regresses, every other parquet path is broken. |
| `temporal.parquet` | `Date32 / Timestamp(ms) / Time64(us)`, including a `null` timestamp. Catches breakage in `format_timestamp` / `format_time64` and the `chrono`-based formatters. |
| `nested.parquet` | `List<Int32> / Struct{name, age} / Map<Utf8, Int32>`. Catches breakage in the `arrow_cast::display::ArrayFormatter` fallback path (this is the layer everything non-primitive flows through). |

Adding a new fixture: extend `build_parquet_fixtures.rs`, re-run the
builder, add a row in `packages/xlsx-preview/src/tabular.test.ts`, then
commit both the regenerated `.parquet` and the test change.
