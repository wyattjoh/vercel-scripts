import { checkbox, confirm } from "@inquirer/prompts";
import { spawn } from "node:child_process";
import colors from "yoctocolors";
import path from "node:path";
import fs from "node:fs/promises";
import yargs from "yargs";
import { hideBin } from "yargs/helpers";
import fileSelector from "inquirer-file-selector";
import os from "node:os";

import { getScripts, type Script } from "./script.js";

const availableColors = [
  colors.green,
  colors.yellow,
  colors.blue,
  colors.magenta,
  colors.cyan,
  colors.red,
];

async function persistSelectedScripts(scripts: Script[]) {
  const dir = path.resolve(process.cwd(), ".vss");
  await fs.mkdir(dir, { recursive: true });

  const selectedFilePath = path.resolve(dir, "selected.txt");
  await fs.writeFile(
    selectedFilePath,
    scripts.map((s) => s.pathname).join("\n")
  );
}

async function loadSelectedScripts(): Promise<string[]> {
  try {
    const dir = path.resolve(process.cwd(), ".vss");
    const selectedFilePath = path.resolve(dir, "selected.txt");
    const selected = await fs.readFile(selectedFilePath, "utf-8");
    return selected.split("\n");
  } catch {
    return [];
  }
}

async function getPersistedArgs(): Promise<Record<string, unknown>> {
  const argsFilePath = path.resolve(import.meta.dirname, "..", "args.json");
  try {
    const args = await fs.readFile(argsFilePath, "utf-8");
    return JSON.parse(args);
  } catch {
    return {};
  }
}

async function persistArgs(args: Record<string, unknown>) {
  const argsFilePath = path.resolve(import.meta.dirname, "..", "args.json");
  await fs.writeFile(argsFilePath, JSON.stringify(args, null, 2));
}

async function getPersistedOpts(): Promise<Record<string, unknown>> {
  const optsFilePath = path.resolve(process.cwd(), ".vss", "opts.json");
  try {
    const opts = await fs.readFile(optsFilePath, "utf-8");
    return JSON.parse(opts);
  } catch {
    return {};
  }
}

async function persistOpts(opts: Record<string, unknown>) {
  const optsFilePath = path.resolve(process.cwd(), ".vss", "opts.json");
  await fs.writeFile(optsFilePath, JSON.stringify(opts, null, 2));
}

const main = async () => {
  const argv = await yargs(hideBin(process.argv)).options({
    replay: {
      type: "boolean",
      description: "Replay the last run",
      default: false,
    },
  }).argv;

  const persisted = await loadSelectedScripts();
  const scripts = await getScripts();

  let selected: Script[];
  if (argv.replay) {
    selected = scripts.filter((script) => persisted.includes(script.pathname));
  } else {
    selected = await checkbox({
      message: "Which scripts do you want to run?",
      choices: scripts.map((script, i) => ({
        value: script,
        name: script.name,
        description: script.description,
        checked: persisted.includes(script.pathname),
      })),
    });
  }

  // Persist the selected scripts to a file.
  await persistSelectedScripts(selected);

  const args = await getPersistedArgs();
  const opts = await getPersistedOpts();

  for (const script of selected) {
    if (script.args && script.args.length > 0) {
      for (const arg of script.args) {
        // If we already have the argument, skip it.
        if (args[arg.name]) continue;

        const value = await fileSelector({
          message: `Enter a value for ${arg.name} - ${arg.description}`,
          type: "directory",
          basePath: os.homedir(),
        });

        args[arg.name] = value;
      }
    }

    if (script.opts && script.opts.length > 0) {
      for (const opt of script.opts) {
        // If we already have the option, skip it.
        if (opts[opt.name]) continue;

        if (opt.type === "boolean") {
          const value = await confirm({
            message: opt.description,
            default: opt.default,
          });

          opts[opt.name] = value;
        }
      }
    }
  }

  // If we have any args, persist them.
  if (Object.keys(args).length > 0) {
    await persistArgs(args);
  }

  // If we have any opts, persist them.
  if (Object.keys(opts).length > 0) {
    await persistOpts(opts);
  }

  // Run the selected scripts synchronously in order.
  let colorIndex = 0;
  for (const script of selected) {
    // Rotate the colors for each script for ease of reading.
    const color = availableColors[colorIndex];

    console.log(color(`✨ Running ${script.name}...`));

    // Get the script arguments if it's required.
    const env: NodeJS.ProcessEnv = { ...process.env };
    if (script.args && script.args.length > 0) {
      for (const arg of script.args) {
        const value = args[arg.name];
        if (typeof value === "string") {
          env[arg.name] = value;
        } else if (typeof value === "boolean") {
          env[arg.name] = value.toString();
        }

        console.log(color(`    ${arg.name}: ${env[arg.name]}`));
      }
    }

    if (script.opts && script.opts.length > 0) {
      for (const opt of script.opts) {
        const value = opts[opt.name];
        if (typeof value === "string") {
          env[opt.name] = value;
        } else if (typeof value === "boolean") {
          env[opt.name] = value.toString();
        }

        console.log(color(`    ${opt.name}: ${env[opt.name]}`));
      }
    }

    const child = spawn(
      path.resolve(import.meta.dirname, "..", "bin", "vss"),
      ["--run", script.pathname],
      { shell: true, env }
    );

    const output: string[] = [];

    // Log the script's stdout and stderr.
    child.stdout.on("data", (data) => {
      const lines = data.toString().trim().split("\n");
      output.push(...lines);

      for (const line of lines) {
        console.log(color(`[${script.pathname}]`), line);
      }
    });

    child.stderr.on("data", (data) => {
      const lines = data.toString().trim().split("\n");

      for (const line of lines) {
        console.log(color(`[${script.pathname}]`), line);
      }
    });

    // Wait for the script to finish.
    await new Promise((resolve) => {
      child.on("close", resolve);
    });

    colorIndex = (colorIndex + 1) % availableColors.length;
  }
};

main().catch((error) => {
  if (error instanceof Error && error.name === "ExitPromptError") {
    // This was a CTRL-C, so exit without error.
  } else {
    // Rethrow unknown errors
    throw error;
  }
});
