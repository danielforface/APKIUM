//! Editor Commands
//! 
//! High-level editor commands that can be executed and undone.

use crate::buffer::{Position, TextBuffer};
use crate::cursor::Direction;
use crate::selection::Selection;

/// Editor command type
#[derive(Debug, Clone)]
pub enum Command {
    // Movement commands
    MoveCursor(Direction),
    MoveCursorWord(Direction),
    MoveCursorLine(Direction),
    MoveToLineStart,
    MoveToLineEnd,
    MoveToDocumentStart,
    MoveToDocumentEnd,
    PageUp,
    PageDown,

    // Selection commands
    SelectAll,
    SelectWord,
    SelectLine,
    ExtendSelection(Direction),
    ClearSelection,

    // Edit commands
    InsertChar(char),
    InsertText(String),
    InsertNewline,
    InsertTab,
    DeleteBackward,
    DeleteForward,
    DeleteWord,
    DeleteLine,

    // Clipboard commands
    Copy,
    Cut,
    Paste(String),

    // Undo/Redo
    Undo,
    Redo,

    // Find/Replace
    Find(String),
    FindNext,
    FindPrevious,
    Replace(String, String),
    ReplaceAll(String, String),

    // Indentation
    Indent,
    Outdent,

    // Code actions
    ToggleComment,
    FormatDocument,

    // Multi-cursor
    AddCursorAbove,
    AddCursorBelow,
    AddCursorAtSelection,
}

/// Command execution result
#[derive(Debug)]
pub struct CommandResult {
    pub success: bool,
    pub message: Option<String>,
}

impl CommandResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            message: None,
        }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: Some(message.into()),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: Some(message.into()),
        }
    }
}

