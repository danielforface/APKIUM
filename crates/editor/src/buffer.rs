//! Text Buffer
//! 
//! High-performance text buffer using rope data structure
//! for efficient insertions, deletions, and large file handling.

use ropey::{Rope, RopeSlice};
use std::ops::Range;
use std::path::PathBuf;

use crate::cursor::Cursor;
use crate::selection::Selection;

/// Edit operation for undo/redo
#[derive(Debug, Clone)]
pub enum EditOperation {
    Insert {
        position: usize,
        text: String,
    },
    Delete {
        position: usize,
        text: String,
    },
    Replace {
        position: usize,
        old_text: String,
        new_text: String,
    },
}

impl EditOperation {
    /// Get the inverse operation for undo
    pub fn inverse(&self) -> Self {
        match self {
            EditOperation::Insert { position, text } => EditOperation::Delete {
                position: *position,
                text: text.clone(),
            },
            EditOperation::Delete { position, text } => EditOperation::Insert {
                position: *position,
                text: text.clone(),
            },
            EditOperation::Replace { position, old_text, new_text } => EditOperation::Replace {
                position: *position,
                old_text: new_text.clone(),
                new_text: old_text.clone(),
            },
        }
    }
}

/// Text position (line, column)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// High-performance text buffer
pub struct TextBuffer {
    /// The rope containing the text
    rope: Rope,
    /// File path (if loaded from file)
    path: Option<PathBuf>,
    /// Whether the buffer has been modified
    dirty: bool,
    /// Undo stack
    undo_stack: Vec<EditOperation>,
    /// Redo stack
    redo_stack: Vec<EditOperation>,
    /// Maximum undo history size
    max_undo_history: usize,
    /// Primary cursor
    cursor: Cursor,
    /// Current selection
    selection: Option<Selection>,
    /// Tab width
    tab_width: usize,
    /// Use soft tabs (spaces)
    soft_tabs: bool,
}

