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
}