/// Command executor
pub struct CommandExecutor {
    clipboard: String,
    find_query: String,
    find_matches: Vec<std::ops::Range<usize>>,
    find_index: usize,
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            clipboard: String::new(),
            find_query: String::new(),
            find_matches: Vec::new(),
            find_index: 0,
        }
    }

    /// Execute a command on a buffer
    pub fn execute(&mut self, command: Command, buffer: &mut TextBuffer) -> CommandResult {
        match command {
            // Movement commands
            Command::MoveCursor(direction) => {
                let line_count = buffer.line_count();
                let mut line_lengths = Vec::new();
                for i in 0..line_count {
                    line_lengths.push(buffer.line(i).map(|l| l.len_chars()).unwrap_or(0));
                }
                buffer.cursor_mut().move_direction(direction, line_count, |line| {
                    line_lengths.get(line).copied().unwrap_or(0)
                });
                buffer.set_selection(None);
                CommandResult::ok()
            }

            Command::MoveToLineStart => {
                buffer.cursor_mut().move_to_line_start();
                buffer.set_selection(None);
                CommandResult::ok()
            }

            Command::MoveToLineEnd => {
                let line = buffer.cursor().line();
                let len = buffer.line(line).map(|l| l.len_chars()).unwrap_or(0);
                buffer.cursor_mut().move_to_line_end(len);
                buffer.set_selection(None);
                CommandResult::ok()
            }

            Command::MoveToDocumentStart => {
                buffer.cursor_mut().move_to_document_start();
                buffer.set_selection(None);
                CommandResult::ok()
            }

            Command::MoveToDocumentEnd => {
                let line_count = buffer.line_count();
                let last_line_len = buffer.line(line_count.saturating_sub(1))
                    .map(|l| l.len_chars())
                    .unwrap_or(0);
                buffer.cursor_mut().move_to_document_end(line_count, last_line_len);
                buffer.set_selection(None);
                CommandResult::ok()
            }

            // Selection commands
            Command::SelectAll => {
                let line_count = buffer.line_count();
                let last_line_len = buffer.line(line_count.saturating_sub(1))
                    .map(|l| l.len_chars())
                    .unwrap_or(0);
                
                let selection = Selection::new(
                    Position::new(0, 0),
                    Position::new(line_count.saturating_sub(1), last_line_len),
                );
                buffer.set_selection(Some(selection));
                CommandResult::ok()
            }

            Command::ClearSelection => {
                buffer.set_selection(None);
                CommandResult::ok()
            }

            // Edit commands
            Command::InsertChar(c) => {
                buffer.delete_selection();
                let pos = buffer.cursor().position();
                buffer.insert(pos, &c.to_string());
                CommandResult::ok()
            }

            Command::InsertText(text) => {
                buffer.delete_selection();
                let pos = buffer.cursor().position();
                buffer.insert(pos, &text);
                CommandResult::ok()
            }

            Command::InsertNewline => {
                buffer.delete_selection();
                buffer.insert_newline();
                CommandResult::ok()
            }

            Command::InsertTab => {
                buffer.delete_selection();
                buffer.insert_tab();
                CommandResult::ok()
            }

            Command::DeleteBackward => {
                if buffer.delete_selection() {
                    return CommandResult::ok();
                }
                
                let pos = buffer.cursor().position();
                if pos.column > 0 {
                    let char_idx = buffer.position_to_char(pos);
                    buffer.delete(char_idx - 1..char_idx);
                } else if pos.line > 0 {
                    // Delete newline at end of previous line
                    let prev_line_len = buffer.line(pos.line - 1)
                        .map(|l| l.len_chars())
                        .unwrap_or(0);
                    let char_idx = buffer.position_to_char(Position::new(pos.line - 1, prev_line_len));
                    buffer.delete(char_idx..char_idx + 1);
                }
                CommandResult::ok()
            }

            Command::DeleteForward => {
                if buffer.delete_selection() {
                    return CommandResult::ok();
                }
                
                let pos = buffer.cursor().position();
                let char_idx = buffer.position_to_char(pos);
                if char_idx < buffer.char_count() {
                    buffer.delete(char_idx..char_idx + 1);
                }
                CommandResult::ok()
            }

            Command::DeleteLine => {
                let line = buffer.cursor().line();
                buffer.delete_line(line);
                CommandResult::ok()
            }

            // Clipboard commands
            Command::Copy => {
                if let Some(text) = buffer.selected_text() {
                    self.clipboard = text;
                    CommandResult::with_message("Copied to clipboard")
                } else {
                    CommandResult::with_message("No selection to copy")
                }
            }

            Command::Cut => {
                if let Some(text) = buffer.selected_text() {
                    self.clipboard = text;
                    buffer.delete_selection();
                    CommandResult::with_message("Cut to clipboard")
                } else {
                    CommandResult::with_message("No selection to cut")
                }
            }

            Command::Paste(text) => {
                buffer.delete_selection();
                let pos = buffer.cursor().position();
                buffer.insert(pos, &text);
                CommandResult::ok()
            }

            // Undo/Redo
            Command::Undo => {
                if buffer.undo() {
                    CommandResult::with_message("Undone")
                } else {
                    CommandResult::with_message("Nothing to undo")
                }
            }

            Command::Redo => {
                if buffer.redo() {
                    CommandResult::with_message("Redone")
                } else {
                    CommandResult::with_message("Nothing to redo")
                }
            }

            // Find
            Command::Find(query) => {
                self.find_query = query.clone();
                self.find_matches = buffer.find(&query, false);
                self.find_index = 0;
                
                let count = self.find_matches.len();
                if count > 0 {
                    CommandResult::with_message(format!("Found {} matches", count))
                } else {
                    CommandResult::with_message("No matches found")
                }
            }

            Command::FindNext => {
                if self.find_matches.is_empty() {
                    return CommandResult::with_message("No matches");
                }
                
                self.find_index = (self.find_index + 1) % self.find_matches.len();
                let range = &self.find_matches[self.find_index];
                let start = buffer.char_to_position(range.start);
                let end = buffer.char_to_position(range.end);
                
                buffer.cursor_mut().set_position(start);
                buffer.set_selection(Some(Selection::new(start, end)));
                
                CommandResult::with_message(format!(
                    "Match {} of {}", 
                    self.find_index + 1, 
                    self.find_matches.len()
                ))
            }

            Command::FindPrevious => {
                if self.find_matches.is_empty() {
                    return CommandResult::with_message("No matches");
                }
                
                if self.find_index == 0 {
                    self.find_index = self.find_matches.len() - 1;
                } else {
                    self.find_index -= 1;
                }
                
                let range = &self.find_matches[self.find_index];
                let start = buffer.char_to_position(range.start);
                let end = buffer.char_to_position(range.end);
                
                buffer.cursor_mut().set_position(start);
                buffer.set_selection(Some(Selection::new(start, end)));
                
                CommandResult::with_message(format!(
                    "Match {} of {}", 
                    self.find_index + 1, 
                    self.find_matches.len()
                ))
            }

            Command::ReplaceAll(query, replacement) => {
                let count = buffer.find_replace(&query, &replacement, false);
                CommandResult::with_message(format!("Replaced {} occurrences", count))
            }

            // Indentation
            Command::Indent => {
                // Get selected lines or current line
                let (start_line, end_line) = if let Some(sel) = buffer.selection() {
                    let (start, end) = sel.normalized();
                    (start.line, end.line)
                } else {
                    let line = buffer.cursor().line();
                    (line, line)
                };

                for line in start_line..=end_line {
                    let pos = Position::new(line, 0);
                    buffer.insert(pos, "    ");
                }
                
                CommandResult::ok()
            }

            Command::Outdent => {
                let (start_line, end_line) = if let Some(sel) = buffer.selection() {
                    let (start, end) = sel.normalized();
                    (start.line, end.line)
                } else {
                    let line = buffer.cursor().line();
                    (line, line)
                };

                for line in start_line..=end_line {
                    if let Some(line_content) = buffer.line_str(line) {
                        let indent = line_content.chars().take_while(|c| *c == ' ').count();
                        let remove = indent.min(4);
                        if remove > 0 {
                            let start = buffer.position_to_char(Position::new(line, 0));
                            buffer.delete(start..start + remove);
                        }
                    }
                }
                
                CommandResult::ok()
            }

            Command::ToggleComment => {
                // Toggle line comments
                let line = buffer.cursor().line();
                if let Some(content) = buffer.line_str(line) {
                    let trimmed = content.trim_start();
                    if trimmed.starts_with("// ") {
                        // Remove comment
                        if let Some(idx) = content.find("// ") {
                            let start = buffer.position_to_char(Position::new(line, idx));
                            buffer.delete(start..start + 3);
                        }
                    } else if trimmed.starts_with("//") {
                        // Remove comment without space
                        if let Some(idx) = content.find("//") {
                            let start = buffer.position_to_char(Position::new(line, idx));
                            buffer.delete(start..start + 2);
                        }
                    } else {
                        // Add comment
                        let indent = content.len() - trimmed.len();
                        let pos = Position::new(line, indent);
                        buffer.insert(pos, "// ");
                    }
                }
                CommandResult::ok()
            }

            _ => CommandResult::error("Command not implemented"),
        }
    }

    /// Get clipboard content
    pub fn clipboard(&self) -> &str {
        &self.clipboard
    }

    /// Set clipboard content
    pub fn set_clipboard(&mut self, content: String) {
        self.clipboard = content;
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_command() {
        let mut buffer = TextBuffer::from_str("Hello");
        let mut executor = CommandExecutor::new();
        
        executor.execute(Command::MoveToLineEnd, &mut buffer);
        executor.execute(Command::InsertText(" World".into()), &mut buffer);
        
        assert_eq!(buffer.text(), "Hello World");
    }

    #[test]
    fn test_undo_redo() {
        let mut buffer = TextBuffer::from_str("Hello");
        let mut executor = CommandExecutor::new();
        
        executor.execute(Command::MoveToLineEnd, &mut buffer);
        executor.execute(Command::InsertText(" World".into()), &mut buffer);
        
        executor.execute(Command::Undo, &mut buffer);
        assert_eq!(buffer.text(), "Hello");
        
        executor.execute(Command::Redo, &mut buffer);
        assert_eq!(buffer.text(), "Hello World");
    }
}
