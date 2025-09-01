//! # Vercel Scripts Selector (VSS)
//!
//! A Rust library and CLI tool for interactive script selection and execution
//! with dependency management for Vercel development workflows.
//!
//! ## Library Usage
//!
//! This crate can be used both as a library and as a CLI binary. The library provides
//! core functionality for script parsing, management, and execution.
//!
//! RUST LEARNING: `//!` comments are "inner doc comments" for modules/crates
//! - Like JSDoc but built into the language and used by `cargo doc`
//! - `//` is regular comment, `///` is doc comment for items, `//!` is for the containing item

// RUST LEARNING: Module declarations - different from TypeScript imports
// - `pub mod` declares a public module (like exporting a namespace)
// - These refer to files/directories: `cli.rs` or `cli/mod.rs`
// - Unlike TS, you must explicitly declare modules in a parent file
pub mod cli;
pub mod commands;
pub mod config;
pub mod script;
pub mod worktree;

// RUST LEARNING: `pub use` re-exports items (like TypeScript's `export { ... } from`)
// - Makes internal modules' items available at the crate root
// - Users can do `use vss::Config` instead of `use vss::config::Config`
// - Like creating a public API surface
pub use config::Config;
pub use script::{Script, ScriptManager, ScriptOpt};
// Export ScriptArg for users who need access to script arguments
pub use cli::runner::{check_for_updates, run_scripts};
pub use script::types::ScriptArg;
pub use worktree::{Worktree, WorktreeManager};

// Re-export command types for library users who want to use commands programmatically
pub use commands::{
    AddScriptDirCommand, CompletionsCommand, ListScriptDirsCommand, ListScriptsCommand,
    RemoveScriptDirCommand,
};

// RUST LEARNING: `/// ` is a doc comment for the following item (like TSDoc)
/// The current version of the crate
// RUST LEARNING: `const` creates compile-time constants (like `const` in TS)
// - `&str` is a string slice (like `string` but points to existing data)
// - `env!()` is a macro that reads environment variables at compile time
// - `CARGO_PKG_VERSION` is automatically set from Cargo.toml
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
