//! Syntax Highlighting
//! 
//! Tree-sitter based syntax highlighting for multiple languages.

use std::collections::HashMap;
use tree_sitter::{Language, Parser, Query, QueryCursor, Tree};
use tracing::{debug, warn};

/// Highlight type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HighlightType {
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Type,
    Variable,
    Operator,
    Attribute,
    Macro,
    Punctuation,
    Property,
    Constant,
    Label,
    Namespace,
}

impl HighlightType {
    /// Convert from tree-sitter capture name
    pub fn from_capture_name(name: &str) -> Option<Self> {
        match name {
            "keyword" | "keyword.control" | "keyword.function" | "keyword.operator" => Some(Self::Keyword),
            "string" | "string.special" => Some(Self::String),
            "number" | "float" => Some(Self::Number),
            "comment" | "comment.line" | "comment.block" => Some(Self::Comment),
            "function" | "function.builtin" | "function.method" | "method" => Some(Self::Function),
            "type" | "type.builtin" => Some(Self::Type),
            "variable" | "variable.builtin" | "variable.parameter" => Some(Self::Variable),
            "operator" => Some(Self::Operator),
            "attribute" => Some(Self::Attribute),
            "macro" => Some(Self::Macro),
            "punctuation" | "punctuation.bracket" | "punctuation.delimiter" => Some(Self::Punctuation),
            "property" | "field" => Some(Self::Property),
            "constant" | "constant.builtin" => Some(Self::Constant),
            "label" => Some(Self::Label),
            "namespace" | "module" => Some(Self::Namespace),
            _ => None,
        }
    }
}

/// A highlighted range in the text
#[derive(Debug, Clone)]
pub struct HighlightRange {
    pub start: usize,
    pub end: usize,
    pub highlight_type: HighlightType,
}

/// Supported languages for syntax highlighting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxLanguage {
    Rust,
    Kotlin,
    Java,
    Xml,
    Json,
    Toml,
    Markdown,
}

impl SyntaxLanguage {
    /// Get the tree-sitter language
    fn tree_sitter_language(&self) -> Language {
        match self {
            SyntaxLanguage::Rust => tree_sitter_rust::language(),
            SyntaxLanguage::Kotlin => tree_sitter_kotlin::language(),
            SyntaxLanguage::Java => tree_sitter_java::language(),
            SyntaxLanguage::Xml => tree_sitter_xml::language_xml(),
            _ => tree_sitter_rust::language(), // Fallback
        }
    }

    /// Get the highlight query for this language
    fn highlight_query(&self) -> &'static str {
        match self {
            SyntaxLanguage::Rust => RUST_HIGHLIGHTS,
            SyntaxLanguage::Kotlin => KOTLIN_HIGHLIGHTS,
            SyntaxLanguage::Java => JAVA_HIGHLIGHTS,
            SyntaxLanguage::Xml => XML_HIGHLIGHTS,
            _ => "",
        }
    }

    /// Detect language from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "rs" => Some(SyntaxLanguage::Rust),
            "kt" | "kts" => Some(SyntaxLanguage::Kotlin),
            "java" => Some(SyntaxLanguage::Java),
            "xml" => Some(SyntaxLanguage::Xml),
            "json" => Some(SyntaxLanguage::Json),
            "toml" => Some(SyntaxLanguage::Toml),
            "md" | "markdown" => Some(SyntaxLanguage::Markdown),
            _ => None,
        }
    }
}

/// Syntax highlighter using tree-sitter
pub struct SyntaxHighlighter {
    parser: Parser,
    language: SyntaxLanguage,
    query: Option<Query>,
    tree: Option<Tree>,
}

impl SyntaxHighlighter {
    /// Create a new syntax highlighter for a language
    pub fn new(language: SyntaxLanguage) -> Self {
        let mut parser = Parser::new();
        let ts_language = language.tree_sitter_language();
        
        if let Err(e) = parser.set_language(&ts_language) {
            warn!("Failed to set language: {}", e);
        }

        let query = Query::new(&ts_language, language.highlight_query()).ok();

        Self {
            parser,
            language,
            query,
            tree: None,
        }
    }

