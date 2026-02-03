use serde::{Deserialize, Serialize};
use crate::backend::editor::EditCommand;

/// An in-memory editable grid for CSV data with undo/redo support
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditableGrid {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[serde(skip)]
    undo_stack: Vec<EditCommand>,
    #[serde(skip)]
    redo_stack: Vec<EditCommand>,
    modified: bool,
}

impl EditableGrid {
    /// Create an empty grid with specified dimensions
    pub fn new(cols: usize, rows: usize) -> Self {
        let headers = (0..cols)
            .map(|i| format!("Column {}", i + 1))
            .collect();
        let row_data = vec![vec![String::new(); cols]; rows];
        Self {
            headers,
            rows: row_data,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            modified: false,
        }
    }

    /// Create from CSV text
    pub fn from_csv(csv_text: &str) -> Self {
        let mut lines = csv_text.lines();
        
        let headers = lines
            .next()
            .map(|h| Self::parse_csv_row(h))
            .unwrap_or_default();
        
        let rows: Vec<Vec<String>> = lines
            .map(|line| Self::parse_csv_row(line))
            .collect();
        
        Self {
            headers,
            rows,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            modified: false,
        }
    }

    /// Simple CSV row parser (handles basic quoting)
    fn parse_csv_row(line: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;
        let mut chars = line.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '"' if !in_quotes => {
                    in_quotes = true;
                }
                '"' if in_quotes => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        current.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                ',' if !in_quotes => {
                    fields.push(current.trim().to_string());
                    current = String::new();
                }
                _ => {
                    current.push(c);
                }
            }
        }
        fields.push(current.trim().to_string());
        fields
    }

    /// Convert to CSV text
    pub fn to_csv(&self) -> String {
        let mut output = String::new();
        
        // Headers
        output.push_str(&self.row_to_csv(&self.headers));
        output.push('\n');
        
        // Data rows
        for row in &self.rows {
            output.push_str(&self.row_to_csv(row));
            output.push('\n');
        }
        
        output
    }

    fn row_to_csv(&self, row: &[String]) -> String {
        row.iter()
            .map(|cell| {
                if cell.contains(',') || cell.contains('"') || cell.contains('\n') {
                    format!("\"{}\"", cell.replace('"', "\"\""))
                } else {
                    cell.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(",")
    }

    // ---- Editing operations ----

    pub fn num_cols(&self) -> usize {
        self.headers.len()
    }

    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    pub fn get_cell(&self, row: usize, col: usize) -> Option<&String> {
        self.rows.get(row).and_then(|r| r.get(col))
    }

    pub fn set_cell(&mut self, row: usize, col: usize, value: String) {
        if let Some(r) = self.rows.get_mut(row) {
            if let Some(cell) = r.get_mut(col) {
                let old_value = std::mem::replace(cell, value.clone());
                let cmd = EditCommand::SetCell { row, col, old_value, new_value: value };
                self.push_undo(cmd);
                self.modified = true;
            }
        }
    }

    pub fn get_header(&self, col: usize) -> Option<&String> {
        self.headers.get(col)
    }

    pub fn set_header(&mut self, col: usize, name: String) {
        if let Some(h) = self.headers.get_mut(col) {
            let old_value = std::mem::replace(h, name.clone());
            let cmd = EditCommand::SetHeader { col, old_value, new_value: name };
            self.push_undo(cmd);
            self.modified = true;
        }
    }

    pub fn add_row(&mut self, after_row: Option<usize>) {
        let new_row = vec![String::new(); self.num_cols()];
        let insert_at = match after_row {
            Some(idx) if idx < self.rows.len() => {
                self.rows.insert(idx + 1, new_row.clone());
                idx + 1
            }
            _ => {
                self.rows.push(new_row.clone());
                self.rows.len() - 1
            }
        };
        let cmd = EditCommand::InsertRow { at: insert_at, data: new_row };
        self.push_undo(cmd);
        self.modified = true;
    }

    pub fn delete_row(&mut self, row: usize) {
        if row < self.rows.len() {
            let data = self.rows.remove(row);
            let cmd = EditCommand::DeleteRow { at: row, data };
            self.push_undo(cmd);
            self.modified = true;
        }
    }

    pub fn add_column(&mut self, after_col: Option<usize>) {
        let insert_pos = after_col.map(|c| c + 1).unwrap_or(self.num_cols());
        let header = format!("Column {}", self.num_cols() + 1);
        
        self.headers.insert(insert_pos, header.clone());
        
        for row in &mut self.rows {
            row.insert(insert_pos, String::new());
        }
        
        let cmd = EditCommand::InsertColumn { at: insert_pos, header };
        self.push_undo(cmd);
        self.modified = true;
    }

    pub fn delete_column(&mut self, col: usize) {
        if col < self.num_cols() {
            let header = self.headers.remove(col);
            let mut data = Vec::new();
            for row in &mut self.rows {
                if col < row.len() {
                    data.push(row.remove(col));
                }
            }
            let cmd = EditCommand::DeleteColumn { at: col, header, data };
            self.push_undo(cmd);
            self.modified = true;
        }
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    // ---- Undo/Redo Support ----

    fn push_undo(&mut self, cmd: EditCommand) {
        self.undo_stack.push(cmd);
        self.redo_stack.clear(); // New action clears redo
        
        // Limit history to 100 items
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self) -> bool {
        if let Some(cmd) = self.undo_stack.pop() {
            self.apply_inverse(&cmd);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(cmd) = self.redo_stack.pop() {
            self.apply_command(&cmd);
            self.undo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    fn apply_command(&mut self, cmd: &EditCommand) {
        match cmd {
            EditCommand::SetCell { row, col, new_value, .. } => {
                if let Some(r) = self.rows.get_mut(*row) {
                    if let Some(cell) = r.get_mut(*col) {
                        *cell = new_value.clone();
                    }
                }
            }
            EditCommand::SetHeader { col, new_value, .. } => {
                if let Some(h) = self.headers.get_mut(*col) {
                    *h = new_value.clone();
                }
            }
            EditCommand::InsertRow { at, data } => {
                if *at <= self.rows.len() {
                    self.rows.insert(*at, data.clone());
                }
            }
            EditCommand::DeleteRow { at, .. } => {
                if *at < self.rows.len() {
                    self.rows.remove(*at);
                }
            }
            EditCommand::InsertColumn { at, header } => {
                if *at <= self.headers.len() {
                    self.headers.insert(*at, header.clone());
                    for row in &mut self.rows {
                        row.insert(*at, String::new());
                    }
                }
            }
            EditCommand::DeleteColumn { at, .. } => {
                if *at < self.headers.len() {
                    self.headers.remove(*at);
                    for row in &mut self.rows {
                        if *at < row.len() {
                            row.remove(*at);
                        }
                    }
                }
            }
        }
    }

    fn apply_inverse(&mut self, cmd: &EditCommand) {
        match cmd {
            EditCommand::SetCell { row, col, old_value, .. } => {
                if let Some(r) = self.rows.get_mut(*row) {
                    if let Some(cell) = r.get_mut(*col) {
                        *cell = old_value.clone();
                    }
                }
            }
            EditCommand::SetHeader { col, old_value, .. } => {
                if let Some(h) = self.headers.get_mut(*col) {
                    *h = old_value.clone();
                }
            }
            EditCommand::InsertRow { at, .. } => {
                if *at < self.rows.len() {
                    self.rows.remove(*at);
                }
            }
            EditCommand::DeleteRow { at, data } => {
                if *at <= self.rows.len() {
                    self.rows.insert(*at, data.clone());
                }
            }
            EditCommand::InsertColumn { at, .. } => {
                if *at < self.headers.len() {
                    self.headers.remove(*at);
                    for row in &mut self.rows {
                        if *at < row.len() {
                            row.remove(*at);
                        }
                    }
                }
            }
            EditCommand::DeleteColumn { at, header, data } => {
                if *at <= self.headers.len() {
                    self.headers.insert(*at, header.clone());
                    for (i, row) in self.rows.iter_mut().enumerate() {
                        let val = data.get(i).cloned().unwrap_or_default();
                        row.insert(*at, val);
                    }
                }
            }
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for EditableGrid {
    fn default() -> Self {
        Self::new(3, 10)
    }
}
