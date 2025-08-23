//! Project workspace management for the demo

use std::path::{Path, PathBuf};
use std::fs;
use anyhow::Result;
use tempfile::TempDir;

/// Manages the project workspace where agents will build the API
pub struct ProjectWorkspace {
    root: PathBuf,
    _temp: Option<TempDir>, // Keep tempdir alive if using temp storage
}

impl ProjectWorkspace {
    /// Create a new project workspace
    pub async fn new(name: &str) -> Result<Self> {
        // Create in a temp directory for the demo
        let temp = TempDir::new()?;
        let root = temp.path().join(name);
        fs::create_dir_all(&root)?;
        
        Ok(Self {
            root,
            _temp: Some(temp),
        })
    }
    
    /// Create workspace in a specific directory
    pub async fn new_at(path: impl AsRef<Path>) -> Result<Self> {
        let root = path.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;
        
        Ok(Self {
            root,
            _temp: None,
        })
    }
    
    /// Create workspace from an existing path (for agent-runner)
    pub fn from_path(path: PathBuf) -> Self {
        Self {
            root: path,
            _temp: None,
        }
    }
    
    /// Get the root directory of the workspace
    pub fn root(&self) -> &Path {
        &self.root
    }
    
    /// Create a subdirectory in the workspace
    pub fn create_dir(&self, name: &str) -> Result<PathBuf> {
        let path = self.root.join(name);
        fs::create_dir_all(&path)?;
        Ok(path)
    }
    
    /// Write a file in the workspace
    pub fn write_file(&self, path: impl AsRef<Path>, content: &str) -> Result<()> {
        let full_path = self.root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(full_path, content)?;
        Ok(())
    }
    
    /// Read a file from the workspace
    pub fn read_file(&self, path: impl AsRef<Path>) -> Result<String> {
        let full_path = self.root.join(path);
        Ok(fs::read_to_string(full_path)?)
    }
    
    /// Check if a file exists
    pub fn file_exists(&self, path: impl AsRef<Path>) -> bool {
        self.root.join(path).exists()
    }
    
    /// List files in a directory
    pub fn list_files(&self, dir: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let full_path = self.root.join(dir);
        let mut files = Vec::new();
        
        if full_path.is_dir() {
            for entry in fs::read_dir(full_path)? {
                let entry = entry?;
                files.push(entry.path());
            }
        }
        
        Ok(files)
    }
}