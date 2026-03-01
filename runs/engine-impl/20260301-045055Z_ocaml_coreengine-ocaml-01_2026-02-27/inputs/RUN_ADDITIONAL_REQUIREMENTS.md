# Run-Specific Additional Requirements

## Focus

- Primary target: close `REQ-CALC-008` function list gaps.
- Secondary targets: improve `REQ-CALC-004` cycle-mode depth and `REQ-STR-*` rewrite-depth evidence.

## Required Function Surface Expansion

- Implement and validate these required groups (at minimum):
  - aggregates: `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`
  - conditional/error: `IF`, `IFERROR`, `IFNA`, `NA`, `ERROR`
  - logical/predicate: `AND`, `OR`, `NOT`, `ISERROR`, `ISNA`, `ISBLANK`, `ISTEXT`, `ISNUMBER`, `ISLOGICAL`, `ERROR.TYPE`
  - numeric/scientific: `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`, `EXP`, `LN`, `LOG10`, `SIN`, `COS`, `TAN`, `ATN`, `PI`
  - financial/lookup: `NPV`, `PV`, `FV`, `PMT`, `LOOKUP`
  - text/reference helpers: `CONCAT`, `LEN`, `ROW`, `COLUMN`

## Evidence Requirements

- Add executable tests per function group and include expected-value assertions.
- Update conformance matrix with REQ-CALC-008 subgroup coverage details.
- Preserve 104 `dvc_*` exports and existing passing CT set.
