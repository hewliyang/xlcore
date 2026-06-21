# Missing functions — implementation triage

156 functions in the ECMA-376 ∪ EPPlus reference are absent from `ironcalc-base`.
Tiered by effort. The dominant constraint is architectural: **ironcalc has no spill
engine.** A formula whose result is `CalcResult::Array` is written to the grid as
`#N/IMPL "Arrays not supported yet"` (`model.rs`), and implicit intersection is stubbed
to `#N/IMPL` too. So every dynamic-array function is blocked until that lands.

## Tier 1 — Trivial (≈1–2 days total, mostly aliases/wrappers) — 65
Drop-in scalar functions following existing patterns.

- **Legacy stat aliases** over modern impls ironcalc already has (`NORM.DIST`, `BETA.DIST`,
  `STDEV.S`, …): NORMDIST, NORMINV, NORMSDIST, NORMSINV, BETADIST, BETAINV, BINOMDIST,
  CRITBINOM, CHIDIST, CHIINV, CHITEST, CONFIDENCE, COVAR, EXPONDIST, FDIST, FINV, FTEST,
  GAMMADIST, GAMMAINV, HYPGEOMDIST, LOGINV, LOGNORMDIST, MODE, MODE.SNGL, NEGBINOMDIST,
  POISSON, RANK, STDEV, STDEVP, TDIST, TINV, TTEST, VAR, VARP, WEIBULL, ZTEST.
  Each ~5 lines mapping to the existing `.`-form.
- **Aliases**: ECMA.CEILING (=ISO.CEILING), USDOLLAR (=DOLLAR).
- **Byte variants** — identical to non-B versions outside DBCS locales: LENB, LEFTB, RIGHTB,
  MIDB, FINDB, SEARCHB, REPLACEB, ASC, JIS (ASC/JIS = identity in non-CJK).
- **Simple text**: CHAR, CODE, CLEAN, PROPER, REPLACE, FIXED, DOLLAR, UNICHAR, NUMBERVALUE.
- **Simple math/ref**: MULTINOMIAL, SERIESSUM, ADDRESS (string build), AREAS (count),
  FVSCHEDULE, PERMUTATIONA, HYPERLINK (return friendly text).

## Tier 2 — Medium scalar, self-contained (≈1–3 weeks) — 44
Real algorithms but no engine changes needed.

- **Quantile/regression**: PERCENTILE(.INC/.EXC), QUARTILE(.INC/.EXC),
  PERCENTRANK(.INC/.EXC), PERMUT, PROB, TRIMMEAN, FORECAST/FORECAST.LINEAR.
- **Financial bond family** — well-defined but need day-count basis (30/360, act/act…) and
  an iterative solver for yields (infra exists in `financial_util.rs`): ACCRINT, ACCRINTM,
  AMORDEGRC, AMORLINC, COUPDAYBS, COUPDAYS, COUPDAYSNC, COUPNCD, COUPNUM, COUPPCD, DISC,
  DURATION, INTRATE, MDURATION, ODDFPRICE, ODDFYIELD, ODDLPRICE, ODDLYIELD, PRICE,
  PRICEDISC, PRICEMAT, RECEIVED, VDB, YIELD, YIELDDISC, YIELDMAT.
- **Scalar matrix/lookup**: MDETERM (determinant), XMATCH (position, match modes).
- **Other**: AGGREGATE (dispatch to 19 subfns + ignore options), BAHTTEXT, VALUETOTEXT.

## Tier 3 — Blocked on a spill/array engine — 25
Return dynamic arrays; cannot be written to the grid today. Implement the spill engine
first, then these are mostly straightforward.

FILTER, SORT, SORTBY, UNIQUE, SEQUENCE, RANDARRAY, DROP, TAKE, EXPAND, HSTACK, VSTACK,
TOCOL, TOROW, CHOOSECOLS, CHOOSEROWS, TRANSPOSE, MMULT, MINVERSE, MUNIT, FREQUENCY,
MODE.MULT, LINEST, LOGEST, TREND, GROWTH, TEXTSPLIT, ARRAYTOTEXT.
(ARRAYTOTEXT/TEXTSPLIT can do scalar/range partials sooner.)

## Tier 4 — Blocked on lambda/closure support — 9
Need first-class lambda values in the parser + evaluator (none today), plus spill for the
array-producing ones.

LAMBDA, LET, MAP, REDUCE, SCAN, BYROW, BYCOL, MAKEARRAY, ISOMITTED.
(LET is the cheapest — named bindings, no closures — could land before the rest.)

## Tier 5 — Out of scope / needs external model — 13
Require subsystems ironcalc doesn't model.

- **Cube/OLAP**: CUBEKPIMEMBER, CUBEMEMBER, CUBEMEMBERPROPERTY, CUBERANKEDMEMBER, CUBESET,
  CUBESETCOUNT, CUBEVALUE.
- **Pivot model**: GETPIVOTDATA, GROUPBY, PERCENTOF.
- **Host/IO**: RTD (live feed), IMAGE (cell image), PHONETIC (furigana metadata).

## Suggested order
1. Tier 1 aliases — fast coverage win, lifts the count ~65 with little risk.
2. Tier 2 financial + quantiles — high user value, mechanical.
3. **Spill engine** (the unlock) → Tier 3 + the array half of Tier 4.
4. Lambda support → rest of Tier 4.
5. Skip Tier 5 unless the surrounding subsystems get built.
