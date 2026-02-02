use std::collections::HashMap;

#[derive(Default)]
pub struct EditBuffer {
    /// Maps (Row Index, Col Index) -> New Content
    edits: HashMap<(usize, usize), String>,
    // Maps Row Index -> Whole Line Replacement (if needed, but cell based is safer for CSV)
    // Actually, just cell edits for now.
}

impl EditBuffer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_edit(&mut self, row: usize, col: usize, content: String) {
        self.edits.insert((row, col), content);
    }

    pub fn get_edit(&self, row: usize, col: usize) -> Option<&String> {
        self.edits.get(&(row, col))
    }

    pub fn clear(&mut self) {
        self.edits.clear();
    }
}
