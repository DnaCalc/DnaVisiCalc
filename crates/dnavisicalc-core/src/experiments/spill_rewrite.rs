use crate::address::{CellRef, SheetBounds, col_index_to_label};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteError {
    OutOfBounds(CellRef),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RewrittenCell {
    pub target: CellRef,
    pub formula: String,
}

pub fn rewrite_sequence_as_cell_formulas(
    anchor: CellRef,
    rows: usize,
    cols: usize,
    start: f64,
    step: f64,
    bounds: SheetBounds,
) -> Result<Vec<RewrittenCell>, RewriteError> {
    let end_col = anchor.col as usize + cols - 1;
    let end_row = anchor.row as usize + rows - 1;
    if end_col > bounds.max_columns as usize || end_row > bounds.max_rows as usize {
        return Err(RewriteError::OutOfBounds(anchor));
    }

    let mut out = Vec::with_capacity(rows * cols);
    for row in 0..rows {
        for col in 0..cols {
            let target = CellRef {
                col: anchor.col + col as u16,
                row: anchor.row + row as u16,
            };
            let offset = row * cols + col;
            let value = start + step * offset as f64;
            out.push(RewrittenCell {
                target,
                formula: format!("={value}"),
            });
        }
    }
    Ok(out)
}

pub fn format_target(target: CellRef) -> String {
    format!("{}{}", col_index_to_label(target.col), target.row)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::{CellRef, DEFAULT_SHEET_BOUNDS};

    #[test]
    fn rewrites_sequence_into_scalar_formulas() {
        let cells = rewrite_sequence_as_cell_formulas(
            CellRef::from_a1("A1").expect("A1"),
            2,
            2,
            1.0,
            1.0,
            DEFAULT_SHEET_BOUNDS,
        )
        .expect("rewrite");

        assert_eq!(cells.len(), 4);
        assert_eq!(format_target(cells[0].target), "A1");
        assert_eq!(format_target(cells[1].target), "B1");
        assert_eq!(format_target(cells[3].target), "B2");
        assert_eq!(cells[0].formula, "=1");
        assert_eq!(cells[3].formula, "=4");
    }
}
