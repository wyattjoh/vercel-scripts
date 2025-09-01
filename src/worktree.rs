use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command; // RUST LEARNING: For spawning child processes (like Node's child_process)
use thiserror::Error;

// RUST LEARNING: Domain-specific error types (better than generic errors)
// - Each module can have its own error enum
// - More specific than throwing generic Error objects
#[derive(Error, Debug)]
pub enum WorktreeError {
    #[error("Git command failed: {0}")]
    GitCommand(String), // Custom error with message
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error), // Auto-conversion from std::io::Error
    #[error("UTF-8 conversion error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error), // Auto-conversion from UTF-8 errors
}

pub type Result<T> = std::result::Result<T, WorktreeError>;

// RUST LEARNING: `PartialEq` enables == comparison (like implementing equals() in Java)
// - Auto-derives comparison logic for all fields
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Worktree {
    pub path: PathBuf,
    pub branch: String,
    pub head: String, // Git commit hash
}

impl Worktree {
    /// Get the relative path from the base directory
    pub fn relative_path(&self, base_dir: &Path) -> PathBuf {
        // RUST LEARNING: `unwrap_or()` provides fallback for failed operations
        // - If strip_prefix fails, use the original path
        // - Like: result.ok() || fallback in JavaScript
        self.path
            .strip_prefix(base_dir)
            .unwrap_or(&self.path)
            .to_path_buf()
    }

    /// Get a display string for this worktree
    pub fn display_name(&self, base_dir: &Path) -> String {
        let relative = self.relative_path(base_dir);
        if relative.as_os_str().is_empty() {
            self.branch.clone()
        } else {
            format!("{} ({})", self.branch, relative.display())
        }
    }

    /// Check if this is the main worktree (path equals base directory)
    // RUST LEARNING: `#[cfg(test)]` only compiles this code during testing
    // - Keeps test-only code out of production builds
    // - Like conditional compilation for tests
    #[cfg(test)]
    pub fn is_main(&self, base_dir: &Path) -> bool {
        self.path == base_dir
    }
}

// RUST LEARNING: Unit struct (struct with no fields)
// - Like a namespace or static class in other languages
// - All methods are associated functions (like static methods)
pub struct WorktreeManager;

impl WorktreeManager {
    /// List all git worktrees in the given base directory
    // RUST LEARNING: Generic parameter with trait bound
    // - `P: AsRef<Path>` means P can be converted to a Path reference
    // - Accepts String, PathBuf, Path, etc. - very flexible
    pub fn list_worktrees<P: AsRef<Path>>(base_dir: P) -> Result<Vec<Worktree>> {
        // RUST LEARNING: `.as_ref()` converts the generic P to &Path
        let base_dir = base_dir.as_ref();

        // RUST LEARNING: Builder pattern for Command (like fluent API)
        let output = Command::new("git")
            .args(["worktree", "list", "--porcelain"]) // Array literal for args
            .current_dir(base_dir)
            .output() // Execute and capture output
            // RUST LEARNING: `map_err()` transforms the error type
            // - Converts std::io::Error to WorktreeError::GitCommand
            .map_err(|e| {
                WorktreeError::GitCommand(format!("Failed to execute git command: {}", e))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(WorktreeError::GitCommand(format!(
                "Git command failed with status {}: {}",
                output.status, stderr
            )));
        }

        let output_str = String::from_utf8(output.stdout)?;
        Self::parse_worktree_output(&output_str)
    }

    /// Parse the porcelain output from `git worktree list --porcelain`
    fn parse_worktree_output(output: &str) -> Result<Vec<Worktree>> {
        let mut worktrees = Vec::new();
        let lines: Vec<&str> = output.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let mut current_worktree = WorktreeEntry::default();

            // Parse this worktree entry until we hit an empty line or EOF
            while i < lines.len() && !lines[i].is_empty() {
                let line = lines[i];

                if let Some(path) = line.strip_prefix("worktree ") {
                    current_worktree.path = Some(PathBuf::from(path));
                } else if let Some(head) = line.strip_prefix("HEAD ") {
                    current_worktree.head = Some(head.to_string());
                } else if let Some(branch) = line.strip_prefix("branch ") {
                    // Remove refs/heads/ prefix if present
                    let branch_name = branch.strip_prefix("refs/heads/").unwrap_or(branch);
                    current_worktree.branch = Some(branch_name.to_string());
                }

                i += 1;
            }

            // Convert to Worktree if we have the required fields
            if let Some(worktree) = current_worktree.into_worktree() {
                worktrees.push(worktree);
            }

            // Skip empty line
            if i < lines.len() && lines[i].is_empty() {
                i += 1;
            }
        }

        Ok(worktrees)
    }
}

