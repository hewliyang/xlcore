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
- [x] **S3 — reference/intersection polish.** `A1#` spill operator + `@`
  implicit intersection both done (see Shipped).
- [ ] **S4 — roll out Tier 3b/3c + array Tier 4** one fn per agent once S2c lands.
  High value first: ~~SEQUENCE~~, ~~SORT~~, ~~UNIQUE~~, ~~FILTER~~, ~~HSTACK~~/~~VSTACK~~, ~~TAKE~~/~~DROP~~,
  ~~CHOOSECOLS~~/~~CHOOSEROWS~~, ~~TOCOL~~/~~TOROW~~, ~~EXPAND~~, ~~SORTBY~~, ~~MMULT~~, ~~MINVERSE~~, ~~MUNIT~~, ~~RANDARRAY~~,
  ~~FREQUENCY~~, ~~MODE.MULT~~, ~~TEXTSPLIT~~, ~~LINEST~~/~~LOGEST~~/~~TREND~~/~~GROWTH~~.
  All S4 high-value rollout items now struck (shipped).

Write path: `CalcResult::Array` at model.rs:684. xlsx persistence: dynamic arrays
live on the anchor cell as `<f t="array" ref=...>` + `cm` → `xl/metadata.xml`
`<dynamicArrayProperties fDynamic="1"/>`; spilled cells are plain cached `<v>`;
`#`/`@` serialize as `_xlfn.ANCHORARRAY`/`_xlfn.SINGLE`.

## Tier 4 — lambda / name-binding support
9 fns: LAMBDA, LET, MAP, REDUCE, SCAN, BYROW, BYCOL, MAKEARRAY, ISOMITTED. Needs a
scoped name-binding environment + a lambda/closure value in the calc engine.
Key seams: `evaluate_function` gets **un-evaluated** `args: &[Node]` (model.rs
~L2289) so a handler controls evaluation order; `WrongVariableKind` eval
(model.rs:440) is where a bare name currently errors `#NAME?` — that's the
scope-resolution hook. LET binding names already parse as `WrongVariableKind`
nodes (no parser change needed for LET); names that happen to lex as a cell ref
(`a1`) are an accepted limitation (Excel rejects them too). Staged backlog — one
item per agent, top to bottom:

- [x] **L1 — eval scope + LET.** See Shipped. Add `Function::Let` (full mod.rs ritual,
  `_xlfn.LET`). Add a scope stack to `Model` (`Vec<HashMap<String, CalcResult>>`
  or similar). `fn_let(args, cell)`: args are name1,value1,[name2,value2,...],calc
  (odd count >=3); read each binding name from the `WrongVariableKind` node string,
  evaluate its value node in the current scope, push the binding; evaluate the
  final calc node in the extended scope; pop. `WrongVariableKind` eval resolves
  from the scope stack (top-down) before falling back to `#NAME?`. Scalar + range
  values both bind. Round-trips as `_xlfn.LET`. Tests: `=LET(x,5,x*2)`=>10,
  `=LET(x,1,y,x+1,x+y)`=>3, nested LET, range binding `=LET(d,A1:A3,SUM(d))`.
- [ ] **L2 — LAMBDA closures + invocation + ISOMITTED.** Add a `CalcResult::Lambda`
  value capturing param names + body `Node` + a snapshot of the enclosing scope.
  `Function::Lambda` builds the closure (last arg = body, preceding args = param
  names). Parser: postfix call `expr(args)` so `=LAMBDA(x,x+1)(5)`=>6 evaluates.
  Named lambdas: a defined name whose formula is `=LAMBDA(...)` is callable as
  `MYFN(5)` (resolve the defined-name FunctionKind/call path to the closure). Add
  ISOMITTED (TRUE when a lambda param was omitted at the call site). Round-trips as
  `_xlfn.LAMBDA` / `_xlfn.ISOMITTED`.
- [ ] **L3 — MAP / REDUCE / SCAN.** Higher-order fns taking array(s) + a lambda
  value; invoke the closure per element. MAP(arr...,lambda) elementwise =>
  CalcResult::Array; REDUCE(init,arr,lambda(acc,x)) => scalar; SCAN(init,arr,
  lambda(acc,x)) => running-accumulation Array. Spill + round-trip.
- [ ] **L4 — BYROW / BYCOL / MAKEARRAY.** BYROW(arr,lambda(row))/BYCOL(arr,
  lambda(col)) => column/row Array; MAKEARRAY(rows,cols,lambda(r,c)) => Array.
  Spill + round-trip.

