//! File system verification for AI agent claims

use crate::{types::*, Result, VerifyError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use walkdir::WalkDir;
use uuid::Uuid;

/// File system snapshot for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemSnapshot {
    /// Snapshot ID
    pub id: Uuid,
    /// When the snapshot was taken
    pub timestamp: DateTime<Utc>,
    /// Root path of the snapshot
    pub root_path: PathBuf,
    /// Files in the snapshot
    pub files: HashMap<PathBuf, FileInfo>,
    /// Directories in the snapshot
    pub directories: HashMap<PathBuf, DirectoryInfo>,
    /// Total size in bytes
    pub total_size: u64,
    /// Number of files
    pub file_count: usize,
    /// Number of directories
    pub dir_count: usize,
}

/// Information about a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path relative to root
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// SHA256 hash of contents
    pub hash: String,
    /// Last modified time
    pub modified: DateTime<Utc>,
    /// File permissions (Unix)
    #[cfg(unix)]
    pub permissions: u32,
    /// Whether file is executable
    pub is_executable: bool,
    /// Whether file is a symlink
    pub is_symlink: bool,
}

/// Information about a directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryInfo {
    /// Directory path relative to root
    pub path: PathBuf,
    /// Number of direct children
    pub children_count: usize,
    /// Last modified time
    pub modified: DateTime<Utc>,
    /// Directory permissions (Unix)
    #[cfg(unix)]
    pub permissions: u32,
}

/// Result of file verification
#[derive(Debug, Clone)]
pub struct FileVerification {
    /// Verification result
    pub result: VerificationResult,
    /// Files that were added
    pub added_files: Vec<PathBuf>,
    /// Files that were modified
    pub modified_files: Vec<PathBuf>,
    /// Files that were deleted
    pub deleted_files: Vec<PathBuf>,
    /// Snapshot after verification
    pub current_snapshot: Option<FileSystemSnapshot>,
}

/// File system verifier for Claude sub-agent claims
pub struct FileSystemVerifier {
    /// Snapshots taken
    snapshots: HashMap<Uuid, FileSystemSnapshot>,
    /// Maximum file size to hash (for performance)
    max_hash_size: u64,
    /// Paths to ignore
    ignore_patterns: Vec<String>,
}

