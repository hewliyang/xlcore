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

Tier 1 complete.

## Tier 2 — medium scalar
Tier 2 complete.

Verified against real Excel (macOS) + SpreadJS: all Tier 2 fns match Excel exactly.
Note: DISC basis-1 multi-year matches real Excel (0.000686384) and SpreadJS's DISC
differs there — SpreadJS is the outlier, xlcore is correct.

## Tier 3 — blocked on spill engine
27 fns. All return `CalcResult::Array`, which model.rs (~L684) rewrites to `#N/IMPL
"Arrays not supported yet"`. A function can only be written to the grid today if its
result is a scalar (String/Number/Boolean). Triaged by that test:

### 3a — ship now (genuine scalar result, no spill)
- **ARRAYTOTEXT** — ARRAYTOTEXT(array,[format]) always returns one string. Format 0
  concise = values joined `", "`; format 1 strict = brace-wrapped, rows joined `;`,
  cols joined `,`, strings quoted, errors as text. Sibling of the shipped
  VALUETOTEXT (text.rs:1524); same range-walk, no spill. Do this independently of
  the spill work.

### 3b — degenerate scalar only (skip — false coverage)
Technically have a 1×1 case but are pointless/misleading without spill; do NOT ship
partials: MMULT (row·col dot product → scalar), MINVERSE/MUNIT (1×1), TRANSPOSE
(1×1), SEQUENCE/RANDARRAY/TAKE/DROP (1×1), MODE.MULT (collapses to existing
MODE.SNGL), TEXTSPLIT (no-delimiter → whole text). Wait for the engine.

### 3c — strictly array, hard-blocked
FILTER, SORT, SORTBY, UNIQUE, EXPAND, HSTACK, VSTACK, TOCOL, TOROW, CHOOSECOLS,
CHOOSEROWS, FREQUENCY, LINEST, LOGEST, TREND, GROWTH. No useful scalar form.

### Spill-engine unlock (round-trip required)
Needed for 3b+3c and array Tier 4. Round-trip (xlsx read+write) is in scope, so
spill metadata must thread through every layer, not just the calc engine. Staged
backlog — work top to bottom, one item per agent:

- [x] **S0 — ARRAYTOTEXT** (decoupled, no spill). Always returns one string;
  sibling of shipped VALUETOTEXT (text.rs:1524). Format 0 concise = values joined
  `", "`; format 1 strict = brace-wrapped, rows `;`, cols `,`, strings quoted,
  errors as text. Ship like the Tier 2 fns.
