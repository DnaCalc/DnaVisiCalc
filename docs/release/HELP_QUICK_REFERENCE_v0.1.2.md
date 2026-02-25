# DNA VisiCalc v0.1.2 - Help Quick Reference

## Navigation
- Arrows or `h/j/k/l`: move selection
- `Enter` or `e`: edit selected cell
- `:`: command mode
- `r`: recalculate
- `q`: quit
- `?` or `F1`: toggle full help

## Command mode
- `w <path>` / `write <path>`: save workbook
- `o <path>` / `open <path>`: open workbook
- `w` (no path): save to last path
- `set <A1> <value|formula>`: assign a cell
- `name <NAME> <value|formula>`: assign workbook name
- `name clear <NAME>`: remove workbook name
- `mode auto|manual`: recalc mode
- `r` / `recalc`: recalculate now
- `q` / `quit`: quit

## Supported functions
- `SUM`, `MIN`, `MAX`, `AVERAGE`, `COUNT`
- `IF`, `AND`, `OR`, `NOT`
- `ABS`, `INT`, `ROUND`, `SIGN`, `SQRT`
- `EXP`, `LN`, `LOG10`
- `SIN`, `COS`, `TAN`, `ATN`, `PI`
- `NPV`, `PV`, `FV`, `PMT`
- `LOOKUP`, `NA`, `ERROR`
- `CONCAT`, `LEN`
- `SEQUENCE`, `RANDARRAY`
