# Missing functions backlog

Reference = ECMA-376 §18.17.7 ∪ EPPlus registry (496) minus ironcalc impl (346) =
156 missing. Full list: `reports/missing_functions.txt`. Triage rationale:
`reports/missing_functions_triage.md`. Regenerate the diff: `python3
scripts/missing_functions.py`.

Hard constraint: **no spill engine.** A formula returning `CalcResult::Array` is
written to the grid as `#N/IMPL "Arrays not supported yet"` (`model.rs`). Implicit
intersection is stubbed to `#N/IMPL`. Tiers 3–4 are blocked on that.

## How to add a scalar function

Edit `crates/ironcalc-base/`:

1. `src/language/mod.rs` — add `pub <field>: String,` to `struct Functions`.
2. `src/language/language.json` — add `"<field>": "NAME"` to all 4 locales
   (en/de/fr/es; use the localized name where known, else the English name).
3. `cargo run -p ironcalc_base --example regen_language` — rebuilds `language.bin`.
4. `src/functions/mod.rs` — six edits:
   - `enum Function` variant.
   - `impl_function_lookup!` row: `<field> => <Variant>,`.
   - `to_localized_name` arm: `Function::<Variant> => functions.<field>.clone(),`.
   - `into_iter()` array: add `Function::<Variant>,` **and bump the count** in
     `IntoIter<Function, N>`.
   - `to_xlsx_string` arm: plain `"NAME"` for legacy fns; `"_xlfn.NAME"` for
     functions introduced after Excel 2007 (the `.`-dotted stat names, UNICHAR,
     NUMBERVALUE, etc.).
   - `evaluate_function` arm: `Function::<Variant> => self.fn_<name>(args, cell),`.
5. Implement `fn_<name>` in the relevant module (`functions/text.rs`,
   `functions/statistical/*`, …). Legacy aliases usually just call the existing
   `.`-form fn (watch arg-count diffs, e.g. NORMSDIST injects `cumulative=TRUE`).
6. Add a test file `src/test/test_fn_<name>.rs`, register it in `src/test/mod.rs`,
   following `test_fn_exact.rs` (`new_empty_model`, `_set`, `evaluate`, `_get_text`).

Verify: `cargo test -p ironcalc_base`. Cross-check a few values against Excel.

## Tier 1 — trivial (aliases / simple scalars)

Each item below = one agent batch.

- [ ] **dispersion legacy** — STDEV, STDEVP, VAR, VARP, RANK (→ STDEV.S, STDEV.P,
  VAR.S, VAR.P, RANK.EQ).
- [ ] **TDIST + MODE family** — TDIST (route to T.DIST.2T/T.DIST.RT by `tails`),
  MODE, MODE.SNGL (single most-frequent; new impl, MODE = MODE.SNGL).
- [ ] **simple text** — CHAR, CODE, CLEAN, PROPER, REPLACE, FIXED, DOLLAR.
- [ ] **newer text** — UNICHAR, NUMBERVALUE.
- [ ] **byte variants** — LENB, LEFTB, RIGHTB, MIDB, FINDB, SEARCHB, REPLACEB
  (= non-B versions outside DBCS).
- [ ] **ASC / JIS** — identity transforms outside CJK locales.
- [ ] **ceiling/dollar aliases** — ECMA.CEILING (→ ISO.CEILING), USDOLLAR (→ DOLLAR).
- [ ] **ref/math simple** — ADDRESS (build A1/R1C1 ref string), AREAS (count areas),
  MULTINOMIAL, SERIESSUM, FVSCHEDULE, PERMUTATIONA, HYPERLINK (return friendly text).

## Tier 2 — medium scalar (not started)
Quantiles (PERCENTILE/QUARTILE/PERCENTRANK families, PROB, TRIMMEAN, FORECAST),
bond financials (PRICE/YIELD/DURATION/COUP*/ODD*/ACCRINT…), MDETERM, XMATCH,
AGGREGATE, BAHTTEXT, VALUETOTEXT. See triage report.

## Tier 3 — blocked on spill engine
FILTER, SORT, UNIQUE, SEQUENCE, TRANSPOSE, MMULT, FREQUENCY, … (full list in triage).

## Tier 4 — blocked on lambda support
LAMBDA, LET, MAP, REDUCE, SCAN, BYROW, BYCOL, MAKEARRAY, ISOMITTED.

## Tier 5 — out of scope
CUBE*, GETPIVOTDATA, GROUPBY, PERCENTOF, RTD, IMAGE, PHONETIC.

## Shipped
- normal-dist legacy — NORMDIST, NORMINV, NORMSDIST, NORMSINV.
- chi/F/t legacy — CHIDIST, CHIINV, CHITEST, FDIST, FINV, FTEST, TINV, TTEST.
- other stat legacy — BETADIST, BETAINV, BINOMDIST, CRITBINOM, CONFIDENCE, COVAR,
  EXPONDIST, GAMMADIST, GAMMAINV, HYPGEOMDIST, LOGINV, LOGNORMDIST, NEGBINOMDIST,
  POISSON, WEIBULL, ZTEST.
