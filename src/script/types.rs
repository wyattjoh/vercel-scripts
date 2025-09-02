use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf; // RUST LEARNING: PathBuf is like a mutable path (vs Path which is immutable)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptArg {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptRequirement {
    pub script: String,
    pub variables: Vec<String>,
}

// RUST LEARNING: Advanced enum with data - much more powerful than TypeScript enums
// - Each variant can have different fields (like tagged unions in TS)
// - `#[serde(tag = "type")]` creates a tagged union in JSON with a "type" field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")] // JSON will have { "type": "boolean", ... } format
pub enum ScriptOpt {
    // RUST LEARNING: Enum variants with struct-like syntax
    // - Like: type BooleanOpt = { type: 'boolean', name: string, ... }
    #[serde(rename = "boolean")]
    Boolean {
        name: String,
        description: String,
        default: Option<bool>,
        // RUST LEARNING: `#[serde(default)]` uses Default::default() if field is missing
        #[serde(default)] // Uses false if not present in JSON
        optional: bool,
    },
    #[serde(rename = "string")]
    String {
        name: String,
        description: String,
        default: Option<String>,
        #[serde(default)]
        optional: bool,
        pattern: Option<String>,
        pattern_help: Option<String>,
    },
    #[serde(rename = "worktree")]
    Worktree {
        name: String,
        description: String,
        // RUST LEARNING: `#[serde(alias = "...")]` accepts alternative field names
        // - Handles both "base_dir_arg" and "baseDirArg" in JSON
        #[serde(alias = "baseDirArg")]
        base_dir_arg: String,
        default: Option<String>,
        #[serde(default)]
        optional: bool,
    },
}

// RUST LEARNING: Implementing methods on enums (like adding methods to a union type)
// - `&self` means we're borrowing self (not taking ownership)
impl ScriptOpt {
    // RUST LEARNING: Returns `&str` (string slice) instead of owned String
    // - More efficient than cloning strings
    // - Like returning a readonly reference to the string data
    pub fn name(&self) -> &str {
        // RUST LEARNING: Pattern matching on enum variants
        // - `..` ignores other fields in the struct variants
        // - Like destructuring: const { name } = variant
        match self {
            ScriptOpt::Boolean { name, .. } => name,
            ScriptOpt::String { name, .. } => name,
            ScriptOpt::Worktree { name, .. } => name,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            ScriptOpt::Boolean { description, .. } => description,
            ScriptOpt::String { description, .. } => description,
            ScriptOpt::Worktree { description, .. } => description,
        }
    }

    pub fn is_optional(&self) -> bool {
        match self {
            // RUST LEARNING: `*optional` dereferences the boolean value
            // - `optional` is &bool (reference), we need bool (value)
            ScriptOpt::Boolean { optional, .. } => *optional,
            ScriptOpt::String { optional, .. } => *optional,
            ScriptOpt::Worktree { optional, .. } => *optional,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Script {
    pub name: String,
    pub description: Option<String>,
    pub after: Option<Vec<String>>,
    pub requires: Option<Vec<ScriptRequirement>>,
    pub absolute_pathname: PathBuf,
    pub pathname: String,
    pub embedded: bool,
    pub args: Option<Vec<ScriptArg>>,
    pub opts: Option<Vec<ScriptOpt>>,
    pub stdin: Option<String>,
}

impl fmt::Display for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {}{}{}",
            self.name,
            "(".bright_black(),
            self.pathname.bright_black(),
            ")".bright_black()
        )
    }
}
