use std::fmt;
use std::str::FromStr;

pub const MAX_COLUMNS: u16 = 63;
pub const MAX_ROWS: u16 = 254;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetBounds {
    pub max_columns: u16,
    pub max_rows: u16,
}

pub const DEFAULT_SHEET_BOUNDS: SheetBounds = SheetBounds {
    max_columns: MAX_COLUMNS,
    max_rows: MAX_ROWS,
};

impl Default for SheetBounds {
    fn default() -> Self {
        DEFAULT_SHEET_BOUNDS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellRef {
    pub col: u16,
    pub row: u16,
}

impl CellRef {
    pub fn new(col: u16, row: u16, bounds: SheetBounds) -> Result<Self, AddressError> {
        if col == 0 || col > bounds.max_columns {
            return Err(AddressError::ColumnOutOfBounds {
                col,
                max: bounds.max_columns,
            });
        }
        if row == 0 || row > bounds.max_rows {
            return Err(AddressError::RowOutOfBounds {
                row,
                max: bounds.max_rows,
            });
        }
        Ok(Self { col, row })
    }

    pub fn from_a1_with_bounds(input: &str, bounds: SheetBounds) -> Result<Self, AddressError> {
        parse_cell_ref(input, bounds)
    }

    pub fn from_a1(input: &str) -> Result<Self, AddressError> {
        Self::from_a1_with_bounds(input, DEFAULT_SHEET_BOUNDS)
    }

    pub fn to_a1(self) -> String {
        format!("{}{}", col_index_to_label(self.col), self.row)
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_a1())
    }
}

impl FromStr for CellRef {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_a1(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CellRange {
    pub start: CellRef,
    pub end: CellRef,
}

impl CellRange {
    pub fn new(a: CellRef, b: CellRef) -> Self {
        let start_col = a.col.min(b.col);
        let end_col = a.col.max(b.col);
        let start_row = a.row.min(b.row);
        let end_row = a.row.max(b.row);
        Self {
            start: CellRef {
                col: start_col,
                row: start_row,
            },
            end: CellRef {
                col: end_col,
                row: end_row,
            },
        }
    }

    pub fn iter(self) -> CellRangeIter {
        CellRangeIter {
            range: self,
            current_col: self.start.col,
            current_row: self.start.row,
            done: false,
        }
    }
}

pub struct CellRangeIter {
    range: CellRange,
    current_col: u16,
    current_row: u16,
    done: bool,
}

impl Iterator for CellRangeIter {
    type Item = CellRef;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let cell = CellRef {
            col: self.current_col,
            row: self.current_row,
        };

        if self.current_row < self.range.end.row {
            self.current_row += 1;
        } else if self.current_col < self.range.end.col {
            self.current_col += 1;
            self.current_row = self.range.start.row;
        } else {
            self.done = true;
        }

        Some(cell)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressError {
    Empty,
    InvalidFormat(String),
    ColumnOutOfBounds { col: u16, max: u16 },
    RowOutOfBounds { row: u16, max: u16 },
    InvalidColumnLabel(String),
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "cell reference is empty"),
            Self::InvalidFormat(input) => write!(f, "invalid cell reference format: {input}"),
            Self::ColumnOutOfBounds { col, max } => {
                write!(f, "column {col} is out of bounds (max {max})")
            }
            Self::RowOutOfBounds { row, max } => {
                write!(f, "row {row} is out of bounds (max {max})")
            }
            Self::InvalidColumnLabel(label) => write!(f, "invalid column label: {label}"),
        }
    }
}

impl std::error::Error for AddressError {}

pub fn col_label_to_index(label: &str) -> Result<u16, AddressError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(AddressError::InvalidColumnLabel(label.to_string()));
    }
    let mut value: u32 = 0;
    for ch in trimmed.chars() {
        if !ch.is_ascii_alphabetic() {
            return Err(AddressError::InvalidColumnLabel(label.to_string()));
        }
        let upper = ch.to_ascii_uppercase();
        let digit = u32::from(upper as u8 - b'A' + 1);
        value = value
            .checked_mul(26)
            .and_then(|v| v.checked_add(digit))
            .ok_or_else(|| AddressError::InvalidColumnLabel(label.to_string()))?;
    }
    u16::try_from(value).map_err(|_| AddressError::InvalidColumnLabel(label.to_string()))
}

pub fn col_index_to_label(index: u16) -> String {
    if index == 0 {
        return String::new();
    }
    let mut value = u32::from(index);
    let mut chars: Vec<char> = Vec::new();
    while value > 0 {
        let rem = ((value - 1) % 26) as u8;
        chars.push(char::from(b'A' + rem));
        value = (value - 1) / 26;
    }
    chars.reverse();
    chars.into_iter().collect()
}

pub fn parse_cell_ref(input: &str, bounds: SheetBounds) -> Result<CellRef, AddressError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AddressError::Empty);
    }

    let split_idx = trimmed
        .char_indices()
        .find(|(_, c)| c.is_ascii_digit())
        .map(|(idx, _)| idx)
        .ok_or_else(|| AddressError::InvalidFormat(trimmed.to_string()))?;

    if split_idx == 0 {
        return Err(AddressError::InvalidFormat(trimmed.to_string()));
    }

    let (col_label, row_part) = trimmed.split_at(split_idx);
    if !col_label.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(AddressError::InvalidFormat(trimmed.to_string()));
    }
    if !row_part.chars().all(|c| c.is_ascii_digit()) {
        return Err(AddressError::InvalidFormat(trimmed.to_string()));
    }

    let col = col_label_to_index(col_label)?;
    let row = row_part
        .parse::<u16>()
        .map_err(|_| AddressError::InvalidFormat(trimmed.to_string()))?;

    CellRef::new(col, row, bounds)
}

pub fn is_cell_reference_token(input: &str) -> bool {
    let mut seen_letter = false;
    let mut seen_digit = false;
    let mut in_digits = false;

    for c in input.chars() {
        if c.is_ascii_alphabetic() {
            if in_digits {
                return false;
            }
            seen_letter = true;
        } else if c.is_ascii_digit() {
            in_digits = true;
            seen_digit = true;
        } else {
            return false;
        }
    }
    seen_letter && seen_digit
}