    /// Parse the source code
    pub fn parse(&mut self, source: &str) {
        self.tree = self.parser.parse(source, self.tree.as_ref());
    }

    /// Update parse tree incrementally
    pub fn update(&mut self, source: &str, start_byte: usize, old_end_byte: usize, new_end_byte: usize) {
        if let Some(tree) = &mut self.tree {
            let start_position = tree_sitter::Point { row: 0, column: start_byte };
            let old_end_position = tree_sitter::Point { row: 0, column: old_end_byte };
            let new_end_position = tree_sitter::Point { row: 0, column: new_end_byte };
            
            tree.edit(&tree_sitter::InputEdit {
                start_byte,
                old_end_byte,
                new_end_byte,
                start_position,
                old_end_position,
                new_end_position,
            });
        }
        self.tree = self.parser.parse(source, self.tree.as_ref());
    }

    /// Get highlights for the parsed code
    pub fn highlights(&self, source: &str) -> Vec<HighlightRange> {
        let mut highlights = Vec::new();

        let (tree, query) = match (&self.tree, &self.query) {
            (Some(t), Some(q)) => (t, q),
            _ => return highlights,
        };

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(query, tree.root_node(), source.as_bytes());

        for match_ in matches {
            for capture in match_.captures {
                let capture_name = &query.capture_names()[capture.index as usize];
                if let Some(highlight_type) = HighlightType::from_capture_name(capture_name) {
                    highlights.push(HighlightRange {
                        start: capture.node.start_byte(),
                        end: capture.node.end_byte(),
                        highlight_type,
                    });
                }
            }
        }

        // Sort by start position
        highlights.sort_by_key(|h| h.start);
        highlights
    }

    /// Get the current language
    pub fn language(&self) -> SyntaxLanguage {
        self.language
    }

    /// Change the language
    pub fn set_language(&mut self, language: SyntaxLanguage) {
        if language != self.language {
            self.language = language;
            let ts_language = language.tree_sitter_language();
            let _ = self.parser.set_language(&ts_language);
            self.query = Query::new(&ts_language, language.highlight_query()).ok();
            self.tree = None;
        }
    }

    /// Check if the tree is valid
    pub fn has_tree(&self) -> bool {
        self.tree.is_some()
    }
}

// Highlight queries for different languages

const RUST_HIGHLIGHTS: &str = r#"
; Keywords
"fn" @keyword.function
"let" @keyword
"mut" @keyword
"const" @keyword
"static" @keyword
"if" @keyword.control
"else" @keyword.control
"match" @keyword.control
"for" @keyword.control
"while" @keyword.control
"loop" @keyword.control
"break" @keyword.control
"continue" @keyword.control
"return" @keyword.control
"pub" @keyword
"mod" @keyword
"use" @keyword
"struct" @keyword
"enum" @keyword
"trait" @keyword
"impl" @keyword
"type" @keyword
"where" @keyword
"async" @keyword
"await" @keyword
"unsafe" @keyword
"extern" @keyword
"crate" @keyword
"self" @variable.builtin
"Self" @type.builtin

; Types
(type_identifier) @type
(primitive_type) @type.builtin

; Functions
(function_item name: (identifier) @function)
(call_expression function: (identifier) @function)
(macro_invocation macro: (identifier) @macro)

; Variables
(identifier) @variable
(field_identifier) @property

; Strings
(string_literal) @string
(char_literal) @string
(raw_string_literal) @string

; Numbers
(integer_literal) @number
(float_literal) @number

; Comments
(line_comment) @comment
(block_comment) @comment

; Attributes
(attribute_item) @attribute

; Operators
"+" @operator
"-" @operator
"*" @operator
"/" @operator
"%" @operator
"=" @operator
"==" @operator
"!=" @operator
"<" @operator
">" @operator
"<=" @operator
">=" @operator
"&&" @operator
"||" @operator
"!" @operator
"&" @operator
"|" @operator
"^" @operator
"<<" @operator
">>" @operator
"->" @operator
"=>" @operator
"::" @operator
"." @operator
"#"
