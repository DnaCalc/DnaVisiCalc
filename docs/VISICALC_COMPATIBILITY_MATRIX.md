# VisiCalc Compatibility Matrix (v0.1)

This file tracks which historically VisiCalc-like features are implemented in DnaVisiCalc v0.1, and which remain compatibility-risk areas.

## 1. Implemented (v0.1)

### 1.1 Sheet and model
- Grid bounds: `A1..BK254`.
- A1 references and ranges (`A1...B7` and `A1:B7`).
- Manual and automatic recalc modes.
- Deterministic dependency ordering and cycle detection.

### 1.2 Formula language
- Arithmetic: `+ - * / ^`.
- Comparisons: `= <> < <= > >=`.
- Text literals (`"..."` with doubled quote escaping) and concatenation (`&`).

### 1.3 Functions
- Aggregates/logical: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`, `IF`, `AND`, `OR`, `NOT`.
- Math/trig: `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`.
- Financial/table/error: `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`, `NA`, `ERROR`.
- Text: `CONCAT`, `LEN`.

## 2. Implemented but Semantics Need Deeper Compatibility Validation

These are available in v0.1, but historical edge semantics should still be cross-checked against public VisiCalc evidence:

- `LOOKUP` behavior for ambiguous table shapes and non-monotonic keys.
- Financial functions (`NPV`, `PV`, `FV`, `PMT`) around sign conventions and optional argument defaults.
- Error display conventions (`NA`, `ERROR`) versus original textual markers.
- Type coercion behavior involving text in numeric aggregates and boolean contexts.

## 3. Not Yet Implemented / Classified as Problematic

### 3.1 Higher-risk compatibility areas
- Exact VisiCalc formatting model and printer/report controls.
- Historical storage/import/export parity beyond the local deterministic `DVISICALC` format.
- Full command-language parity from the original interactive menus.

### 3.2 Architecture-era differences (intentional)
- Dynamic arrays (`SEQUENCE`, `RANDARRAY`, spill semantics) are modern extensions and not historical VisiCalc features.
- Current TUI command shell is modernized and test-oriented, not a strict clone of VisiCalc UI flows.

## 4. Recommended Next Compatibility Steps

1. Build a public-evidence fixture pack for `LOOKUP` and financial functions.
2. Add explicit compatibility profile flags for strict VisiCalc coercion/error text behavior.
3. Expand command/UI matrix tests against archived VisiCalc interaction references.