#[derive(Default)]
struct WorktreeEntry {
    path: Option<PathBuf>,
    head: Option<String>,
    branch: Option<String>,
}

impl WorktreeEntry {
    fn into_worktree(self) -> Option<Worktree> {
        let path = self.path?;
        let head = self.head?;
        let branch = self.branch.unwrap_or_else(|| "(detached)".to_string());

        Some(Worktree { path, head, branch })
    }
}

// RUST LEARNING: Test module with conditional compilation
// - `#[cfg(test)]` only compiles when running `cargo test`
// - `mod tests` creates a nested module for test functions
#[cfg(test)]
mod tests {
    // RUST LEARNING: `use super::*;` imports everything from parent module
    // - Like `import * from '../'` but for the parent module
    use super::*;

    // RUST LEARNING: `#[test]` attribute marks functions as tests
    // - No test runner needed - built into Rust
    // - Tests run with `cargo test`
    #[test]
    fn test_parse_worktree_output() {
        // RUST LEARNING: Raw string literals with `r#"..."#`
        // - No need to escape quotes or backslashes inside
        // - Like template literals but for any string content
        let output = r#"worktree /home/user/project
HEAD 1234567890abcdef1234567890abcdef12345678
branch refs/heads/main

worktree /home/user/project-feature
HEAD abcdef1234567890abcdef1234567890abcdef12
branch refs/heads/feature-branch

worktree /home/user/project-detached
HEAD fedcba0987654321fedcba0987654321fedcba09
detached
"#;

        let worktrees = WorktreeManager::parse_worktree_output(output).unwrap();

        // RUST LEARNING: `assert_eq!()` macro for testing equality
        // - Built-in assertion macros (like Jest's expect().toBe())
        // - Panics with helpful diff if assertion fails
        assert_eq!(worktrees.len(), 3);

        // Test main worktree
        assert_eq!(worktrees[0].path, PathBuf::from("/home/user/project"));
        assert_eq!(worktrees[0].branch, "main");
        assert_eq!(
            worktrees[0].head,
            "1234567890abcdef1234567890abcdef12345678"
        );

        // Test feature branch worktree
        assert_eq!(
            worktrees[1].path,
            PathBuf::from("/home/user/project-feature")
        );
        assert_eq!(worktrees[1].branch, "feature-branch");
        assert_eq!(
            worktrees[1].head,
            "abcdef1234567890abcdef1234567890abcdef12"
        );

        // Test detached worktree
        assert_eq!(
            worktrees[2].path,
            PathBuf::from("/home/user/project-detached")
        );
        assert_eq!(worktrees[2].branch, "(detached)");
        assert_eq!(
            worktrees[2].head,
            "fedcba0987654321fedcba0987654321fedcba09"
        );
    }

    #[test]
    fn test_worktree_display_name() {
        let worktree = Worktree {
            path: PathBuf::from("/home/user/project/worktrees/feature"),
            branch: "feature-branch".to_string(),
            head: "abc123".to_string(),
        };

        let base_dir = PathBuf::from("/home/user/project");
        let display_name = worktree.display_name(&base_dir);

        assert_eq!(display_name, "feature-branch (worktrees/feature)");
    }

    #[test]
    fn test_worktree_is_main() {
        let main_worktree = Worktree {
            path: PathBuf::from("/home/user/project"),
            branch: "main".to_string(),
            head: "abc123".to_string(),
        };

        let feature_worktree = Worktree {
            path: PathBuf::from("/home/user/project/worktrees/feature"),
            branch: "feature".to_string(),
            head: "def456".to_string(),
        };

        let base_dir = PathBuf::from("/home/user/project");

        assert!(main_worktree.is_main(&base_dir));
        assert!(!feature_worktree.is_main(&base_dir));
    }
}