impl FileSystemVerifier {
    /// Creates a new file system verifier
    pub fn new() -> Self {
        Self {
            snapshots: HashMap::new(),
            max_hash_size: 100 * 1024 * 1024, // 100MB
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                ".DS_Store".to_string(),
                "*.pyc".to_string(),
            ],
        }
    }
    
    /// Creates a verifier with custom ignore patterns
    pub fn with_ignore_patterns(patterns: Vec<String>) -> Self {
        Self {
            snapshots: HashMap::new(),
            max_hash_size: 100 * 1024 * 1024,
            ignore_patterns: patterns,
        }
    }
    
    /// Takes a snapshot of the file system
    pub async fn take_snapshot(&mut self, root_path: &Path) -> Result<FileSystemSnapshot> {
        if !root_path.exists() {
            return Err(VerifyError::FileSystemError(
                format!("Path does not exist: {}", root_path.display())
            ));
        }
        
        let mut files = HashMap::new();
        let mut directories = HashMap::new();
        let mut total_size = 0u64;
        
        // Walk directory tree
        for entry in WalkDir::new(root_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.should_ignore(e.path()))
        {
            let entry = entry.map_err(|e| VerifyError::FileSystemError(e.to_string()))?;
            let path = entry.path();
            let relative_path = path.strip_prefix(root_path)
                .unwrap_or(path)
                .to_path_buf();
            
            let metadata = entry.metadata()
                .map_err(|e| VerifyError::FileSystemError(e.to_string()))?;
            
            if metadata.is_file() {
                let file_info = self.get_file_info(&relative_path, path, &metadata)?;
                total_size += file_info.size;
                files.insert(relative_path, file_info);
            } else if metadata.is_dir() && path != root_path {
                let dir_info = self.get_directory_info(&relative_path, &metadata)?;
                directories.insert(relative_path, dir_info);
            }
        }
        
        let snapshot = FileSystemSnapshot {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            root_path: root_path.to_path_buf(),
            file_count: files.len(),
            dir_count: directories.len(),
            files,
            directories,
            total_size,
        };
        
        self.snapshots.insert(snapshot.id, snapshot.clone());
        
        Ok(snapshot)
    }
    
    /// Verifies file system against a snapshot
    pub async fn verify_snapshot(
        &mut self,
        paths: &[&str],
        expected: FileSystemSnapshot,
    ) -> Result<FileVerification> {
        let start = Utc::now();
        
        // Take current snapshot
        let root_path = &expected.root_path;
        let current = self.take_snapshot(root_path).await?;
        
        // Compare snapshots
        let mut added_files = Vec::new();
        let mut modified_files = Vec::new();
        let mut deleted_files = Vec::new();
        
        // Check for added and modified files
        for (path, current_info) in &current.files {
            if let Some(expected_info) = expected.files.get(path) {
                // File exists in both - check if modified
                if current_info.hash != expected_info.hash ||
                   current_info.size != expected_info.size {
                    modified_files.push(path.clone());
                }
            } else {
                // File only in current - was added
                added_files.push(path.clone());
            }
        }
        
        // Check for deleted files
        for path in expected.files.keys() {
            if !current.files.contains_key(path) {
                deleted_files.push(path.clone());
            }
        }
        
        // Check specific paths if provided
        let mut path_checks_passed = true;
        let mut path_errors = Vec::new();
        
        for path_str in paths {
            let path = PathBuf::from(path_str);
            let full_path = root_path.join(&path);
            
            if !full_path.exists() {
                path_checks_passed = false;
                path_errors.push(format!("Path does not exist: {}", path_str));
            }
        }
        
        let success = added_files.is_empty() && 
                     modified_files.is_empty() && 
                     deleted_files.is_empty() &&
                     path_checks_passed;
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        // Create verification result
        let result = if success {
            VerificationResult::success(
                VerificationType::FileSystem,
                serde_json::json!({
                    "snapshot_id": expected.id,
                    "paths": paths,
                }),
                serde_json::json!({
                    "files": current.file_count,
                    "directories": current.dir_count,
                    "total_size": current.total_size,
                }),
            )
        } else {
            let error_msg = if !path_errors.is_empty() {
                path_errors.join(", ")
            } else {
                format!(
                    "File system changed: {} added, {} modified, {} deleted",
                    added_files.len(),
                    modified_files.len(),
                    deleted_files.len()
                )
            };
            
            VerificationResult::failure(
                VerificationType::FileSystem,
                serde_json::json!({
                    "snapshot_id": expected.id,
                    "paths": paths,
                }),
                serde_json::json!({
                    "added": added_files.len(),
                    "modified": modified_files.len(),
                    "deleted": deleted_files.len(),
                }),
                error_msg,
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        // Add evidence
        final_result.evidence.push(Evidence {
            evidence_type: EvidenceType::FileMetadata,
            data: serde_json::json!({
                "added_files": &added_files[..added_files.len().min(10)],
                "modified_files": &modified_files[..modified_files.len().min(10)],
                "deleted_files": &deleted_files[..deleted_files.len().min(10)],
            }),
            source: "FileSystemVerifier".to_string(),
            timestamp: Utc::now(),
        });
        
        Ok(FileVerification {
            result: final_result,
            added_files,
            modified_files,
            deleted_files,
            current_snapshot: Some(current),
        })
    }
    
    /// Verifies that specific files exist
    pub async fn verify_files_exist(&self, files: &[&str]) -> Result<VerificationResult> {
        let start = Utc::now();
        let mut missing_files = Vec::new();
        let mut found_files = Vec::new();
        
        for file in files {
            let path = Path::new(file);
            if path.exists() && path.is_file() {
                found_files.push(file.to_string());
            } else {
                missing_files.push(file.to_string());
            }
        }
        
        let success = missing_files.is_empty();
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        let result = if success {
            VerificationResult::success(
                VerificationType::FileSystem,
                serde_json::json!({ "files": files }),
                serde_json::json!({ "found": found_files }),
            )
        } else {
            VerificationResult::failure(
                VerificationType::FileSystem,
                serde_json::json!({ "files": files }),
                serde_json::json!({ 
                    "found": found_files,
                    "missing": missing_files 
                }),
                format!("Files not found: {:?}", missing_files),
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        Ok(final_result)
    }
    
    /// Verifies file content matches expected
    pub async fn verify_file_content(
        &self,
        file_path: &str,
        expected_content: Option<&str>,
        expected_hash: Option<&str>,
    ) -> Result<VerificationResult> {
        let start = Utc::now();
        let path = Path::new(file_path);
        
        if !path.exists() {
            return Ok(VerificationResult::failure(
                VerificationType::FileSystem,
                serde_json::json!({
                    "file": file_path,
                    "expected_content": expected_content,
                    "expected_hash": expected_hash,
                }),
                serde_json::json!({}),
                format!("File does not exist: {}", file_path),
            ));
        }
        
        let content = fs::read_to_string(path)
            .map_err(|e| VerifyError::FileSystemError(e.to_string()))?;
        
        let mut success = true;
        let mut error = None;
        
        // Check content if provided
        if let Some(expected) = expected_content {
            if !content.contains(expected) {
                success = false;
                error = Some(format!("Content does not contain expected text"));
            }
        }
        
        // Check hash if provided
        if let Some(expected) = expected_hash {
            let actual_hash = self.calculate_hash(path)?;
            if actual_hash != expected {
                success = false;
                error = Some(format!("Hash mismatch: expected {}, got {}", expected, actual_hash));
            }
        }
        
        let duration_ms = (Utc::now() - start).num_milliseconds() as u64;
        
        let result = if success {
            VerificationResult::success(
                VerificationType::FileSystem,
                serde_json::json!({
                    "file": file_path,
                    "expected_content": expected_content,
                    "expected_hash": expected_hash,
                }),
                serde_json::json!({
                    "size": content.len(),
                    "lines": content.lines().count(),
                }),
            )
        } else {
            VerificationResult::failure(
                VerificationType::FileSystem,
                serde_json::json!({
                    "file": file_path,
                    "expected_content": expected_content,
                    "expected_hash": expected_hash,
                }),
                serde_json::json!({
                    "size": content.len(),
                }),
                error.unwrap_or_else(|| "Verification failed".to_string()),
            )
        };
        
        let mut final_result = result;
        final_result.duration_ms = duration_ms;
        
        Ok(final_result)
    }
    
    // Helper methods
    
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.ignore_patterns.iter().any(|pattern| {
            if pattern.starts_with('*') {
                path_str.ends_with(&pattern[1..])
            } else {
                path_str.contains(pattern)
            }
        })
    }
    
    fn get_file_info(
        &self,
        relative_path: &Path,
        full_path: &Path,
        metadata: &fs::Metadata,
    ) -> Result<FileInfo> {
        let size = metadata.len();
        
        // Calculate hash for files under size limit
        let hash = if size <= self.max_hash_size {
            self.calculate_hash(full_path)?
        } else {
            format!("SKIPPED_LARGE_FILE_{}", size)
        };
        
        let modified = metadata.modified()
            .map_err(|e| VerifyError::FileSystemError(e.to_string()))?
            .into();
        
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        
        Ok(FileInfo {
            path: relative_path.to_path_buf(),
            size,
            hash,
            modified,
            #[cfg(unix)]
            permissions: metadata.permissions().mode(),
            is_executable: metadata.permissions().readonly(),
            is_symlink: metadata.file_type().is_symlink(),
        })
    }
    
    fn get_directory_info(
        &self,
        relative_path: &Path,
        metadata: &fs::Metadata,
    ) -> Result<DirectoryInfo> {
        let modified = metadata.modified()
            .map_err(|e| VerifyError::FileSystemError(e.to_string()))?
            .into();
        
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        
        Ok(DirectoryInfo {
            path: relative_path.to_path_buf(),
            children_count: 0, // Will be calculated separately if needed
            modified,
            #[cfg(unix)]
            permissions: metadata.permissions().mode(),
        })
    }
    
    fn calculate_hash(&self, path: &Path) -> Result<String> {
        let mut hasher = Sha256::new();
        let content = fs::read(path)
            .map_err(|e| VerifyError::FileSystemError(e.to_string()))?;
        hasher.update(&content);
        Ok(format!("{:x}", hasher.finalize()))
    }
}

impl Default for FileSystemVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;
    
    #[tokio::test]
    async fn test_file_verification() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create a test file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "test content").unwrap();
        
        let verifier = FileSystemVerifier::new();
        
        // Verify file exists
        let result = verifier.verify_files_exist(&[file_path.to_str().unwrap()]).await.unwrap();
        assert!(result.success);
        
        // Verify content
        let result = verifier.verify_file_content(
            file_path.to_str().unwrap(),
            Some("test content"),
            None
        ).await.unwrap();
        assert!(result.success);
    }
    
    #[tokio::test]
    async fn test_snapshot_comparison() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        
        // Create initial file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "initial content").unwrap();
        
        let mut verifier = FileSystemVerifier::new();
        
        // Take initial snapshot
        let snapshot1 = verifier.take_snapshot(temp_dir.path()).await.unwrap();
        assert_eq!(snapshot1.file_count, 1);
        
        // Modify file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "modified content").unwrap();
        
        // Verify against original snapshot
        let verification = verifier.verify_snapshot(&[], snapshot1).await.unwrap();
        assert!(!verification.result.success);
        assert_eq!(verification.modified_files.len(), 1);
    }
}