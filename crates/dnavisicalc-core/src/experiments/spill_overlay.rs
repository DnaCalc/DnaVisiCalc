use std::collections::{BTreeMap, BTreeSet};

use crate::address::{CellRange, CellRef, SheetBounds};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpillOverlayError {
    OutOfBounds(CellRef),
    BlockedByInput(CellRef),
    BlockedBySpill(CellRef),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpillOverlayPlan {
    pub anchor: CellRef,
    pub range: CellRange,
}

#[derive(Debug, Default)]
pub struct SpillOverlayPlanner {
    occupied_inputs: BTreeSet<CellRef>,
    claimed_spills: BTreeMap<CellRef, CellRef>,
}

impl SpillOverlayPlanner {
    pub fn with_inputs(inputs: impl IntoIterator<Item = CellRef>) -> Self {
        Self {
            occupied_inputs: inputs.into_iter().collect(),
            claimed_spills: BTreeMap::new(),
        }
    }

    pub fn plan_spill(
        &mut self,
        anchor: CellRef,
        rows: usize,
        cols: usize,
        bounds: SheetBounds,
    ) -> Result<SpillOverlayPlan, SpillOverlayError> {
        let end_col = anchor.col as usize + cols - 1;
        let end_row = anchor.row as usize + rows - 1;
        if end_col > bounds.max_columns as usize || end_row > bounds.max_rows as usize {
            return Err(SpillOverlayError::OutOfBounds(anchor));
        }

        let end = CellRef {
            col: end_col as u16,
            row: end_row as u16,
        };
        let range = CellRange::new(anchor, end);

        for cell in range.iter() {
            if cell != anchor && self.occupied_inputs.contains(&cell) {
                return Err(SpillOverlayError::BlockedByInput(cell));
            }
            if cell != anchor && self.claimed_spills.contains_key(&cell) {
                return Err(SpillOverlayError::BlockedBySpill(cell));
            }
        }

        for cell in range.iter() {
            if cell != anchor {
                self.claimed_spills.insert(cell, anchor);
            }
        }

        Ok(SpillOverlayPlan { anchor, range })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::address::{CellRef, DEFAULT_SHEET_BOUNDS};

    #[test]
    fn detects_blocked_input_cells() {
        let mut planner = SpillOverlayPlanner::with_inputs([
            CellRef::from_a1("A2").expect("A2"),
            CellRef::from_a1("C1").expect("C1"),
        ]);
        let result = planner.plan_spill(
            CellRef::from_a1("A1").expect("A1"),
            3,
            1,
            DEFAULT_SHEET_BOUNDS,
        );
        assert_eq!(
            result,
            Err(SpillOverlayError::BlockedByInput(
                CellRef::from_a1("A2").expect("A2")
            ))
        );
    }

    #[test]
    fn detects_inter_spill_collision() {
        let mut planner = SpillOverlayPlanner::default();
        planner
            .plan_spill(
                CellRef::from_a1("A1").expect("A1"),
                2,
                2,
                DEFAULT_SHEET_BOUNDS,
            )
            .expect("first spill");
        let second = planner.plan_spill(
            CellRef::from_a1("B1").expect("B1"),
            2,
            2,
            DEFAULT_SHEET_BOUNDS,
        );
        assert_eq!(
            second,
            Err(SpillOverlayError::BlockedBySpill(
                CellRef::from_a1("B2").expect("B2")
            ))
        );
    }
}