- [x] **S1 — spill metadata in ironcalc `Cell`.** Add a `CellFormulaArray`
  variant (anchor flag + spill `ref` + cached value) at the `// TODO: Array
  formulas` seam (types.rs:163). Wire it inert through every exhaustive
  `match cell` site (behaves like a cached formula cell), keep bitcode
  serialization + all tests green. No behavior. (xlcore-types/CellInfo surfacing
  folded into S2 where it's exercised.)
- [x] **S2a — in-memory spill (calc engine only).** Pilot fn: TRANSPOSE. See
  Shipped. Numeric arrays fully cached; non-numeric neighbours spill but the
  anchor caches numeric top-left only. No xlsx yet (S2b).
- [x] **S2b — xlsx round-trip persistence (CSE-array layer).** See Shipped.
  Reads `<f t="array" ref=...>`, excludes spilled targets from harvest, writes
  cached `<v>` per spilled cell. metadata.xml / `cm` / dynamicArrayProperties
  fidelity + `CellInfo` spill-range surfacing deferred to a follow-up.
- [x] **S2c — generalize anchor cached value.** `CellFormulaArray.v` now holds a
  typed `ArrayCachedValue` (Number/String/Boolean/Error) instead of `f64`, so a
  non-numeric top-left no longer flattens to `0`. `get_cell_value`,
  `Cell::value`/`get_type`, and `spill_array` cache/render the real top-left
  ArrayNode. Prerequisite for text-producing array fns; numeric path unchanged.
- [ ] **S3 — reference/intersection polish.** `A1#` spill operator;
  `@` implicit intersection (currently `#N/IMPL`, model.rs:~648).
- [ ] **S4 — roll out Tier 3b/3c + array Tier 4** one fn per agent once S2c lands.
  High value first: ~~SEQUENCE~~, SORT, UNIQUE, FILTER, then HSTACK/VSTACK, TAKE/DROP,
  CHOOSECOLS/ROWS, TOCOL/TOROW, EXPAND, SORTBY, MMULT, MINVERSE, MUNIT, RANDARRAY,
  FREQUENCY, MODE.MULT, SEQUENCE, TEXTSPLIT, LINEST/LOGEST/TREND/GROWTH.

Write path: `CalcResult::Array` at model.rs:684. xlsx persistence: dynamic arrays
live on the anchor cell as `<f t="array" ref=...>` + `cm` → `xl/metadata.xml`
`<dynamicArrayProperties fDynamic="1"/>`; spilled cells are plain cached `<v>`;
`#`/`@` serialize as `_xlfn.ANCHORARRAY`/`_xlfn.SINGLE`.

## Tier 4 — blocked on lambda support
LAMBDA, LET, MAP, REDUCE, SCAN, BYROW, BYCOL, MAKEARRAY, ISOMITTED.

## Tier 5 — out of scope
CUBE*, GETPIVOTDATA, GROUPBY, PERCENTOF, RTD, IMAGE, PHONETIC.

## Shipped
- sequence (S4) — SEQUENCE(rows,[columns],[start],[step]): row-major rows x columns
  array of numbers starting at start (default 1) stepping by step (default 1),
  columns default 1; dims truncated to int, rows<1 or columns<1 => #VALUE!;
  returns CalcResult::Array(Number) so it spills via model.rs spill_array and
  round-trips through xlsx; 1-4 args else #ERROR!; _xlfn.
- spill round-trip (S2b, CSE-array) — xlcore-bridge recalc now persists
  dynamic-array spills as legacy `<f t="array" ref=...>` + cached `<v>` on the
  anchor AND every spilled cell. Write (write_cached_formula_values): after
  recalc, for each array anchor parse the `ref` range and write a cached `<v>`
  (creating sorted `<row>`/`<c>` if absent) for every non-anchor cell via
  engine.cell_value. Read (harvest_sheet_cells): collect array ref ranges and
  skip emitting literal HarvestedCells for non-anchor cells inside a range so
  pre-cached spill outputs from real Excel saves don't occupy targets (fixes a
  false #SPILL! on reload). Fixtures tests/fixtures/spill/transpose.xlsx +
  transpose_precached.xlsx round-trip to A4=1,B4=4,A5=2,B5=5,A6=3,B6=6,A8=50.
  Deferred: xl/metadata.xml / `cm` / dynamicArrayProperties dynamic-array
  fidelity and CellInfo spill-range surfacing.
- transpose + in-memory spill (S2a) — TRANSPOSE(array): reads the single
  range/array arg row-major and returns CalcResult::Array transposed (rows<->cols);
  non-numeric cells pass through as their ArrayNode. Spill mechanism: model.rs
  set_cell_value's `CalcResult::Array` arm now spills instead of #N/IMPL — anchor
  (formula cell) becomes `Cell::CellFormulaArray { f, v: numeric top-left, s, range
  "A1:C3" }`, rectangular block spills into neighbour `sheet_data` cells
  (NumberCell/BooleanCell/ErrorCell/SharedString), `#SPILL!` (CellFormulaError,
  Error::SPILL) if any non-anchor target is non-empty. Spilled neighbours are
  readable by other formulas; `Model::spill_owners` maps spilled cell->anchor,
  rebuilt each `evaluate()` to clear stale spills (idempotent, no false #SPILL!)
  and to force anchor eval when a spilled cell is read out of order. Scope: numeric
  arrays are the supported path; the anchor's cached `v` is a typed
  `ArrayCachedValue` (Number/String/Boolean/Error, S2c) so a text/boolean/error
  top-left renders and round-trips correctly; calc-engine only, no xlsx
  round-trip (S2b). _xlfn.
- arraytotext — ARRAYTOTEXT(array,[format]): always one string; walks range/array
  row-major; format 0 (default) concise joins all values with `, `; format 1
  strict wraps in `{...}`, cols `,` rows `;`, strings double-quoted, numbers as
  number string, TRUE/FALSE and error text literal; scalar/range arg supported;
  format not 0/1 => #VALUE!; sibling of VALUETOTEXT; _xlfn.
- valuetotext/bahttext — VALUETOTEXT, BAHTTEXT (VALUETOTEXT(value,[format]):
  format 0 concise = displayed text (numbers as number string, text as-is,
  TRUE/FALSE, error text), format 1 strict wraps strings in double quotes; range
  arg returns top-left scalar (no spill); format not 0/1 => #VALUE!; _xlfn.
  BAHTTEXT(number): Thai numeral-to-words, digit groups of 6 joined by ล้าน with
  positions สิบ/ร้อย/พัน/หมื่น/แสน, units 1 after higher digit => เอ็ด, tens 2 =>
  ยี่สิบ, tens 1 => สิบ; rounds to 2 dp, integer + บาท, satang two-digit + สตางค์
  else ถ้วน, negative prefix ลบ; legacy, no _xlfn).
- aggregate — AGGREGATE (reference form only; function_num 1-19 dispatches to
  AVERAGE/COUNT/COUNTA/MAX/MIN/PRODUCT/STDEV.S/STDEV.P/SUM/VAR.S/VAR.P/MEDIAN/
  MODE.SNGL/LARGE/SMALL/PERCENTILE.INC/QUARTILE.INC/PERCENTILE.EXC/QUARTILE.EXC;
  options 0-7 control ignore behavior, error-ignoring for 2/3/6/7 (skip error
  cells, else propagate), hidden-row skipping for 1/3/5/7 via cell_hidden_status;
  nested SUBTOTAL/AGGREGATE always skipped; reuses Model::percentile_inc/exc;
  function_num 14-19 take trailing k arg; validates function_num 1-19 & options
  0-7 else #VALUE!; array form unsupported (no spill engine); _xlfn).
- depreciation — VDB, AMORLINC, AMORDEGRC (VDB variable declining balance:
  cumulative dep 0->period via per-period DDB (=ScGetGDA) switching to straight-line
  remaining_book/remaining_life when SL>DDB unless no_switch=TRUE, fractional last
  period prorated, VDB=total(end)-total(start); AMORLINC French linear: first period
  cost*rate*yearfrac(purchase,first_period,basis), full periods cost*rate, remainder
  on last, 0 beyond life; AMORDEGRC adds French degressive coeff by 1/rate life band
  (<3=>1, 3-4=>1.5, 5-6=>2, >6=>2.5), last two periods 50%/100%, rounds each year,
  caps at salvage; validate cost/salvage>=0, life>0, 0<=start<=end<=life, factor>0,
  rate>0, basis 0-4 else #NUM!; VDB legacy, AMORLINC/AMORDEGRC _xlfn).
- odd-period bonds — ODDLPRICE, ODDLYIELD, ODDFPRICE, ODDFYIELD (ODDL closed
  form per ECMA/OpenOffice: DCi/DSCi/Ai = yearfrac_basis(last_interest|settlement
  ->maturity, last_interest->settlement)*freq; price = (redemption+DCi*100*rate/f)
  /(DSCi*yld/f+1)-Ai*100*rate/f, yield rearranges same. ODDF discounts quasi-coupon
  cash flows: N/DSC/E via coupon_price_factors, t1=DSC/E, odd first coupon at
  first_coupon = 100*rate/f*odd_first_accrual(issue->first_coupon) when NCD==
  first_coupon, sum regular coupons/redemption over factor^(k-1+t1), minus accrued
  = 100*rate/f*odd_first_accrual(issue->settlement); odd_first_accrual walks
  quasi-coupon periods stepped back from first_coupon summing covered/length per
  basis, handles short & long first periods; ODDFYIELD bisects price-pr. Validates
  rate>=0, yld>=0/pr>0, redemption>0, freq{1,2,4}, basis 0-4, date ordering else
  #NUM!; legacy, no _xlfn).
- duration — DURATION, MDURATION (Macaulay & modified bond duration;
  N=COUPNUM, DSC=COUPDAYSNC, E=COUPDAYS, t1=DSC/E; per period cf=100*coupon/freq
  plus 100 redemption at k=N, time_k=(k-1)+t1, df=1/(1+yld/freq)^time_k;
  Macaulay = sum((time_k/freq)*cf*df)/sum(cf*df), MDURATION = that/(1+yld/freq);
  validates coupon>=0, yld>=0, freq 1/2/4, basis 0-4, settlement<maturity else
  #NUM!; reuses coupon-schedule helpers; legacy, no _xlfn).
- accrued/maturity — ACCRINT, ACCRINTM, PRICEMAT, YIELDMAT (ACCRINTM =
  par*rate*yearfrac(issue,settlement,basis); ACCRINT simplified single-period
  par*rate*yearfrac(issue,settlement,basis), calc_method ignored beyond bool
  validation; PRICEMAT = (100+DIM*rate*100)/(1+DSM*yld)-A*rate*100 with
  DSM=yf(settlement,maturity), DIM=yf(issue,maturity), A=yf(issue,settlement);
  YIELDMAT inverts closed-form; validates rate/par/pr/freq{1,2,4}/basis 0-4 and
  date order else #NUM!; reuses yearfrac_basis; legacy, no _xlfn).
- coupon dates — COUPPCD, COUPNCD, COUPNUM, COUPDAYBS, COUPDAYS, COUPDAYSNC
  (shared coupon-schedule helper: dates stepped back from maturity in 12/freq-month
  increments with end-of-month handling; PCD = last coupon <= settlement, NCD =
  first after, NUM = periods settlement->maturity; DAYBS/DAYSNC via basis day-count
  30/360 for basis 0/4, actual for 1/2/3; DAYS = actual PCD->NCD for basis 1, 365/freq
  for basis 3, else 360/freq; freq must be 1/2/4, basis 0-4, settlement<maturity else
  #NUM!; PCD/NCD return serials; legacy, no _xlfn).
- price/yield — PRICE (settlement,maturity,rate,yld,redemption,freq,[basis]) via
  standard coupon formula redemption/(1+yld/f)^(N-1+DSC/E) + sum coupons -
  accrued; reuses coupon-schedule helpers for N/DSC/E/A; YIELD inverts PRICE for
  pr via bracket+bisection; validates rate>=0, yld>=0/pr>0, redemption>0,
  freq 1/2/4, basis 0-4, settlement<maturity else #NUM!; legacy, no _xlfn.
- disc family — DISC, INTRATE, RECEIVED, PRICEDISC, YIELDDISC (discount-security
  financials; year-fraction via shared `yearfrac_basis` helper refactored out of
  YEARFRAC, bases 0-4; settlement>=maturity or bad basis => #NUM!, bad dates =>
  #VALUE!; legacy, no _xlfn).
- xmatch — XMATCH (position within a single-row/col vector; reuses XLOOKUP
  matching: match_mode 0 exact / -1 next-smaller / 1 next-larger / 2 wildcard,
  search_mode 1 first / -1 last / ±2 binary; 1-based, #N/A if not found; _xlfn).
- mdeterm — MDETERM (determinant of a square range/array via Gaussian elimination
  with partial pivoting; non-square/empty or non-numeric cell => #VALUE!; singular
  => 0).
- forecast — FORECAST/FORECAST.LINEAR (alias) linear regression a+b*x; b =
  sum((xi-xbar)(yi-ybar))/sum((xi-xbar)^2), a = ybar-b*xbar; known_ys/known_xs
  must be equal length & nonempty (#N/A else); xs variance 0 => #DIV/0!.
- permut/prob/trimmean — PERMUT (n!/(n-k)!, trunc to int, #NUM! if n<0|k<0|k>n),
  PROB (sum of probs where lower<=x<=upper; upper omitted => x==lower; ranges must
  match length else #N/A; probs in [0,1] summing to ~1 else #NUM!), TRIMMEAN
  (percent in [0,1) else #NUM!; trim TRUNC(n*percent/2)*2 evenly from sorted ends).
- percentrank — PERCENTRANK/PERCENTRANK.INC, PERCENTRANK.EXC (rank of x as a
  fraction; .INC = i/(n-1), .EXC = (i+1)/(n+1), interpolating between bracketing
  values; x outside [min,max] => #N/A; result truncated to `significance` sig
  digits, default 3, significance<1 => #NUM!; PERCENTRANK == PERCENTRANK.INC).
- quantiles — PERCENTILE/PERCENTILE.INC/PERCENTILE.EXC, QUARTILE/QUARTILE.INC/
  QUARTILE.EXC (.INC interpolates rank k*(n-1); .EXC rank k*(n+1)-1, #NUM! outside
  1/(n+1)..n/(n+1); QUARTILE maps q=0..4 to k=q/4; legacy == .INC).
- ref/math simple — ADDRESS (A1/R1C1 ref string, abs_num 1-4, sheet prefix),
  AREAS (1 for any single reference), MULTINOMIAL, SERIESSUM, FVSCHEDULE,
  PERMUTATIONA (number^chosen), HYPERLINK (returns friendly/link text).
- ceiling/dollar aliases — ECMA.CEILING (→ ISO.CEILING), USDOLLAR (→ DOLLAR).
- width conversion — ASC (full-width -> half-width: U+FF01..U+FF5E -> -0xFEE0,
  U+3000 -> space; ASCII is identity), JIS (inverse half-width -> full-width).
- byte variants — LENB, LEFTB, RIGHTB, MIDB, FINDB, SEARCHB, REPLACEB (aliases of
  the non-B versions; identical outside DBCS).
- newer text — UNICHAR (code point -> char; <1 or invalid/surrogate -> #VALUE!),
  NUMBERVALUE (parse with decimal/group separators, trailing %, empty -> 0).
- simple text — CHAR, CODE (Windows-1252), CLEAN, PROPER, REPLACE, FIXED, DOLLAR.
- TDIST + MODE family — TDIST (→ T.DIST.RT/T.DIST.2T by `tails`), MODE, MODE.SNGL
  (first-encountered most-frequent value, #N/A if none repeats; MODE = MODE.SNGL).
- dispersion legacy — STDEV, STDEVP, VAR, VARP, RANK (→ STDEV.S, STDEV.P, VAR.S,
  VAR.P, RANK.EQ).
- normal-dist legacy — NORMDIST, NORMINV, NORMSDIST, NORMSINV.
- chi/F/t legacy — CHIDIST, CHIINV, CHITEST, FDIST, FINV, FTEST, TINV, TTEST.
- other stat legacy — BETADIST, BETAINV, BINOMDIST, CRITBINOM, CONFIDENCE, COVAR,
  EXPONDIST, GAMMADIST, GAMMAINV, HYPGEOMDIST, LOGINV, LOGNORMDIST, NEGBINOMDIST,
  POISSON, WEIBULL, ZTEST.
