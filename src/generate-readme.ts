import fs from "node:fs/promises";
import path from "node:path";
import { getScripts } from "./script.js";

async function generateReadme() {
  const projectRoot = path.resolve(import.meta.dirname, "..");
  const packageJsonPath = path.join(projectRoot, "package.json");
  const readmePath = path.join(projectRoot, "README.md");

  // Read package.json for project info
  const packageJson = JSON.parse(await fs.readFile(packageJsonPath, "utf-8"));
  const nodeVersion = packageJson.engines?.node || ">= 20.0.0";

  // Get all scripts with parsed metadata
  const scripts = await getScripts();

  // Generate script documentation
  let scriptDocs = "## Available Scripts\n\n";
  scriptDocs += "The tool includes the following pre-configured scripts:\n\n";

  for (const script of scripts) {
    scriptDocs += `### ${script.name}\n\n`;

    if (script.description) {
      scriptDocs += `${script.description}\n\n`;
    }

    // Show arguments if any
    if (script.args && script.args.length > 0) {
      scriptDocs += "**Required Arguments:**\n";
      for (const arg of script.args) {
        scriptDocs += `- \`${arg.name}\`: ${arg.description}\n`;
      }
      scriptDocs += "\n";
    }

    // Show options if any
    if (script.opts && script.opts.length > 0) {
      scriptDocs += "**Optional Parameters:**\n";
      for (const opt of script.opts) {
        scriptDocs += `- \`${opt.name}\`: ${opt.description} (default: ${opt.default})\n`;
      }
      scriptDocs += "\n";
    }

    // Show dependencies if any
    if (script.afterAbsolutePathnames && script.afterAbsolutePathnames.length > 0) {
      const deps = script.afterAbsolutePathnames
        .map((p) => `./${path.basename(p)}`)
        .join(", ");
      scriptDocs += `**Dependencies:** Runs after ${deps}\n\n`;
    }
  }

  // Generate complete README
  const readme = `# Vercel Scripts

An interactive CLI tool for managing Vercel and Next.js development workflows through a collection of reusable scripts.

## Features

- **Interactive Script Selection** - Choose which scripts to run with checkboxes
- **Smart Dependencies** - Scripts automatically run in the correct order based on dependencies
- **Persistent Configuration** - Remembers your selections and arguments between runs
- **Environment Variables** - Script arguments are passed as environment variables
- **Replay Mode** - Re-run your last selection with \`vss --replay\`

## Prerequisites

- Node.js ${nodeVersion}
- pnpm
- zsh shell
- jq (for JSON processing in scripts)

## Setup

1. **Install and build:**
   \`\`\`bash
   pnpm install && pnpm build
   \`\`\`

2. **Add to PATH:**
   \`\`\`bash
   export PATH="$PATH:/path/to/vercel-scripts/bin"
   \`\`\`

3. **Run the CLI:**
   \`\`\`bash
   vss
   \`\`\`
   Run from any project directory to launch the interactive script selector.

## Usage

The tool will prompt you to select scripts and provide any required arguments (like directory paths). Your selections and arguments are persisted for future runs.

**Commands:**
- \`vss\` - Interactive script selector
- \`vss --replay\` - Re-run the last selection without prompts

## Adding New Scripts

Create a bash script in the \`scripts/\` directory with metadata annotations:

\`\`\`bash
#!/bin/bash

# @vercel.name Your Script Name
# @vercel.description What this script does
# @vercel.arg VARIABLE_NAME Description of required argument
# @vercel.opt { "name": "OPTION_NAME", "description": "Optional setting", "type": "boolean", "default": false }
# @vercel.after ./dependency_script.sh

# Your script logic here
\`\`\`

Make it executable: \`chmod +x scripts/your_script.sh\`

${scriptDocs}

## Generating Documentation

This README is automatically generated from script metadata. To regenerate:

\`\`\`bash
pnpm build && node dist/generate-readme.js
\`\`\`

The script reads \`@vercel.*\` annotations from all scripts in the \`scripts/\` directory and builds comprehensive documentation including dependencies, arguments, and options.
`;

  await fs.writeFile(readmePath, readme);
  console.log("✅ README.md generated successfully");
}

// Run the script
generateReadme().catch(console.error);