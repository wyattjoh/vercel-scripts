# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

- `pnpm install && pnpm build` - Install dependencies and build the TypeScript CLI
- `pnpm dev` - Watch mode development with tsup

## Running the CLI

Run `./bin/vss` from any project directory to launch the interactive script selector. The binary is built from TypeScript source in `src/` using tsup.

## Architecture

This is a TypeScript CLI tool that provides an interactive script selector for Vercel project workflows. The main components:

- **`src/main.ts`** - Main CLI entry point with interactive prompts using @inquirer/prompts
- **`src/script.ts`** - Script parser that reads bash scripts with special `@vercel.*` annotations and handles dependency ordering via topological sort
- **`src/config.ts`** - JSON-based configuration persistence for user selections and arguments
- **`scripts/` directory** - Collection of bash scripts with metadata annotations

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

- `.vss-global.json` - Global user arguments (persisted in tool directory)
- `.vss-app.json` - Per-project selections and options (created in working directory)

The tool remembers previous selections and allows replay with `--replay` flag.