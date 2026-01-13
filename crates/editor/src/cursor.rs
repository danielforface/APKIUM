//! Cursor Management
//! 
//! Handles cursor position, movement, and multi-cursor support.

use crate::buffer::Position;

/// Cursor movement direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

/// Cursor movement unit
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementUnit {
    Character,
    Word,
    Line,
    Paragraph,
    Page,
    Document,
}

/// A text cursor
#[derive(Debug, Clone)]
pub struct Cursor {
    /// Current position
    position: Position,
    /// Preferred column (for vertical movement)
    preferred_column: Option<usize>,
    /// Cursor blink state
    visible: bool,
    /// Cursor ID (for multi-cursor)
    id: usize,
}

impl Cursor {
    /// Create a new cursor at position (0, 0)
    pub fn new() -> Self {
        Self {
            position: Position::default(),
            preferred_column: None,
            visible: true,
            id: 0,
        }
    }

    /// Create a cursor at a specific position
    pub fn at(line: usize, column: usize) -> Self {
        Self {
            position: Position { line, column },
            preferred_column: None,
            visible: true,
            id: 0,
        }
    }

    /// Get the current position
    pub fn position(&self) -> Position {
        self.position
    }

    /// Get the line number
    pub fn line(&self) -> usize {
        self.position.line
    }

    /// Get the column number
    pub fn column(&self) -> usize {
        self.position.column
    }

    /// Set the position
    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
        self.preferred_column = None;
    }

    /// Move to a specific line and column
    pub fn move_to(&mut self, line: usize, column: usize) {
        self.position = Position { line, column };
        self.preferred_column = None;
    }

    /// Move to a char index
    pub fn move_to_char(&mut self, _char_idx: usize) {
        // This would need the buffer to convert char index to position
        // For now, just reset preferred column
        self.preferred_column = None;
    }

    /// Move in a direction by one character
    pub fn move_direction(&mut self, direction: Direction, line_count: usize, line_length: impl Fn(usize) -> usize) {
        match direction {
            Direction::Up => {
                if self.position.line > 0 {
                    self.position.line -= 1;
                    let len = line_length(self.position.line);
                    let preferred = self.preferred_column.unwrap_or(self.position.column);
                    self.position.column = preferred.min(len);
                    self.preferred_column = Some(preferred);
                }
            }
            Direction::Down => {
                if self.position.line + 1 < line_count {
                    self.position.line += 1;
                    let len = line_length(self.position.line);
                    let preferred = self.preferred_column.unwrap_or(self.position.column);
                    self.position.column = preferred.min(len);
                    self.preferred_column = Some(preferred);
                }
            }
            Direction::Left => {
                if self.position.column > 0 {
                    self.position.column -= 1;
                } else if self.position.line > 0 {
                    self.position.line -= 1;
                    self.position.column = line_length(self.position.line);
                }
                self.preferred_column = None;
            }
            Direction::Right => {
                let len = line_length(self.position.line);
                if self.position.column < len {
                    self.position.column += 1;
                } else if self.position.line + 1 < line_count {
                    self.position.line += 1;
                    self.position.column = 0;
                }
                self.preferred_column = None;
            }
        }
    }

    /// Move to the start of the line
    pub fn move_to_line_start(&mut self) {
        self.position.column = 0;
        self.preferred_column = None;
    }

    /// Move to the end of the line
    pub fn move_to_line_end(&mut self, line_length: usize) {
        self.position.column = line_length;
        self.preferred_column = None;
    }

    /// Move to the start of the document
    pub fn move_to_document_start(&mut self) {
        self.position = Position::default();
        self.preferred_column = None;
    }

    /// Move to the end of the document
    pub fn move_to_document_end(&mut self, line_count: usize, last_line_length: usize) {
        self.position = Position {
            line: line_count.saturating_sub(1),
            column: last_line_length,
        };
        self.preferred_column = None;
    }

    /// Toggle cursor visibility (for blinking)
    pub fn toggle_visibility(&mut self) {
        self.visible = !self.visible;
    }

    /// Check if cursor is visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Set cursor visibility
    pub fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
    }

    /// Get cursor ID
    pub fn id(&self) -> usize {
        self.id
    }

    /// Set cursor ID
    pub fn set_id(&mut self, id: usize) {
        self.id = id;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-cursor support
pub struct CursorSet {
    cursors: Vec<Cursor>,
    primary_index: usize,
}

impl CursorSet {
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor::new()],
            primary_index: 0,
        }
    }

    /// Get the primary cursor
    pub fn primary(&self) -> &Cursor {
        &self.cursors[self.primary_index]
    }

    /// Get mutable primary cursor
    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary_index]
    }

    /// Get all cursors
    pub fn all(&self) -> &[Cursor] {
        &self.cursors
    }

    /// Get all cursors mutably
    pub fn all_mut(&mut self) -> &mut [Cursor] {
        &mut self.cursors
    }

    /// Add a new cursor
    pub fn add(&mut self, cursor: Cursor) {
        let mut cursor = cursor;
        cursor.set_id(self.cursors.len());
        self.cursors.push(cursor);
    }

    /// Add a cursor at a position
    pub fn add_at(&mut self, line: usize, column: usize) {
        let mut cursor = Cursor::at(line, column);
        cursor.set_id(self.cursors.len());
        self.cursors.push(cursor);
    }

    /// Remove all cursors except primary
    pub fn clear_secondary(&mut self) {
        let primary = self.cursors.remove(self.primary_index);
        self.cursors.clear();
        self.cursors.push(primary);
        self.primary_index = 0;
    }

    /// Check if there are multiple cursors
    pub fn has_multiple(&self) -> bool {
        self.cursors.len() > 1
    }

    /// Get cursor count
    pub fn count(&self) -> usize {
        self.cursors.len()
    }

    /// Move all cursors in a direction
    pub fn move_all(&mut self, direction: Direction, line_count: usize, line_length: impl Fn(usize) -> usize) {
        for cursor in &mut self.cursors {
            cursor.move_direction(direction, line_count, &line_length);
        }
        self.merge_overlapping();
    }

    /// Merge overlapping cursors
    fn merge_overlapping(&mut self) {
        // Sort by position
        self.cursors.sort_by(|a, b| {
            a.position.line.cmp(&b.position.line)
                .then(a.position.column.cmp(&b.position.column))
        });

        // Remove duplicates
        self.cursors.dedup_by(|a, b| {
            a.position.line == b.position.line && a.position.column == b.position.column
        });

        // Ensure primary index is valid
        if self.primary_index >= self.cursors.len() {
            self.primary_index = self.cursors.len() - 1;
        }
    }
}

impl Default for CursorSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_movement() {
        let mut cursor = Cursor::at(5, 10);
        
        cursor.move_direction(Direction::Down, 100, |_| 80);
        assert_eq!(cursor.line(), 6);
        
        cursor.move_direction(Direction::Up, 100, |_| 80);
        assert_eq!(cursor.line(), 5);
    }

    #[test]
    fn test_multi_cursor() {
        let mut set = CursorSet::new();
        set.add_at(5, 0);
        set.add_at(10, 0);
        
        assert_eq!(set.count(), 3);
        assert!(set.has_multiple());
        
        set.clear_secondary();
        assert_eq!(set.count(), 1);
    }
}
