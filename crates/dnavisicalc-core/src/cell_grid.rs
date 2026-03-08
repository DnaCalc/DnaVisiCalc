use crate::address::CellRef;

/// A flat array indexed by `CellRef` for O(1) cell-keyed lookups.
///
/// Replaces `HashMap<CellRef, T>` on hot paths where every cell in the
/// 63×254 grid is a valid key. Index: `(col-1) * max_rows + (row-1)`.
pub struct CellGrid<T> {
    data: Vec<Option<T>>,
    max_cols: u16,
    max_rows: u16,
    len: usize,
}

impl<T> CellGrid<T> {
    pub fn new(max_cols: u16, max_rows: u16) -> Self {
        let size = max_cols as usize * max_rows as usize;
        let mut data = Vec::with_capacity(size);
        data.resize_with(size, || None);
        Self {
            data,
            max_cols,
            max_rows,
            len: 0,
        }
    }

    #[inline(always)]
    fn index(&self, cell: &CellRef) -> usize {
        (cell.col as usize - 1) * self.max_rows as usize + (cell.row as usize - 1)
    }

    #[inline]
    pub fn get(&self, cell: &CellRef) -> Option<&T> {
        self.data.get(self.index(cell))?.as_ref()
    }

    #[inline]
    pub fn get_mut(&mut self, cell: &CellRef) -> Option<&mut T> {
        let idx = self.index(cell);
        self.data.get_mut(idx)?.as_mut()
    }

    #[inline]
    pub fn insert(&mut self, cell: CellRef, value: T) -> Option<T> {
        let idx = self.index(&cell);
        let slot = &mut self.data[idx];
        let old = slot.take();
        if old.is_none() {
            self.len += 1;
        }
        *slot = Some(value);
        old
    }

    #[inline]
    pub fn remove(&mut self, cell: &CellRef) -> Option<T> {
        let idx = self.index(cell);
        let old = self.data[idx].take();
        if old.is_some() {
            self.len -= 1;
        }
        old
    }

    #[inline]
    pub fn contains_key(&self, cell: &CellRef) -> bool {
        self.data
            .get(self.index(cell))
            .is_some_and(|slot| slot.is_some())
    }

    pub fn clear(&mut self) {
        for slot in &mut self.data {
            *slot = None;
        }
        self.len = 0;
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (CellRef, &T)> {
        let max_rows = self.max_rows;
        self.data.iter().enumerate().filter_map(move |(idx, slot)| {
            let value = slot.as_ref()?;
            let col = (idx / max_rows as usize) as u16 + 1;
            let row = (idx % max_rows as usize) as u16 + 1;
            Some((CellRef { col, row }, value))
        })
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.data.iter().filter_map(|slot| slot.as_ref())
    }

    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.data.iter_mut().filter_map(|slot| slot.as_mut())
    }

    pub fn keys(&self) -> impl Iterator<Item = CellRef> + '_ {
        let max_rows = self.max_rows;
        self.data.iter().enumerate().filter_map(move |(idx, slot)| {
            slot.as_ref()?;
            let col = (idx / max_rows as usize) as u16 + 1;
            let row = (idx % max_rows as usize) as u16 + 1;
            Some(CellRef { col, row })
        })
    }
}

impl<T: Clone> CellGrid<T> {
    pub fn clone_grid(&self) -> Self {
        Self {
            data: self.data.clone(),
            max_cols: self.max_cols,
            max_rows: self.max_rows,
            len: self.len,
        }
    }
}

impl<T: Clone> Clone for CellGrid<T> {
    fn clone(&self) -> Self {
        self.clone_grid()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for CellGrid<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellGrid")
            .field("max_cols", &self.max_cols)
            .field("max_rows", &self.max_rows)
            .field("len", &self.len)
            .finish()
    }
}

impl<T> Default for CellGrid<T> {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            max_cols: 0,
            max_rows: 0,
            len: 0,
        }
    }
}

/// A flat boolean array indexed by `CellRef` for O(1) set membership tests.
///
/// Replaces `HashSet<CellRef>` when used for contains/insert/remove on
/// cell coordinates bounded by the grid dimensions.
pub struct CellBitset {
    bits: Vec<bool>,
    max_rows: u16,
}

impl CellBitset {
    pub fn new(max_cols: u16, max_rows: u16) -> Self {
        let size = max_cols as usize * max_rows as usize;
        Self {
            bits: vec![false; size],
            max_rows,
        }
    }

    #[inline(always)]
    fn index(&self, cell: &CellRef) -> usize {
        (cell.col as usize - 1) * self.max_rows as usize + (cell.row as usize - 1)
    }

    #[inline]
    pub fn insert(&mut self, cell: CellRef) -> bool {
        let idx = self.index(&cell);
        let was_set = self.bits[idx];
        self.bits[idx] = true;
        !was_set
    }

    #[inline]
    pub fn contains(&self, cell: &CellRef) -> bool {
        self.bits[self.index(cell)]
    }

    #[inline]
    pub fn remove(&mut self, cell: &CellRef) -> bool {
        let idx = self.index(cell);
        let was_set = self.bits[idx];
        self.bits[idx] = false;
        was_set
    }

    pub fn clear(&mut self) {
        for bit in &mut self.bits {
            *bit = false;
        }
    }
}

impl Default for CellBitset {
    fn default() -> Self {
        Self {
            bits: Vec::new(),
            max_rows: 0,
        }
    }
}

impl std::fmt::Debug for CellBitset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CellBitset")
            .field("size", &self.bits.len())
            .finish()
    }
}
