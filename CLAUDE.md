# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

- `deno task build` - Compile TypeScript CLI to standalone binary
- `deno task dev` - Watch mode development with Deno
- `deno fmt` - Format TypeScript code
- `deno lint` - Lint TypeScript code

## Running the CLI

Run `deno run --allow-all src/main.ts` to launch the interactive script selector from source, or compile with `deno task build` to create a standalone binary.

## Architecture

This is a Deno-based TypeScript CLI tool that provides an interactive script selector for Vercel project workflows. The main components:

- **`src/main.ts`** - Main CLI entry point with interactive prompts using @cliffy/prompt
- **`src/script.ts`** - Script parser that reads bash scripts with special `@vercel.*` annotations and handles dependency ordering via topological sort
- **`src/config.ts`** - JSON-based configuration persistence for user selections and arguments
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

- `.vss-global.json` - Global user arguments (persisted in tool directory)
- `.vss-app.json` - Per-project selections and options (created in working directory)

The tool remembers previous selections and allows replay with `--replay` flag.
