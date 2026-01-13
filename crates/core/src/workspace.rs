//! Workspace Management
//! 
//! Handles the IDE workspace, file tree, and open files.

use std::path::PathBuf;
use std::collections::HashMap;
// use serde::{Deserialize, Serialize};
use tracing::info;

use crate::error::{Result, RDroidError};
use crate::project::Project;

/// File type classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    Rust,
    Kotlin,
    Java,
    Xml,
    Gradle,
    Json,
    Yaml,
    Toml,
    Markdown,
    Text,
    Image,
    Binary,
    Unknown,
}

impl FileType {
    /// Detect file type from extension
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => FileType::Rust,
            "kt" | "kts" => FileType::Kotlin,
            "java" => FileType::Java,
            "xml" => FileType::Xml,
            "gradle" => FileType::Gradle,
            "json" => FileType::Json,
            "yml" | "yaml" => FileType::Yaml,
            "toml" => FileType::Toml,
            "md" | "markdown" => FileType::Markdown,
            "txt" => FileType::Text,
            "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp" => FileType::Image,
            "so" | "dll" | "exe" | "apk" | "aab" => FileType::Binary,
            _ => FileType::Unknown,
        }
    }

    /// Get the language ID for LSP
    pub fn language_id(&self) -> Option<&'static str> {
        match self {
            FileType::Rust => Some("rust"),
            FileType::Kotlin => Some("kotlin"),
            FileType::Java => Some("java"),
            FileType::Xml => Some("xml"),
            FileType::Json => Some("json"),
            FileType::Yaml => Some("yaml"),
            FileType::Toml => Some("toml"),
            FileType::Markdown => Some("markdown"),
            _ => None,
        }
    }
}

/// Entry in the file tree
#[derive(Debug, Clone)]
pub struct FileTreeEntry {
    /// File/folder name
    pub name: String,
    /// Full path
    pub path: PathBuf,
    /// Is this a directory?
    pub is_dir: bool,
    /// File type
    pub file_type: FileType,
    /// Children (if directory)
    pub children: Vec<FileTreeEntry>,
    /// Is expanded in UI?
    pub expanded: bool,
}

impl FileTreeEntry {
    /// Create a new file entry
    pub fn new_file(path: PathBuf) -> Self {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let ext = path.extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        Self {
            name,
            path,
            is_dir: false,
            file_type: FileType::from_extension(&ext),
            children: Vec::new(),
            expanded: false,
        }
    }

    /// Create a new directory entry
    pub fn new_dir(path: PathBuf) -> Self {
        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        Self {
            name,
            path,
            is_dir: true,
            file_type: FileType::Unknown,
            children: Vec::new(),
            expanded: false,
        }
    }
}

/// Open file buffer
#[derive(Debug, Clone)]
pub struct OpenFile {
    /// File path
    pub path: PathBuf,
    /// File content
    pub content: String,
    /// Original content (for dirty detection)
    pub original_content: String,
    /// File type
    pub file_type: FileType,
    /// Cursor position (line, column)
    pub cursor: (usize, usize),
    /// Selection range (start, end)
    pub selection: Option<((usize, usize), (usize, usize))>,
    /// Is modified?
    pub is_dirty: bool,
}

impl OpenFile {
    /// Create a new open file
    pub async fn open(path: PathBuf) -> Result<Self> {
        let content = tokio::fs::read_to_string(&path).await?;
        let ext = path.extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();

        Ok(Self {
            path,
            original_content: content.clone(),
            content,
            file_type: FileType::from_extension(&ext),
            cursor: (0, 0),
            selection: None,
            is_dirty: false,
        })
    }

    /// Update content
    pub fn set_content(&mut self, content: String) {
        self.content = content;
        self.is_dirty = self.content != self.original_content;
    }

    /// Save the file
    pub async fn save(&mut self) -> Result<()> {
        tokio::fs::write(&self.path, &self.content).await?;
        self.original_content = self.content.clone();
        self.is_dirty = false;
        Ok(())
    }

    /// Revert to original content
    pub fn revert(&mut self) {
        self.content = self.original_content.clone();
        self.is_dirty = false;
    }
}

/// Workspace state
#[derive(Debug, Clone)]
pub struct Workspace {
    /// Workspace root directory
    pub root: PathBuf,
    /// Associated project (if any)
    pub project: Option<Project>,
    /// File tree
    pub file_tree: Vec<FileTreeEntry>,
    /// Open files
    open_files: HashMap<PathBuf, OpenFile>,
    /// Currently active file
    pub active_file: Option<PathBuf>,
}

