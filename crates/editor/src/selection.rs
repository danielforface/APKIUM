//! Selection Management
//! 
//! Handles text selection and multi-selection operations.

use crate::buffer::Position;

/// Selection anchor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    /// Normal character selection
    Character,
    /// Word selection (double-click)
    Word,
    /// Line selection (triple-click)
    Line,
    /// Block/column selection (Alt+drag)
    Block,
}

/// A text selection range
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// Start position (anchor)
    pub start: Position,
    /// End position (active/cursor)
    pub end: Position,
    /// Selection mode
    pub mode: SelectionMode,
}

impl Selection {
    /// Create a new selection
    pub fn new(start: Position, end: Position) -> Self {
        Self {
            start,
            end,
            mode: SelectionMode::Character,
        }
    }

    /// Create a selection with a specific mode
    pub fn with_mode(start: Position, end: Position, mode: SelectionMode) -> Self {
        Self { start, end, mode }
    }

    /// Create an empty selection at a position
    pub fn empty(pos: Position) -> Self {
        Self {
            start: pos,
            end: pos,
            mode: SelectionMode::Character,
        }
    }

    /// Check if the selection is empty
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Get the normalized selection (start always before end)
    pub fn normalized(&self) -> (Position, Position) {
        if self.start.line < self.end.line
            || (self.start.line == self.end.line && self.start.column <= self.end.column)
        {
            (self.start, self.end)
        } else {
            (self.end, self.start)
        }
    }

    /// Get the start position (always the lesser)
    pub fn min(&self) -> Position {
        self.normalized().0
    }

    /// Get the end position (always the greater)
    pub fn max(&self) -> Position {
        self.normalized().1
    }

    /// Check if a position is within the selection
    pub fn contains(&self, pos: Position) -> bool {
        let (start, end) = self.normalized();
        
        if pos.line < start.line || pos.line > end.line {
            return false;
        }
        
        if pos.line == start.line && pos.column < start.column {
            return false;
        }
        
        if pos.line == end.line && pos.column > end.column {
            return false;
        }
        
        true
    }

    /// Extend the selection to include a position
    pub fn extend_to(&mut self, pos: Position) {
        self.end = pos;
    }

    /// Get the number of lines in the selection
    pub fn line_count(&self) -> usize {
        let (start, end) = self.normalized();
        end.line - start.line + 1
    }

    /// Check if this selection overlaps with another
    pub fn overlaps(&self, other: &Selection) -> bool {
        let (s1, e1) = self.normalized();
        let (s2, e2) = other.normalized();

        // Check if either selection starts within the other
        self.contains(s2) || self.contains(e2) || other.contains(s1) || other.contains(e1)
    }

    /// Merge two overlapping selections
    pub fn merge(&self, other: &Selection) -> Option<Selection> {
        if !self.overlaps(other) {
            return None;
        }

        let (s1, e1) = self.normalized();
        let (s2, e2) = other.normalized();

        let start = if s1.line < s2.line || (s1.line == s2.line && s1.column < s2.column) {
            s1
        } else {
            s2
        };

        let end = if e1.line > e2.line || (e1.line == e2.line && e1.column > e2.column) {
            e1
        } else {
            e2
        };

        Some(Selection::new(start, end))
    }
}

/// Multiple selection manager
pub struct SelectionSet {
    selections: Vec<Selection>,
    primary_index: usize,
}

impl SelectionSet {
    /// Create a new empty selection set
    pub fn new() -> Self {
        Self {
            selections: Vec::new(),
            primary_index: 0,
        }
    }

    /// Create with a single selection
    pub fn single(selection: Selection) -> Self {
        Self {
            selections: vec![selection],
            primary_index: 0,
        }
    }

    /// Check if there are any selections
    pub fn is_empty(&self) -> bool {
        self.selections.is_empty()
    }

    /// Get the primary selection
    pub fn primary(&self) -> Option<&Selection> {
        self.selections.get(self.primary_index)
    }

    /// Get mutable primary selection
    pub fn primary_mut(&mut self) -> Option<&mut Selection> {
        self.selections.get_mut(self.primary_index)
    }

    /// Get all selections
    pub fn all(&self) -> &[Selection] {
        &self.selections
    }

    /// Add a selection
    pub fn add(&mut self, selection: Selection) {
        self.selections.push(selection);
        self.merge_overlapping();
    }

    /// Set the primary selection (clears others)
    pub fn set_primary(&mut self, selection: Selection) {
        self.selections.clear();
        self.selections.push(selection);
        self.primary_index = 0;
    }

    /// Clear all selections
    pub fn clear(&mut self) {
        self.selections.clear();
        self.primary_index = 0;
    }

    /// Get selection count
    pub fn count(&self) -> usize {
        self.selections.len()
    }

    /// Merge overlapping selections
    fn merge_overlapping(&mut self) {
        if self.selections.len() < 2 {
            return;
        }

        // Sort selections by start position
        self.selections.sort_by(|a, b| {
            let (s1, _) = a.normalized();
            let (s2, _) = b.normalized();
            s1.line.cmp(&s2.line).then(s1.column.cmp(&s2.column))
        });

        // Merge overlapping
        let mut merged = Vec::new();
        let mut current = self.selections[0].clone();

        for selection in self.selections.iter().skip(1) {
            if let Some(m) = current.merge(selection) {
                current = m;
            } else {
                merged.push(current);
                current = selection.clone();
            }
        }
        merged.push(current);

        self.selections = merged;

        // Ensure primary index is valid
        if self.primary_index >= self.selections.len() {
            self.primary_index = self.selections.len().saturating_sub(1);
        }
    }

    /// Extend all selections to their new endpoints
    pub fn extend_all_to(&mut self, pos: Position) {
        for selection in &mut self.selections {
            selection.extend_to(pos);
        }
        self.merge_overlapping();
    }
}

impl Default for SelectionSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selection_contains() {
        let sel = Selection::new(Position::new(5, 10), Position::new(5, 20));
        
        assert!(sel.contains(Position::new(5, 15)));
        assert!(!sel.contains(Position::new(5, 5)));
        assert!(!sel.contains(Position::new(5, 25)));
    }

    #[test]
    fn test_selection_normalized() {
        let sel = Selection::new(Position::new(10, 0), Position::new(5, 0));
        let (start, end) = sel.normalized();
        
        assert_eq!(start, Position::new(5, 0));
        assert_eq!(end, Position::new(10, 0));
    }

    #[test]
    fn test_selection_merge() {
        let sel1 = Selection::new(Position::new(5, 0), Position::new(10, 0));
        let sel2 = Selection::new(Position::new(8, 0), Position::new(15, 0));
        
        let merged = sel1.merge(&sel2).unwrap();
        let (start, end) = merged.normalized();
        
        assert_eq!(start, Position::new(5, 0));
        assert_eq!(end, Position::new(15, 0));
    }
}
