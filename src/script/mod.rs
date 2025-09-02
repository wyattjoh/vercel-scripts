pub mod manager;
pub mod parser;
pub mod types;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Circular dependency detected")]
    CircularDependency,
    #[error("Dependency not found: {0}")]
    DependencyNotFound(String),
    #[error("Invalid script option: {0}")]
    InvalidScriptOption(String),
    #[error("Invalid path - cannot extract filename: {0}")]
    InvalidPath(std::path::PathBuf),
}

pub type Result<T> = std::result::Result<T, ScriptError>;

pub use manager::ScriptManager;
pub use types::{Script, ScriptOpt};

#[cfg(test)]
mod tests {
    use super::*;
    use parser::ScriptParser;
    use std::path::Path;

    #[test]
    fn test_script_parser() {
        let content = r#"#!/bin/bash
# @vercel.name Test Script
# @vercel.description This is a test script
# @vercel.arg TEST_ARG This is a test argument
# @vercel.opt { "name": "TEST_BOOL", "description": "Test boolean", "type": "boolean", "default": false }
echo "Hello World"
"#;

        let path = Path::new("test.sh");
        let script = ScriptParser::parse_script(content, path, false).unwrap();

        assert_eq!(script.name, "Test Script");
        assert_eq!(
            script.description,
            Some("This is a test script".to_string())
        );
        assert!(script.args.is_some());
        assert!(script.opts.is_some());

        let args = script.args.unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args[0].name, "TEST_ARG");