## Tier 5 — out of scope
CUBE*, GETPIVOTDATA, GROUPBY, PERCENTOF, RTD, IMAGE, PHONETIC.

## Shipped
- let (L1) — LET(name1,value1,[name2,value2,...],calc): odd arg count >=3 else
  #ERROR!. Each name node must be a `Node::WrongVariableKind` (a bare name that
  lexed as a cell ref => #VALUE!); evaluate each value in the current scope, bind
  name->CalcResult into a single pushed frame so later bindings see earlier ones,
  evaluate the final calc in the extended scope, pop once (every return path).
  `Model.let_scopes: Vec<HashMap<String, CalcResult>>` is transient runtime state
  (mirrors spill_owners, not serialized); `WrongVariableKind` eval walks it
  top-down before the #NAME? fallback. Scalars and ranges both bind
  (=LET(d,A1:A3,SUM(d))=>6, nested LET=>6); round-trips as _xlfn.LET.
- spill operator (S3, A1#) — a reference followed by `#` parses to
  Node::SpillReferenceKind (new lexer TokenType::Spill: bare `#` not forming a
  known error literal and not followed by a letter falls through from
  consume_error). Eval (evaluate_node_in_context) resolves the reference to its
  anchor cell, forces its evaluation, and if it's a Cell::CellFormulaArray reads
  the cached spill `range` and returns CalcResult::Array of the whole block (so a
  bare `=A1#` spills again and `=SUM(A1#)` aggregates); a non-array, non-empty
  cell returns that cell's value (`A1#` == `A1`); an empty anchor => #REF!.
  Stringify emits `_xlfn.ANCHORARRAY(ref)` for xlsx and `ref#` for display, and
  `_xlfn.ANCHORARRAY(...)` parses back on read; the formula string round-trips.
  Deferred: xl/metadata.xml dynamic-array `cm` fidelity (same as S2b). _xlfn.
- implicit intersection (S3, @) — set_cell_value's `CalcResult::Range` arm now
  applies real implicit intersection via the `implicit_intersection` helper
  (intersecting row/col => that cell's value, non-intersecting => #VALUE!,
  single-cell range => that cell) instead of the old #N/IMPL stub; matches the
  eval-time `ImplicitIntersection` arm. `@` coercion of a dynamic-array result
  (e.g. `=@SEQUENCE(3)`) now collapses to the top-left element rather than
  spilling; bare arrays still spill. A1# spill operator remains the open S3 item.
- logest/growth (S4) — exponential analogues of LINEST/TREND for y=b*m1^x1*...
  via shared ols_fit on ln(known_ys) (all known_ys must be >0 else #NUM!). LOGEST
  returns LINEST shape with coefficients exponentiated (row0 m_i=exp(coef),
  b=exp(intercept)); stats=TRUE keeps the ln-fit regression stats unexponentiated
  (standard errors of linear coeffs, r^2, sey, F, df, ss_reg, ss_resid). GROWTH
  fits the ln model and predicts exp(linear_prediction) for new_xs (omitted =>
  known xs), output shaped to new_xs orientation. CalcResult::Array; 1-4 args else
  #ERROR!; legacy (plain name, no _xlfn).
- linest/trend (S4) — LINEST(known_ys,[known_xs],[const],[stats]) /
  TREND(known_ys,[known_xs],[new_xs],[const]) via shared ordinary-least-squares
  ols_fit (normal equations (X^T X)beta = X^T y, X^T X inverted with the matrix.rs
  Gauss-Jordan helper). known_ys flattened to n via read_array_arg; known_xs read as
  matrix, oriented to n rows x k predictors (omitted => single column 1..=n); const
  default TRUE fits intercept, FALSE forces 0 (uncorrected sums). LINEST returns the
  coefficient row {m_k..m_1, b}; stats=TRUE returns the 5-row array (coeffs, standard
  errors, {r^2,sey}, {F,df}, {ss_reg,ss_resid}) padded with #N/A. TREND fits then
  predicts each new_xs row (omitted => known_xs), output shaped column/row to match
  new_xs orientation. Both CalcResult::Array (spill + round-trip); 1-4 args else
  #ERROR!; non-numeric/length mismatch => #VALUE!; legacy (plain name, no _xlfn).
- mode.mult (S4) — MODE.MULT(number1,[number2],...): collects numeric values like
  MODE.SNGL (shared collect_mode_values; Number/Range/Array, non-numeric ignored,
  Error propagates), finds the max frequency among values appearing >=2 times, and
  returns a vertical (single-column) CalcResult::Array of every distinct value with
  that count in first-occurrence order; no value repeats => #N/A; single mode => 1x1
  (spills); >=1 arg else #ERROR!; _xlfn.
- textsplit (S4) — TEXTSPLIT(text,col_delimiter,[row_delimiter],[ignore_empty],
  [match_mode],[pad_with]): row_delimiter splits text into ROWS (whole text = 1 row
  if omitted), col_delimiter splits each row into COLUMNS. Each delimiter arg is a
  single string or a range/array of strings (split on ANY of them, via split_on_any,
  empty delimiter strings dropped); col_delimiter required and non-empty else
  #VALUE!. ignore_empty (default FALSE) drops empty fields from consecutive
  delimiters and fully-empty rows; match_mode 0 case-sensitive (default) / 1
  case-insensitive (else #VALUE!). Short rows padded to max column count with
  pad_with (default #N/A). Fields kept as ArrayNode::String (no numeric coercion).
  Returns CalcResult::Array (spills + round-trips); 2-6 args else #ERROR!; _xlfn.
- frequency (S4) — FREQUENCY(data_array,bins_array): counts data values per bin,
  returns a COLUMN array of length bins+1. data/bins read via read_array_arg,
  flattened ignoring non-numeric cells; bins sorted ascending + deduped for
  counting, result[0]=count(x<=bin0), result[i]=count(bin[i-1]<x<=bin[i]),
  result[last]=count(x>last bin); duplicate bin slots get 0 (mapped back to
  original bin order). Returns CalcResult::Array (spills + round-trips); exactly 2
  args else #ERROR!; legacy (no _xlfn).
- randarray (S4) — RANDARRAY([rows],[columns],[min],[max],[whole_number]):
  builds a rows x columns numeric grid (SEQUENCE pattern) of random values from the
  engine RAND source (functions::random). rows/columns default 1, truncated to int,
  <1 => #VALUE!; min default 0, max default 1, min>max => #VALUE!; whole_number FALSE
  (default) reals in [min,max), TRUE integers in [min,max] inclusive (min/max must be
  integers else #VALUE!). Returns CalcResult::Array (spills + round-trips); 0-5 args
  else #ERROR!; static_analysis not_implemented (volatile, recalcs each eval); _xlfn.
- mmult/minverse/munit (S4) — MMULT(a,b) matrix product via shared
  read_numeric_matrix (rectangular numeric reader factored out of read_square_matrix;
  any non-numeric/empty cell => #VALUE!); a is m x n, b must be n x p else #VALUE!,
  result[i][j]=sum_k a[i][k]*b[k][j]. MINVERSE(a) inverse of a square matrix
  (non-square => #VALUE!) via Gauss-Jordan with partial pivoting augmenting with
  identity; zero pivot => #NUM! (singular). MUNIT(n) n x n identity, n truncated to
  int, n<1 => #VALUE!. All return CalcResult::Array (spill + round-trip); MMULT 2
  args, MINVERSE/MUNIT 1 arg else #ERROR!; MMULT/MINVERSE legacy (plain name), MUNIT
  _xlfn.
- sortby (S4) — SORTBY(array,by_array1,[sort_order1],[by_array2,sort_order2],...):
  read array via shared read_array_arg into Vec<Vec<ArrayNode>>; args[1..] parsed as
  (by_array, [sort_order]) groups — each by_array flattened to a vector that must
  equal array's row count else #VALUE!, optional following scalar number is
  sort_order (1 asc default, -1 desc, else #VALUE!). Stable multi-key sort of row
  indices via array_node_cmp (key1, tie-break key2, ...), reorders array rows.
  Returns CalcResult::Array (spills + round-trips); >=2 args else #ERROR!; _xlfn.
- expand (S4) — EXPAND(array,rows,[columns],[pad_with]): read array via shared
  read_array_arg into Vec<Vec<ArrayNode>>, pad out to rows tall x columns wide
  copying existing cells top-left and filling the rest with pad_with (default
  #N/A). rows/columns truncated to int; omitted/empty keeps current dimension;
  rows<1 or columns<1, or target < current dimension (EXPAND can't shrink) =>
  #VALUE!. pad_with is a scalar -> ArrayNode. Returns CalcResult::Array (spills +
  round-trips); 2-4 args else #ERROR!; _xlfn.
- tocol/torow (S4) — TOCOL(array,[ignore],[scan_by_column])/TOROW(...): read
  array via shared read_array_arg, flatten in row-major (default) or column-major
  order (scan_by_column TRUE), apply ignore filter, then shape into a single
  column (TOCOL) or row (TOROW). ignore validated 0-3 else #VALUE!; ignore 2/3
  drops error cells; ignore-blanks (1/3) is a no-op because read_array_arg
  materializes empty cells as Number(0.0) so blanks can't be distinguished. Empty
  result => #CALC!. Returns CalcResult::Array (spills + round-trips); 1-3 args else
  #ERROR!; _xlfn.
- choosecols/chooserows (S4) — CHOOSECOLS(array,col_num1,...)/CHOOSEROWS(
  array,row_num1,...): read array via shared read_array_arg into
  Vec<Vec<ArrayNode>>; index args flattened in order (each may be a scalar or a
  range/array of indices, truncated to int); 1-based, negative counts from end
  (-1=last), repeats allowed; index 0 or |index|>dimension => #VALUE!. Returns
  CalcResult::Array (spills + round-trips); >=2 args else #ERROR!; _xlfn.
- take/drop (S4) — TAKE(array,rows,[columns])/DROP(array,rows,[columns]): read
  array via shared read_array_arg into Vec<Vec<ArrayNode>>; rows/columns truncated
  to int, positive keeps/drops from start (top/left), negative from end
  (bottom/right). TAKE omitted rows/cols keeps all; abs>=len clamps to whole. DROP
  omitted rows/cols drops none. rows=0/cols=0 in TAKE or DROP removing everything
  => #CALC!. Returns CalcResult::Array (spills + round-trips); 2-3 args else
  #ERROR!; _xlfn.
- hstack/vstack (S4) — HSTACK(a,b,...)/VSTACK(a,b,...): each reads 1+ array/range
  args into Vec<Vec<ArrayNode>> via shared read_array_arg (transpose pattern;
  Range walks evaluate_cell, Array passthrough, Error propagates, scalar=>1x1).
  HSTACK concats columns left-to-right, result rows = max input rows, missing cells
  padded ArrayNode::Error(NA) (#N/A). VSTACK concats rows top-to-bottom, result cols
  = max input cols, missing padded #N/A. Returns CalcResult::Array (spills +
  round-trips); 0 args => #ERROR!; _xlfn.
- filter (S4) — FILTER(array,include,[if_empty]): reads array range/array row-major
  into Vec<Vec<ArrayNode>> (TRANSPOSE/SORT pattern); include is a 1-D mask flattened
  to a vector — column vector of height==rows keeps matching ROWS, row vector of
  width==cols keeps matching COLUMNS; truthiness: nonzero Number/TRUE keep, 0/FALSE
  drop, Error propagates, text => #VALUE!; mask length != rows and != cols =>
  #VALUE!; no rows/cols kept => if_empty as 1x1 array if given else #CALC!; returns
  CalcResult::Array preserving cell types (spills + round-trips); 2-3 args else
  #ERROR!; _xlfn.
- unique (S4) — UNIQUE(array,[by_col],[exactly_once]): reads range/array row-major
  into Vec<Vec<ArrayNode>> (TRANSPOSE/SORT pattern); by_col FALSE (default) dedups
  ROWS, TRUE dedups COLUMNS; exactly_once FALSE (default) keeps each distinct
  row/col once in first-occurrence order, TRUE keeps only rows/cols appearing
  exactly once; whole-row/col equality elementwise via array_node_cmp (numbers
  exact, text case-insensitive, types distinct); empty result => #CALC!; returns
  CalcResult::Array preserving cell types (spills + round-trips); 1-3 args else
  #ERROR!; _xlfn.
- sort (S4) — SORT(array,[sort_index],[sort_order],[by_col]): reads range/array
  row-major into Vec<Vec<ArrayNode>> (like TRANSPOSE), stable-sorts and returns
  CalcResult::Array (spills + round-trips). sort_index 1-based (default 1), out of
  range => #VALUE!; sort_order 1 asc (default) / -1 desc, else => #VALUE!; by_col
  FALSE sorts rows by sort_index column, TRUE sorts columns by sort_index row.
  Comparison mirrors CalcResult Ord (numbers < text < FALSE < TRUE, text
  case-insensitive) via array_node_cmp; types preserved (text anchor caches text,
  S2c); 1-4 args else #ERROR!; _xlfn.
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
