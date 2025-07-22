# Vercel Scripts

An interactive CLI tool for managing Vercel and Next.js development workflows through a collection of reusable scripts.

## Features

- **Interactive Script Selection** - Choose which scripts to run with checkboxes
- **Smart Dependencies** - Scripts automatically run in the correct order based on dependencies
- **Persistent Configuration** - Remembers your selections and arguments between runs
- **Environment Variables** - Script arguments are passed as environment variables
- **Replay Mode** - Re-run your last selection with `vss --replay`

## Prerequisites

- Node.js >=20.0.0
- pnpm
- zsh shell
- jq (for JSON processing in scripts)

## Setup

1. **Install and build:**
   ```bash
   pnpm install && pnpm build
   ```

2. **Add to PATH:**
   ```bash
   export PATH="$PATH:/path/to/vercel-scripts/bin"
   ```

3. **Run the CLI:**
   ```bash
   vss
   ```
   Run from any project directory to launch the interactive script selector.

## Usage

The tool will prompt you to select scripts and provide any required arguments (like directory paths). Your selections and arguments are persisted for future runs.

**Commands:**
- `vss` - Interactive script selector
- `vss --replay` - Re-run the last selection without prompts

## Adding New Scripts

Create a bash script in the `scripts/` directory with metadata annotations:

```bash
#!/bin/bash

# @vercel.name Your Script Name
# @vercel.description What this script does
# @vercel.arg VARIABLE_NAME Description of required argument
# @vercel.opt { "name": "OPTION_NAME", "description": "Optional setting", "type": "boolean", "default": false }
# @vercel.after ./dependency_script.sh

# Your script logic here
```

Make it executable: `chmod +x scripts/your_script.sh`

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



## Generating Documentation

This README is automatically generated from script metadata. To regenerate:

```bash
pnpm build && node dist/generate-readme.js
```

The script reads `@vercel.*` annotations from all scripts in the `scripts/` directory and builds comprehensive documentation including dependencies, arguments, and options.
