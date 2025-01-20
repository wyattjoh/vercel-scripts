import { checkbox } from "@inquirer/prompts";
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

async function loadSelectedScripts() {
  try {
    const dir = path.resolve(process.cwd(), ".vss");
    const selectedFilePath = path.resolve(dir, "selected.txt");
    const selected = await fs.readFile(selectedFilePath, "utf-8");
    return selected.split("\n");
  } catch {
    return [];
  }
}

async function getPersistedArgs() {
  const argsFilePath = path.resolve(import.meta.dirname, "..", "args.json");
  try {
    const args = await fs.readFile(argsFilePath, "utf-8");
    return JSON.parse(args);
  } catch {
    return {};
  }
}

async function persistArgs(args: Record<string, string>) {
  const argsFilePath = path.resolve(import.meta.dirname, "..", "args.json");
  await fs.writeFile(argsFilePath, JSON.stringify(args, null, 2));
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

  // Run the selected scripts synchronously in order.
  let colorIndex = 0;
  for (const script of selected) {
    // Rotate the colors for each script for ease of reading.
    const color = availableColors[colorIndex];

    // Get the script arguments if it's required.
    const env: Record<string, string> = {};
    if (script.args && script.args.length > 0) {
      const args = await getPersistedArgs();
      for (const arg of script.args) {
        if (args[arg.name]) {
          env[arg.name] = args[arg.name];
          continue;
        }

        const value = await fileSelector({
          message: `Enter a value for ${arg.name} - ${arg.description}`,
          type: "directory",
          basePath: os.homedir(),
        });

        env[arg.name] = value;
        args[arg.name] = value;
      }

      await persistArgs(args);
    }

    console.log(color(`✨ Running ${script.name}...`));
    const child = spawn(
      path.resolve(import.meta.dirname, "..", "bin", "vss"),
      ["--run", script.pathname],
      { shell: true, env: { ...process.env, ...env } }
    );

    // Log the script's stdout and stderr.
    child.stdout.on("data", (data) => {
      const lines = data.toString().trim().split("\n");

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
