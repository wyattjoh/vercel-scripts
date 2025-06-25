import { checkbox, confirm } from "@inquirer/prompts";
import { spawn } from "node:child_process";
import colors from "yoctocolors";
import path from "node:path";
import yargs from "yargs";
import { hideBin } from "yargs/helpers";
import fileSelector from "inquirer-file-selector";
import os from "node:os";
import updateNotifier from "update-notifier";
import packageJson from "../package.json" with { type: "json" };

import { Config } from "./config.js";
import { getScripts, type Script } from "./script.js";

const config = {
  global: new Config<{
    args: Record<string, unknown>;
  }>({
    file: path.resolve(import.meta.dirname, "..", ".vss-global.json"),
    defaults: {
      args: {},
    },
  }),
  app: new Config<{
    selected: string[];
    opts: Record<string, unknown>;
  }>({
    file: path.resolve(process.cwd(), ".vss-app.json"),
    defaults: {
      selected: [],
      opts: {},
    },
  }),
};

const availableColors = [
  colors.green,
  colors.yellow,
  colors.blue,
  colors.magenta,
  colors.cyan,
  colors.red,
];

const main = async () => {
  // Check for package updates on startup
  const notifier = updateNotifier({ pkg: packageJson });
  notifier.notify();

  const argv = await yargs(hideBin(process.argv)).options({
    replay: {
      type: "boolean",
      description: "Replay the last run",
      default: false,
    },
  }).argv;

  const persisted = config.app.get("selected");
  const scripts = await getScripts();

  let selected: Script[];
  if (argv.replay) {
    selected = scripts.filter((script) => persisted.includes(script.pathname));
  } else {
    selected = await checkbox({
      message: "Which scripts do you want to run?",
      choices: scripts.map((script) => ({
        value: script,
        name: script.name,
        description: script.description,
        checked: persisted.includes(script.pathname),
        short: script.pathname,
      })),
      loop: false,
      required: true,
      pageSize: scripts.length,
    });
  }

  // Persist the selected scripts to a file.
  config.app.set(
    "selected",
    selected.map((s) => s.pathname)
  );

  const args = config.global.get("args");
  const opts = config.app.get("opts");

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
    config.global.set("args", args);
  }

  // If we have any opts, persist them.
  if (Object.keys(opts).length > 0) {
    config.app.set("opts", opts);
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
    await new Promise<void>((resolve) => {
      child.on("close", (code) => {
        if (code === 0) {
          resolve();
        } else {
          process.exit(code);
        }
      });
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
