# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

### Using Make (Recommended)
- `make build-release` - Compile optimized release binary
- `make install` - Build and install CLI globally as `vss` command
- `make test` - Run test suite
- `make fmt` - Format Rust code
- `make clippy` - Lint Rust code
- `make clean` - Clean build artifacts
- `make help` - Show all available make targets

### Using Cargo Directly
- `cargo build` - Compile debug binary
- `cargo build --release` - Compile optimized release binary
- `cargo run` - Run CLI from source
- `cargo install --path .` - Install CLI globally
- `cargo test` - Run test suite
- `cargo fmt` - Format Rust code
- `cargo clippy` - Lint Rust code

## Running the CLI

Run `cargo run` to launch the interactive script selector from source, or use `make install` to build and install the `vss` command globally.

## Architecture

This is a Rust CLI tool that provides an interactive script selector for Vercel project workflows. The main components:

- **`src/main.rs`** - Main CLI entry point using clap for argument parsing
- **`src/lib.rs`** - Library entry point with public API
- **`src/cli/`** - CLI module with user interaction (dialoguer) and script execution
- **`src/script/`** - Script parsing, dependency resolution, and management
- **`src/config.rs`** - JSON-based configuration persistence using serde
- **`src/commands/`** - CLI subcommands for script directory management
- **`src/worktree.rs`** - Git worktree utilities
- **`src/scripts/` directory** - Collection of bash scripts with metadata annotations

### Script Annotation System

Scripts use special comments for metadata:

```bash
# @vercel.name Script Name
# @vercel.description Description text
# @vercel.arg VARIABLE_NAME Description of the argument
# @vercel.opt { "name": "VAR", "description": "desc", "type": "boolean", "default": false }
# @vercel.after ./other_script.sh
```

Arguments become environment variables, and dependency ordering is handled automatically through the `@vercel.after` annotation.

### Configuration Files

- `~/.vss.json` - Global user arguments (persisted in home directory)
- `.vss-app.json` - Per-project selections and options (created in working directory)

The tool remembers previous selections and allows replay with `--replay` flag.

## Key Dependencies

- **clap** - Command-line argument parsing with subcommands
- **dialoguer** - Interactive prompts and multi-select interfaces
- **serde/serde_json** - Configuration serialization with camelCase compatibility
- **anyhow** - Simplified error handling and propagation
- **thiserror** - Structured error types for specific modules
- **petgraph** - Dependency graph resolution and topological sorting
- **dirs** - Cross-platform home directory detection
- **colored** - Terminal color output for better UX
- **comfy-table** - Pretty table formatting for script listings
- **regex** - Pattern matching for script parsing
- **include_dir** - Embed default scripts in the binary
- **tempfile** - Temporary file handling for tests

## Testing

Run the full test suite with `make test` or `cargo test`. Tests cover:
- Configuration file operations and serialization
- Script parsing and metadata extraction
- Dependency resolution and topological sorting
- Error handling scenarios
