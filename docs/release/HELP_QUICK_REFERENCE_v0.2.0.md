# DNA VisiCalc v0.2.0 - Help Quick Reference

## Navigation
- Arrows or `h/j/k/l`: move selection
- `Shift+Arrows` or `Shift+H/J/K/L`: extend selection
- `Delete`: clear selected cell/range contents
- `Enter`, `e`, or `F2`: edit selected cell
- `Ctrl+C`: copy selected range to system clipboard
- `Ctrl+V`: paste from system clipboard (opens Paste Special)
- `:`: command mode
- `r`: recalculate
- `q`: quit
- `?` or `F1`: toggle full help

## Paste Special (`Ctrl+V`)
- `1` `All`: formulas/values + formatting
- `2` `Formulas`: formulas only
- `3` `Values`: values only
- `4` `Values+KeepDestFmt`: values only, keep destination formatting
- `5` `Formatting`: formatting only
- `Tab` or arrows or `j/k`: cycle mode
- `Enter`: apply
- `Esc`: cancel

## Command mode
- `w <path>` / `write <path>`: save workbook
- `o <path>` / `open <path>`: open workbook
- `w` (no path): save to last path
- `set <A1> <value|formula>`: assign a cell
- `name <NAME> <value|formula>`: assign workbook name
- `name clear <NAME>`: remove workbook name
- `fmt decimals <0..9|none>`: set decimals on selection
- `fmt bold on|off`: set text bold on selection
- `fmt italic on|off`: set text italic on selection
- `fmt fg <color|none>`: set foreground color on selection
- `fmt bg <color|none>`: set background color on selection
- `fmt clear`: reset formatting on selection
- `mode auto|manual`: recalc mode
- `r` / `recalc`: recalculate now
- `q` / `quit`: quit

Palette colors:
- `MIST`, `SAGE`, `FERN`, `MOSS`, `OLIVE`, `SEAFOAM`, `LAGOON`, `TEAL`
- `SKY`, `CLOUD`, `SAND`, `CLAY`, `PEACH`, `ROSE`, `LAVENDER`, `SLATE`

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
- `LET`, `LAMBDA`, `MAP` (including array-returning lambda spill tiling)
- `INDIRECT` (A1 and R1C1), `OFFSET`, `ROW`, `COLUMN`
