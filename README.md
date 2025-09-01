# Vercel Scripts

A powerful Rust CLI tool for managing Vercel and Next.js development workflows through an interactive collection of reusable scripts.

## Installation

```shell
brew install wyattjoh/stable/vercel-scripts
```

## Features

- **Interactive Script Selection** - Choose which scripts to run with checkboxes
- **Smart Dependencies** - Scripts automatically run in the correct order based on dependencies
- **Persistent Configuration** - Remembers your selections and arguments between runs
- **Environment Variables** - Script arguments are passed as environment variables
- **Replay Mode** - Re-run your last selection with `vss --replay`
- **Git Worktree Support** - Seamlessly work with multiple Git worktrees

## Requirements

- [Rust](https://rustup.rs/) (for building from source)
- [next-dev-utilities](https://github.com/wyattjoh/next-dev-utils)
- jq - for JSON processing in scripts
- git - for worktree functionality

## Usage

The tool provides an interactive interface to select and execute development scripts. Your selections and arguments are persisted for future runs.

**Commands:**

- `vss` - Interactive script selector
- `vss --replay` - Re-run the last selection without prompts
- `vss --help` - Show help information

## Configuration

The tool creates configuration files to persist your settings:

- `~/.vss-global.json` - Global user arguments (persisted in tool directory)
- `.vss-app.json` - Per-project selections and options (created in working directory)

## Adding New Scripts

Create a bash script in the `src/scripts/` directory with metadata annotations:

```bash
#!/bin/bash

# @vercel.name Your Script Name
# @vercel.description What this script does
# @vercel.arg VARIABLE_NAME Description of required argument
# @vercel.opt { "name": "OPTION_NAME", "description": "Optional setting", "type": "boolean", "default": false }
# @vercel.after ./dependency_script.sh

# Your script logic here
```

Make it executable: `chmod +x src/scripts/your_script.sh`

## Available Scripts

The tool includes the following pre-configured scripts:

### Build Next.js

Build the Next.js project

**Required Arguments:**

- `VERCEL_NEXT_DIRECTORY`: The directory for the vercel/next.js repo

**Optional Parameters:**

- `VERCEL_NEXT_WORKTREE`: Select Next.js worktree to build (default: null)

### Build Vercel CLI

Build the vercel CLI

**Required Arguments:**

- `VERCEL_VERCEL_DIRECTORY`: The directory for the vercel/vercel repo

### Link Local Next.js

Install the local Next.js package as the project's dependency

**Required Arguments:**

- `VERCEL_NEXT_DIRECTORY`: The directory for the vercel/next.js repo

**Dependencies:** Runs after ./build_next.sh

### Package Next.js

Package the Next.js project

**Required Arguments:**

- `NEXT_DEV_UTILS_DIRECTORY`: The directory for the wyattjoh/next-dev-utils repo

**Dependencies:** Runs after ./build_next.sh

### Build Project for Local (next build)

Build the project for local testing (not deployment).

**Dependencies:** Runs after ./link_local_next.sh

### Build Project for Deploy (vc build)

Build the project for deployment.

**Required Arguments:**

- `VERCEL_VERCEL_DIRECTORY`: The directory for the vercel/vercel repo

**Optional Parameters:**

- `VERCEL_BUILD_PRODUCTION`: Build the project in production mode (default: false)

**Dependencies:** Runs after ./package_next.sh, ./package_vercel.sh, ./link_next.sh

### Start Local Project (next start)

Start the project for local testing (not deployment).

**Dependencies:** Runs after ./build_local_project.sh

### Deploy Project (vc deploy --prebuilt)

Deploy the project using the prebuilt build.

**Required Arguments:**

- `VERCEL_VERCEL_DIRECTORY`: The directory for the vercel/vercel repo

**Optional Parameters:**

- `VERCEL_BUILD_PRODUCTION`: Build the project in production mode (default: false)

**Dependencies:** Runs after ./build_project.sh

## Development

To contribute or modify the CLI:

```bash
# Clone the repository
git clone https://github.com/wyattjoh/vercel-scripts.git
cd vercel-scripts

# Run in development
cargo run

# Build release binary
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint code
cargo clippy
```

The CLI reads `@vercel.*` annotations from all scripts in the `src/scripts/` directory and builds comprehensive documentation including dependencies, arguments, and options.

## License

MIT License - see [LICENSE](LICENSE) file for details.
