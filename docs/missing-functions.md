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

## Tier 2 — medium scalar (not started)
AGGREGATE, BAHTTEXT, VALUETOTEXT. See triage report.

## Tier 3 — blocked on spill engine
FILTER, SORT, UNIQUE, SEQUENCE, TRANSPOSE, MMULT, FREQUENCY, … (full list in triage).

## Tier 4 — blocked on lambda support
LAMBDA, LET, MAP, REDUCE, SCAN, BYROW, BYCOL, MAKEARRAY, ISOMITTED.

## Tier 5 — out of scope
CUBE*, GETPIVOTDATA, GROUPBY, PERCENTOF, RTD, IMAGE, PHONETIC.

## Shipped
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