        let opts = script.opts.unwrap();
        assert_eq!(opts.len(), 1);
        assert_eq!(opts[0].name(), "TEST_BOOL");
    }

    #[test]
    fn test_script_parser_invalid_path() {
        let content = r#"#!/bin/bash
echo "Hello World"
"#;

        // Create a path that can't be converted to a valid filename
        // Using an empty path should fail filename extraction
        let path = Path::new("");
        let result = ScriptParser::parse_script(content, path, false);

        assert!(result.is_err());
        match result.unwrap_err() {
            ScriptError::InvalidPath(invalid_path) => {
                assert_eq!(invalid_path, Path::new(""));
            }
            _ => panic!("Expected InvalidPath error"),
        }
    }

    #[test]
    fn test_embedded_scripts_loading() {
        let mut manager = ScriptManager::new();
        let scripts = manager.load_embedded_scripts().unwrap();

        // Should load at least some scripts from the embedded directory
        assert!(!scripts.is_empty());

        // All should be marked as embedded
        for script in &scripts {
            assert!(script.embedded);
        }
    }

    #[test]
    fn test_prepare_script_embedded() {
        let mut manager = ScriptManager::new();
        let scripts = manager.load_embedded_scripts().unwrap();

        // Take the first script as a test case
        let script = &scripts[0];
        let prefix = "test-prefix";

        let prepared_path = manager.prepare_script(script, prefix).unwrap();

        // Verify the path structure matches expected pattern: cache_dir/prefix/basename
        assert!(prepared_path.exists());
        assert!(prepared_path.is_file());

        // Check that the path contains the prefix as a directory
        let parent = prepared_path.parent().unwrap();
        assert_eq!(parent.file_name().unwrap(), prefix);

        // Check that the filename is preserved
        assert_eq!(
            prepared_path.file_name().unwrap(),
            script.absolute_pathname.file_name().unwrap()
        );

        // Verify the file is executable on Unix systems
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&prepared_path).unwrap();
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }
    }

    #[test]
    fn test_prepare_script_external() {
        use std::fs;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("external_script.sh");

        // Create a test script
        let script_content = r#"#!/bin/bash
# @vercel.name External Test Script
echo "Hello from external script"
"#;
        fs::write(&script_path, script_content).unwrap();

        // Parse the script
        let script =
            parser::ScriptParser::parse_script(script_content, &script_path, false).unwrap();

        let mut manager = ScriptManager::new();
        let prefix = "external-test";

        let prepared_path = manager.prepare_script(&script, prefix).unwrap();

        // Verify the path structure
        assert!(prepared_path.exists());
        assert!(prepared_path.is_file());

        // Check directory structure: cache_dir/prefix/basename
        let parent = prepared_path.parent().unwrap();
        assert_eq!(parent.file_name().unwrap(), prefix);
        assert_eq!(prepared_path.file_name().unwrap(), "external_script.sh");

        // Verify content is copied correctly
        let copied_content = fs::read_to_string(&prepared_path).unwrap();
        assert_eq!(copied_content, script_content);
    }

    #[test]
    fn test_prepare_script_multiple_prefixes() {
        let mut manager = ScriptManager::new();
        let scripts = manager.load_embedded_scripts().unwrap();
        let script = &scripts[0];

        // Prepare the same script with different prefixes
        let prefix1 = "prefix-one";
        let prefix2 = "prefix-two";

        let path1 = manager.prepare_script(script, prefix1).unwrap();
        let path2 = manager.prepare_script(script, prefix2).unwrap();

        // Both should exist and be in different directories
        assert!(path1.exists());
        assert!(path2.exists());
        assert_ne!(path1, path2);

        // Check they're in different subdirectories
        let parent1 = path1.parent().unwrap();
        let parent2 = path2.parent().unwrap();
        assert_eq!(parent1.file_name().unwrap(), prefix1);
        assert_eq!(parent2.file_name().unwrap(), prefix2);

        // But same filename
        assert_eq!(path1.file_name(), path2.file_name());
    }

    #[test]
    fn test_prepare_script_content_caching() {
        use std::fs;

        let mut manager = ScriptManager::new();
        let scripts = manager.load_embedded_scripts().unwrap();
        let script = &scripts[0];
        let prefix = "cache-test";

        // Prepare script first time
        let prepared_path = manager.prepare_script(script, prefix).unwrap();
        assert!(prepared_path.exists());

        // Get original modification time
        let original_metadata = fs::metadata(&prepared_path).unwrap();
        let original_modified = original_metadata.modified().unwrap();

        // Wait a bit to ensure different timestamp if file gets rewritten
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Prepare the same script again
        let prepared_path2 = manager.prepare_script(script, prefix).unwrap();
        assert_eq!(prepared_path, prepared_path2);

        // Check modification time hasn't changed (file wasn't rewritten)
        let new_metadata = fs::metadata(&prepared_path2).unwrap();
        let new_modified = new_metadata.modified().unwrap();
        assert_eq!(original_modified, new_modified);
    }

    #[test]
    fn test_prepare_runtime_script() {
        let mut manager = ScriptManager::new();

        let runtime_path = manager.prepare_runtime().unwrap();

        assert!(runtime_path.exists());
        assert!(runtime_path.is_file());
        assert_eq!(runtime_path.file_name().unwrap(), "runtime.sh");

        // Verify it's executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(&runtime_path).unwrap();
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }

        // Verify content is not empty
        let content = std::fs::read_to_string(&runtime_path).unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_prepare_script_invalid_path() {
        use std::fs;
        use tempfile::TempDir;

        let mut manager = ScriptManager::new();

        // Create a temporary directory to simulate the issue
        let temp_dir = TempDir::new().unwrap();

        // Create a script with an invalid path (directory instead of file)
        let dir_path = temp_dir.path().join("not-a-file");
        fs::create_dir_all(&dir_path).unwrap(); // Create as directory

        let invalid_script = Script {
            name: "Invalid Script".to_string(),
            description: None,
            after: None,
            absolute_pathname: dir_path, // This is a directory, not a file
            pathname: "invalid".to_string(),
            embedded: false,
            args: None,
            opts: None,
            stdin: None,
        };

        let result = manager.prepare_script(&invalid_script, "test-prefix");

        assert!(result.is_err());
        // For directories, file_name() returns Some(...) but reading fails
        // Let's just verify that the operation fails appropriately
        match result.unwrap_err() {
            ScriptError::Io(_) | ScriptError::InvalidPath(_) => {} // Both are acceptable
            other => panic!("Expected IO or InvalidPath error, got: {:?}", other),
        }
    }

    #[test]
    fn test_prepare_script_no_filename_extraction() {
        use std::path::PathBuf;

        let mut manager = ScriptManager::new();

        // Create a script with a path that can't have filename extracted
        // Using an empty PathBuf should trigger the InvalidPath error
        let invalid_script = Script {
            name: "No Filename Script".to_string(),
            description: None,
            after: None,
            absolute_pathname: PathBuf::new(), // Empty path - no filename
            pathname: "empty".to_string(),
            embedded: false,
            args: None,
            opts: None,
            stdin: None,
        };

        let result = manager.prepare_script(&invalid_script, "test-prefix");

        assert!(result.is_err());
        match result.unwrap_err() {
            ScriptError::InvalidPath(_) => {} // This is what we expect
            other => panic!("Expected InvalidPath error, got: {:?}", other),
        }
    }
}
