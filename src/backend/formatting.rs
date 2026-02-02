use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cell formatting information
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CellFormat {
    pub bg_color: Option<[u8; 4]>,   // RGBA
    pub text_color: Option<[u8; 4]>, // RGBA
    pub bold: bool,
    pub italic: bool,
}

/// Container for all cell formatting
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct FormatMap {
    cells: HashMap<(usize, usize), CellFormat>,
}

impl FormatMap {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    pub fn get(&self, row: usize, col: usize) -> Option<&CellFormat> {
        self.cells.get(&(row, col))
    }

    pub fn set(&mut self, row: usize, col: usize, format: CellFormat) {
        self.cells.insert((row, col), format);
    }

    pub fn remove(&mut self, row: usize, col: usize) {
        self.cells.remove(&(row, col));
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Shift all row indices after a deleted row
    pub fn shift_rows_up(&mut self, deleted_row: usize) {
        let mut new_cells = HashMap::new();
        for ((row, col), fmt) in self.cells.drain() {
            if row > deleted_row {
                new_cells.insert((row - 1, col), fmt);
            } else if row < deleted_row {
                new_cells.insert((row, col), fmt);
            }
            // Skip the deleted row
        }
        self.cells = new_cells;
    }

    /// Shift all row indices after an inserted row
    pub fn shift_rows_down(&mut self, inserted_row: usize) {
        let mut new_cells = HashMap::new();
        for ((row, col), fmt) in self.cells.drain() {
            if row >= inserted_row {
                new_cells.insert((row + 1, col), fmt);
            } else {
                new_cells.insert((row, col), fmt);
            }
        }
        self.cells = new_cells;
    }

    /// Shift all column indices after a deleted column
    pub fn shift_cols_left(&mut self, deleted_col: usize) {
        let mut new_cells = HashMap::new();
        for ((row, col), fmt) in self.cells.drain() {
            if col > deleted_col {
                new_cells.insert((row, col - 1), fmt);
            } else if col < deleted_col {
                new_cells.insert((row, col), fmt);
            }
        }
        self.cells = new_cells;
    }

    /// Shift all column indices after an inserted column
    pub fn shift_cols_right(&mut self, inserted_col: usize) {
        let mut new_cells = HashMap::new();
        for ((row, col), fmt) in self.cells.drain() {
            if col >= inserted_col {
                new_cells.insert((row, col + 1), fmt);
            } else {
                new_cells.insert((row, col), fmt);
            }
        }
        self.cells = new_cells;
    }
}

impl CellFormat {
    pub fn with_bg(color: [u8; 4]) -> Self {
        Self {
            bg_color: Some(color),
            ..Default::default()
        }
    }

    pub fn with_text_color(color: [u8; 4]) -> Self {
        Self {
            text_color: Some(color),
            ..Default::default()
        }
    }

    pub fn bold() -> Self {
        Self {
            bold: true,
            ..Default::default()
        }
    }
}
