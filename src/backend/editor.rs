use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

/// Represents an edit command that can be undone/redone
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EditCommand {
    /// Set a cell value (row, col, old_value, new_value)
    SetCell {
        row: usize,
        col: usize,
        old_value: String,
        new_value: String,
    },
    /// Insert a row at position with data
    InsertRow {
        at: usize,
        data: Vec<String>,
    },
    /// Delete a row at position (stores data for undo)
    DeleteRow {
        at: usize,
        data: Vec<String>,
    },
    /// Insert a column at position with header
    InsertColumn {
        at: usize,
        header: String,
    },
    /// Delete a column at position (stores header and column data for undo)
    DeleteColumn {
        at: usize,
        header: String,
        data: Vec<String>, // Value at each row for this column
    },
    /// Set a header value
    SetHeader {
        col: usize,
        old_value: String,
        new_value: String,
    },
}

impl EditCommand {
    /// Create the inverse command for undo
    pub fn inverse(&self) -> Self {
        match self.clone() {
            EditCommand::SetCell { row, col, old_value, new_value } => {
                EditCommand::SetCell { row, col, old_value: new_value, new_value: old_value }
            }
            EditCommand::InsertRow { at, data } => {
                EditCommand::DeleteRow { at, data }
            }
            EditCommand::DeleteRow { at, data } => {
                EditCommand::InsertRow { at, data }
            }
            EditCommand::InsertColumn { at, header } => {
                EditCommand::DeleteColumn { at, header, data: Vec::new() }
            }
            EditCommand::DeleteColumn { at, header, data: _ } => {
                EditCommand::InsertColumn { at, header }
            }
            EditCommand::SetHeader { col, old_value, new_value } => {
                EditCommand::SetHeader { col, old_value: new_value, new_value: old_value }
            }
        }
    }
}

/// Delta buffer that tracks edits with full undo/redo support
#[derive(Default, Clone, Debug)]
pub struct DeltaBuffer {
    /// Current cell edits: (row, col) -> value
    edits: BTreeMap<(usize, usize), String>,
    /// Undo stack - commands that have been executed
    undo_stack: Vec<EditCommand>,
    /// Redo stack - commands that have been undone
    redo_stack: Vec<EditCommand>,
    /// Dirty flag - true if there are unsaved changes
    dirty: bool,
    /// Maximum undo history size
    max_history: usize,
}

impl DeltaBuffer {
    pub fn new() -> Self {
        Self {
            edits: BTreeMap::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            dirty: false,
            max_history: 100,
        }
    }

    /// Execute a command and add to undo stack
    pub fn execute(&mut self, cmd: EditCommand) {
        // Apply the command to our edit map
        self.apply_command(&cmd);
        
        // Add to undo stack
        self.undo_stack.push(cmd);
        
        // Clear redo stack (new action breaks redo chain)
        self.redo_stack.clear();
        
        // Trim history if needed
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.remove(0);
        }
        
        self.dirty = true;
    }

    /// Undo the last command
    pub fn undo(&mut self) -> Option<EditCommand> {
        if let Some(cmd) = self.undo_stack.pop() {
            let inverse = cmd.inverse();
            self.apply_command(&inverse);
            self.redo_stack.push(cmd.clone());
            self.dirty = !self.undo_stack.is_empty();
            Some(cmd)
        } else {
            None
        }
    }

    /// Redo the last undone command
    pub fn redo(&mut self) -> Option<EditCommand> {
        if let Some(cmd) = self.redo_stack.pop() {
            self.apply_command(&cmd);
            self.undo_stack.push(cmd.clone());
            self.dirty = true;
            Some(cmd)
        } else {
            None
        }
    }

    /// Apply a command to the edit map
    fn apply_command(&mut self, cmd: &EditCommand) {
        match cmd {
            EditCommand::SetCell { row, col, new_value, .. } => {
                if new_value.is_empty() {
                    self.edits.remove(&(*row, *col));
                } else {
                    self.edits.insert((*row, *col), new_value.clone());
                }
            }
            EditCommand::SetHeader { .. } => {
                // Headers are handled at the grid level
            }
            EditCommand::InsertRow { .. } |
            EditCommand::DeleteRow { .. } |
            EditCommand::InsertColumn { .. } |
            EditCommand::DeleteColumn { .. } => {
                // Row/column operations are handled at the grid level
                // The DeltaBuffer just tracks the command history
            }
        }
    }

    /// Add an edit (convenience method that creates SetCell command)
    pub fn add_edit(&mut self, row: usize, col: usize, old_value: String, new_value: String) {
        let cmd = EditCommand::SetCell { row, col, old_value, new_value };
        self.execute(cmd);
    }

    /// Get an edit for a specific cell
    pub fn get_edit(&self, row: usize, col: usize) -> Option<&String> {
        self.edits.get(&(row, col))
    }

    /// Check if there are changes that can be undone
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Check if there are changes that can be redone
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Get the number of undo steps available
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Get the number of redo steps available
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Check if there are unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark as saved (clears dirty flag)
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Clear all edits and history
    pub fn clear(&mut self) {
        self.edits.clear();
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.dirty = false;
    }
}

// Keep backwards compatibility with old EditBuffer name
pub type EditBuffer = DeltaBuffer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undo_redo() {
        let mut buffer = DeltaBuffer::new();
        
        // Edit cell
        buffer.add_edit(0, 0, "old".to_string(), "new".to_string());
        assert_eq!(buffer.get_edit(0, 0), Some(&"new".to_string()));
        assert!(buffer.can_undo());
        assert!(!buffer.can_redo());
        
        // Undo
        buffer.undo();
        assert_eq!(buffer.get_edit(0, 0), None);
        assert!(!buffer.can_undo());
        assert!(buffer.can_redo());
        
        // Redo
        buffer.redo();
        assert_eq!(buffer.get_edit(0, 0), Some(&"new".to_string()));
        assert!(buffer.can_undo());
        assert!(!buffer.can_redo());
    }

    #[test]
    fn test_new_edit_clears_redo() {
        let mut buffer = DeltaBuffer::new();
        
        buffer.add_edit(0, 0, "".to_string(), "first".to_string());
        buffer.undo();
        assert!(buffer.can_redo());
        
        // New edit should clear redo stack
        buffer.add_edit(0, 1, "".to_string(), "second".to_string());
        assert!(!buffer.can_redo());
    }
}
