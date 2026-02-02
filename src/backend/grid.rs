use serde::{Deserialize, Serialize};

/// An in-memory editable grid for CSV data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EditableGrid {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
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
                *cell = value;
                self.modified = true;
            }
        }
    }

    pub fn get_header(&self, col: usize) -> Option<&String> {
        self.headers.get(col)
    }

    pub fn set_header(&mut self, col: usize, name: String) {
        if let Some(h) = self.headers.get_mut(col) {
            *h = name;
            self.modified = true;
        }
    }

    pub fn add_row(&mut self, after_row: Option<usize>) {
        let new_row = vec![String::new(); self.num_cols()];
        match after_row {
            Some(idx) if idx < self.rows.len() => {
                self.rows.insert(idx + 1, new_row);
            }
            _ => {
                self.rows.push(new_row);
            }
        }
        self.modified = true;
    }

    pub fn delete_row(&mut self, row: usize) {
        if row < self.rows.len() {
            self.rows.remove(row);
            self.modified = true;
        }
    }

    pub fn add_column(&mut self, after_col: Option<usize>) {
        let insert_pos = after_col.map(|c| c + 1).unwrap_or(self.num_cols());
        
        self.headers.insert(insert_pos, format!("Column {}", self.num_cols() + 1));
        
        for row in &mut self.rows {
            row.insert(insert_pos, String::new());
        }
        self.modified = true;
    }

    pub fn delete_column(&mut self, col: usize) {
        if col < self.num_cols() {
            self.headers.remove(col);
            for row in &mut self.rows {
                if col < row.len() {
                    row.remove(col);
                }
            }
            self.modified = true;
        }
    }

    pub fn is_modified(&self) -> bool {
        self.modified
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }
}

impl Default for EditableGrid {
    fn default() -> Self {
        Self::new(3, 10)
    }
}
