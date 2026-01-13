//! R-Droid Editor
//! 
//! High-performance code editor with:
//! - Rope-based text buffer for efficient large file handling
//! - Tree-sitter syntax highlighting
//! - LSP integration ready

pub mod buffer;
pub mod syntax;
pub mod cursor;
pub mod selection;
pub mod commands;

pub use buffer::TextBuffer;
pub use syntax::SyntaxHighlighter;
pub use cursor::Cursor;
pub use selection::Selection;