impl Workspace {
    /// Open a workspace at the given path
    pub async fn open(path: PathBuf) -> Result<Self> {
        if !path.exists() {
            return Err(RDroidError::NotFound(format!("Path not found: {:?}", path)));
        }

        info!("Opening workspace at {:?}", path);

        // Try to load as a project
        let project = Project::open(path.clone()).await.ok();

        // Build initial file tree
        let file_tree = Self::build_file_tree(&path, 2).await?;

        Ok(Self {
            root: path,
            project,
            file_tree,
            open_files: HashMap::new(),
            active_file: None,
        })
    }

    /// Build file tree recursively
    async fn build_file_tree(path: &PathBuf, depth: usize) -> Result<Vec<FileTreeEntry>> {
        if depth == 0 {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(path).await?;

        while let Some(entry) = dir.next_entry().await? {
            let entry_path = entry.path();
            let name = entry_path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            // Skip hidden files and common ignored directories
            if name.starts_with('.') || name == "target" || name == "build" || name == "node_modules" {
                continue;
            }

            let metadata = entry.metadata().await?;
            
            if metadata.is_dir() {
                let mut dir_entry = FileTreeEntry::new_dir(entry_path.clone());
                dir_entry.children = Box::pin(Self::build_file_tree(&entry_path, depth - 1)).await?;
                entries.push(dir_entry);
            } else {
                entries.push(FileTreeEntry::new_file(entry_path));
            }
        }

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            }
        });

        Ok(entries)
    }

    /// Refresh the file tree
    pub async fn refresh_file_tree(&mut self) -> Result<()> {
        self.file_tree = Self::build_file_tree(&self.root, 2).await?;
        Ok(())
    }

    /// Expand a directory in the file tree
    pub async fn expand_directory(&mut self, path: &PathBuf) -> Result<()> {
        let children = Self::build_file_tree(path, 1).await?;
        
        fn expand_in_tree(entries: &mut Vec<FileTreeEntry>, path: &PathBuf, children: Vec<FileTreeEntry>) -> bool {
            for entry in entries.iter_mut() {
                if &entry.path == path {
                    entry.children = children;
                    entry.expanded = true;
                    return true;
                }
                if entry.is_dir && expand_in_tree(&mut entry.children, path, children.clone()) {
                    return true;
                }
            }
            false
        }

        expand_in_tree(&mut self.file_tree, path, children);
        Ok(())
    }

    /// Open a file
    pub async fn open_file(&mut self, path: PathBuf) -> Result<&OpenFile> {
        if !self.open_files.contains_key(&path) {
            let file = OpenFile::open(path.clone()).await?;
            self.open_files.insert(path.clone(), file);
        }
        self.active_file = Some(path.clone());
        self.open_files.get(&path).ok_or_else(|| RDroidError::Internal("File not found".into()))
    }

    /// Get an open file
    pub fn get_file(&self, path: &PathBuf) -> Option<&OpenFile> {
        self.open_files.get(path)
    }

    /// Get a mutable reference to an open file
    pub fn get_file_mut(&mut self, path: &PathBuf) -> Option<&mut OpenFile> {
        self.open_files.get_mut(path)
    }

    /// Close a file
    pub fn close_file(&mut self, path: &PathBuf) -> Option<OpenFile> {
        if self.active_file.as_ref() == Some(path) {
            self.active_file = self.open_files.keys()
                .find(|k| *k != path)
                .cloned();
        }
        self.open_files.remove(path)
    }

    /// Get all open files
    pub fn open_files(&self) -> impl Iterator<Item = &OpenFile> {
        self.open_files.values()
    }

    /// Check if any files are dirty
    pub fn has_dirty_files(&self) -> bool {
        self.open_files.values().any(|f| f.is_dirty)
    }

    /// Save all dirty files
    pub async fn save_all(&mut self) -> Result<()> {
        for file in self.open_files.values_mut() {
            if file.is_dirty {
                file.save().await?;
            }
        }
        Ok(())
    }

    /// Get file count (for UI virtualization)
    pub fn total_file_count(&self) -> usize {
        fn count_entries(entries: &[FileTreeEntry]) -> usize {
            entries.iter().map(|e| 1 + count_entries(&e.children)).sum()
        }
        count_entries(&self.file_tree)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileType::from_extension("rs"), FileType::Rust);
        assert_eq!(FileType::from_extension("kt"), FileType::Kotlin);
        assert_eq!(FileType::from_extension("xml"), FileType::Xml);
        assert_eq!(FileType::from_extension("unknown"), FileType::Unknown);
    }
}