impl TextBuffer {
    /// Create a new empty buffer
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            path: None,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_history: 1000,
            cursor: Cursor::default(),
            selection: None,
            tab_width: 4,
            soft_tabs: true,
        }
    }

    /// Create a buffer from a string
    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            path: None,
            dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_undo_history: 1000,
            cursor: Cursor::default(),
            selection: None,
            tab_width: 4,
            soft_tabs: true,
        }
    }

    /// Load a buffer from a file
    pub async fn from_file(path: PathBuf) -> anyhow::Result<Self> {
        let content = tokio::fs::read_to_string(&path).await?;
        let mut buffer = Self::from_str(&content);
        buffer.path = Some(path);
        buffer.dirty = false;
        Ok(buffer)
    }

    /// Save the buffer to its file
    pub async fn save(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.path {
            tokio::fs::write(path, self.rope.to_string()).await?;
            self.dirty = false;
            Ok(())
        } else {
            Err(anyhow::anyhow!("No file path set"))
        }
    }

    /// Save the buffer to a new file
    pub async fn save_as(&mut self, path: PathBuf) -> anyhow::Result<()> {
        tokio::fs::write(&path, self.rope.to_string()).await?;
        self.path = Some(path);
        self.dirty = false;
        Ok(())
    }

    /// Get the full text content
    pub fn text(&self) -> String {
        self.rope.to_string()
    }

    /// Get a slice of the text
    pub fn slice(&self, range: Range<usize>) -> RopeSlice {
        self.rope.slice(range)
    }

    /// Get a line by index
    pub fn line(&self, line_idx: usize) -> Option<RopeSlice> {
        if line_idx < self.rope.len_lines() {
            Some(self.rope.line(line_idx))
        } else {
            None
        }
    }

    /// Get line content as string
    pub fn line_str(&self, line_idx: usize) -> Option<String> {
        self.line(line_idx).map(|l| l.to_string())
    }

    /// Get the number of lines
    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    /// Get the total character count
    pub fn char_count(&self) -> usize {
        self.rope.len_chars()
    }

    /// Get the byte length
    pub fn byte_len(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Check if the buffer has been modified
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get the file path
    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    /// Convert a position to a char index
    pub fn position_to_char(&self, pos: Position) -> usize {
        if pos.line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(pos.line);
        let line_len = self.rope.line(pos.line).len_chars();
        line_start + pos.column.min(line_len)
    }

    /// Convert a char index to a position
    pub fn char_to_position(&self, char_idx: usize) -> Position {
        let line = self.rope.char_to_line(char_idx);
        let line_start = self.rope.line_to_char(line);
        let column = char_idx - line_start;
        Position { line, column }
    }

    /// Insert text at a position
    pub fn insert(&mut self, pos: Position, text: &str) {
        let char_idx = self.position_to_char(pos);
        self.insert_at_char(char_idx, text);
    }

    /// Insert text at a char index
    pub fn insert_at_char(&mut self, char_idx: usize, text: &str) {
        let char_idx = char_idx.min(self.rope.len_chars());
        
        // Record operation for undo
        let op = EditOperation::Insert {
            position: char_idx,
            text: text.to_string(),
        };
        self.push_undo(op);
        
        // Perform insert
        self.rope.insert(char_idx, text);
        self.dirty = true;
        
        // Move cursor
        self.cursor.move_to_char(char_idx + text.len());
    }

    /// Delete text in a range
    pub fn delete(&mut self, range: Range<usize>) {
        if range.start >= range.end || range.start >= self.rope.len_chars() {
            return;
        }
        
        let end = range.end.min(self.rope.len_chars());
        let deleted_text = self.rope.slice(range.start..end).to_string();
        
        // Record operation for undo
        let op = EditOperation::Delete {
            position: range.start,
            text: deleted_text,
        };
        self.push_undo(op);
        
        // Perform delete
        self.rope.remove(range.start..end);
        self.dirty = true;
        
        // Move cursor
        self.cursor.move_to_char(range.start);
    }

    /// Delete a line
    pub fn delete_line(&mut self, line_idx: usize) {
        if line_idx >= self.rope.len_lines() {
            return;
        }
        
        let start = self.rope.line_to_char(line_idx);
        let end = if line_idx + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line_idx + 1)
        } else {
            self.rope.len_chars()
        };
        
        self.delete(start..end);
    }

    /// Replace text in a range
    pub fn replace(&mut self, range: Range<usize>, text: &str) {
        if range.start > self.rope.len_chars() {
            return;
        }
        
        let end = range.end.min(self.rope.len_chars());
        let old_text = self.rope.slice(range.start..end).to_string();
        
        // Record operation for undo
        let op = EditOperation::Replace {
            position: range.start,
            old_text,
            new_text: text.to_string(),
        };
        self.push_undo(op);
        
        // Perform replace
        self.rope.remove(range.start..end);
        self.rope.insert(range.start, text);
        self.dirty = true;
        
        // Move cursor
        self.cursor.move_to_char(range.start + text.len());
    }

    /// Push an operation to the undo stack
    fn push_undo(&mut self, op: EditOperation) {
        self.undo_stack.push(op);
        self.redo_stack.clear();
        
        // Trim undo history if needed
        if self.undo_stack.len() > self.max_undo_history {
            self.undo_stack.remove(0);
        }
    }

    /// Undo the last operation
    pub fn undo(&mut self) -> bool {
        if let Some(op) = self.undo_stack.pop() {
            self.apply_operation(&op.inverse());
            self.redo_stack.push(op);
            true
        } else {
            false
        }
    }

    /// Redo the last undone operation
    pub fn redo(&mut self) -> bool {
        if let Some(op) = self.redo_stack.pop() {
            self.apply_operation(&op);
            self.undo_stack.push(op.inverse());
            true
        } else {
            false
        }
    }

    /// Apply an edit operation without recording it
    fn apply_operation(&mut self, op: &EditOperation) {
        match op {
            EditOperation::Insert { position, text } => {
                let pos = (*position).min(self.rope.len_chars());
                self.rope.insert(pos, text);
            }
            EditOperation::Delete { position, text } => {
                let pos = (*position).min(self.rope.len_chars());
                let end = (pos + text.len()).min(self.rope.len_chars());
                self.rope.remove(pos..end);
            }
            EditOperation::Replace { position, old_text, new_text } => {
                let pos = (*position).min(self.rope.len_chars());
                let end = (pos + old_text.len()).min(self.rope.len_chars());
                self.rope.remove(pos..end);
                self.rope.insert(pos, new_text);
            }
        }
        self.dirty = true;
    }

    /// Get the cursor
    pub fn cursor(&self) -> &Cursor {
        &self.cursor
    }

    /// Get mutable cursor
    pub fn cursor_mut(&mut self) -> &mut Cursor {
        &mut self.cursor
    }

    /// Get the current selection
    pub fn selection(&self) -> Option<&Selection> {
        self.selection.as_ref()
    }

    /// Set the selection
    pub fn set_selection(&mut self, selection: Option<Selection>) {
        self.selection = selection;
    }

    /// Get selected text
    pub fn selected_text(&self) -> Option<String> {
        self.selection.as_ref().map(|sel| {
            let start = self.position_to_char(sel.start);
            let end = self.position_to_char(sel.end);
            self.rope.slice(start..end).to_string()
        })
    }

    /// Delete the current selection
    pub fn delete_selection(&mut self) -> bool {
        if let Some(sel) = self.selection.take() {
            let start = self.position_to_char(sel.start);
            let end = self.position_to_char(sel.end);
            self.delete(start..end);
            true
        } else {
            false
        }
    }

    /// Insert a tab (or spaces)
    pub fn insert_tab(&mut self) {
        let text = if self.soft_tabs {
            " ".repeat(self.tab_width)
        } else {
            "\t".to_string()
        };
        let pos = self.cursor.position();
        self.insert(pos, &text);
    }

    /// Insert a newline with auto-indent
    pub fn insert_newline(&mut self) {
        let pos = self.cursor.position();
        let current_line = self.line_str(pos.line).unwrap_or_default();
        
        // Calculate indent of current line
        let indent: String = current_line
            .chars()
            .take_while(|c| c.is_whitespace())
            .collect();
        
        let text = format!("\n{}", indent);
        self.insert(pos, &text);
    }

    /// Find text in the buffer
    pub fn find(&self, query: &str, case_sensitive: bool) -> Vec<Range<usize>> {
        let text = self.rope.to_string();
        let search_text = if case_sensitive {
            text.clone()
        } else {
            text.to_lowercase()
        };
        let query = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        
        let mut results = Vec::new();
        let mut start = 0;
        
        while let Some(pos) = search_text[start..].find(&query) {
            let abs_pos = start + pos;
            results.push(abs_pos..abs_pos + query.len());
            start = abs_pos + 1;
        }
        
        results
    }

    /// Find and replace
    pub fn find_replace(&mut self, query: &str, replacement: &str, case_sensitive: bool) -> usize {
        let matches = self.find(query, case_sensitive);
        let count = matches.len();
        
        // Replace in reverse order to preserve positions
        for range in matches.into_iter().rev() {
            self.replace(range, replacement);
        }
        
        count
    }
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = TextBuffer::from_str("Hello, World!");
        assert_eq!(buffer.text(), "Hello, World!");
        assert_eq!(buffer.line_count(), 1);
        assert_eq!(buffer.char_count(), 13);
    }

    #[test]
    fn test_insert() {
        let mut buffer = TextBuffer::from_str("Hello, World!");
        buffer.insert(Position::new(0, 7), "Beautiful ");
        assert_eq!(buffer.text(), "Hello, Beautiful World!");
    }

    #[test]
    fn test_delete() {
        let mut buffer = TextBuffer::from_str("Hello, World!");
        buffer.delete(0..7);
        assert_eq!(buffer.text(), "World!");
    }

    #[test]
    fn test_undo_redo() {
        let mut buffer = TextBuffer::from_str("Hello");
        buffer.insert(Position::new(0, 5), " World");
        assert_eq!(buffer.text(), "Hello World");
        
        buffer.undo();
        assert_eq!(buffer.text(), "Hello");
        
        buffer.redo();
        assert_eq!(buffer.text(), "Hello World");
    }

    #[test]
    fn test_find() {
        let buffer = TextBuffer::from_str("Hello World, Hello Universe");
        let results = buffer.find("Hello", true);
        assert_eq!(results.len(), 2);
    }
}
