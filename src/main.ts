import { spawn } from "node:child_process";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { Checkbox, Confirm, Input, Select } from "@cliffy/prompt";
import { Command } from "@cliffy/command";
import colors from "yoctocolors";

import { createConfig } from "./config.ts";
import { getScripts, prepareScript, type Script } from "./script.ts";
import { listWorktrees } from "./worktree.ts";
import deno from "../deno.json" with { type: "json" };
import { fileURLToPath } from "node:url";

const config = {
  global: createConfig<{
    args: Record<string, unknown>;
  }>(
    path.join(
      os.homedir(),
      ".vss.json",
    ),
    {
      args: {},
    },
  ),
  app: createConfig<{
    selected: string[];
    opts: Record<string, unknown>;
  }>(
    path.resolve(process.cwd(), ".vss-app.json"),
    {
      selected: [],
      opts: {},
    },
  ),
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
  const { options } = await new Command()
    .name("vss")
    .description("Vercel Scripts Selector")
    .version(deno.version)
    .option("-r, --replay", "Replay the last run")
    .parse(Deno.args);

  const persisted = config.app.get("selected");
  const scripts = await getScripts();

  let selected: Script[];
  if (options.replay) {
    selected = scripts.filter((script) => persisted.includes(script.pathname));
  } else {
    selected = await Checkbox.prompt({
      message: "Which scripts do you want to run?",
      options: scripts.map((script) => ({
        value: script,
        name: script.name,
        checked: persisted.includes(script.pathname),
      })),
    });
  }

  // Persist the selected scripts to a file.
  config.app.set(
    "selected",
    selected.map((s) => s.pathname),
  );

  const args = config.global.get("args");
  const opts = config.app.get("opts");

  for (const script of selected) {
    if (script.args && script.args.length > 0) {
      for (const arg of script.args) {
        // If we already have the argument, skip it.
        if (args[arg.name]) continue;

        const value = await Input.prompt({
          message:
            `Enter a directory path for ${arg.name} - ${arg.description}`,
          default: os.homedir(),
        });

        args[arg.name] = value;
      }
    }

    if (script.opts && script.opts.length > 0) {
      for (const opt of script.opts) {
        // If we already have the option, skip it.
        if (opts[opt.name] !== undefined) continue;

        if (opt.type === "boolean") {
          const value = await Confirm.prompt({
            message: opt.description,
            default: opt.default as boolean,
          });

          opts[opt.name] = value;
        } else if (opt.type === "worktree") {
          if (!opt.baseDirArg) {
            console.warn(`Worktree option ${opt.name} missing baseDirArg`);
            continue;
          }

          const baseDir = args[opt.baseDirArg];
          if (!baseDir || typeof baseDir !== "string") {
            console.warn(
              `Base directory ${opt.baseDirArg} not set, skipping ${opt.name}`,
            );
            continue;
          }

          const worktrees = listWorktrees(baseDir);
          const choices = [
            { value: null, name: "(Use base directory)" },
            ...worktrees.map((wt) => ({
              value: wt.path,
              name: `${wt.branch} (${path.relative(baseDir, wt.path)})`,
            })),
          ];

          // Only prompt if there are worktrees or if not optional
          if (worktrees.length > 0 || !opt.optional) {
            const value = await Select.prompt({
              message: opt.description,
              options: choices.map((choice) => ({
                name: choice.name,
                value: choice.value,
              })),
              default: (opt.default as string | null) ?? null,
            });
            opts[opt.name] = value;
          }
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
        if (value === null || value === undefined) {
          // Don't set the env var if null/undefined
          continue;
        }
        if (typeof value === "string") {
          env[opt.name] = value;
        } else if (typeof value === "boolean") {
          env[opt.name] = value.toString();
        }

        console.log(color(`    ${opt.name}: ${env[opt.name]}`));
      }
    }

    const child = spawn(
      // The execute.sh script is embedded in the VSS package, so we need to
      // prepare it to run in the temporary directory.
      await prepareScript(
        fileURLToPath(import.meta.resolve("./runtime/runtime.sh")),
        "runtime",
        true,
      ),
      [
        // If the script is embedded, prepare it to run in the temporary
        // directory. We can't execute it directly because the script will not
        // be able to see the other script files.
        script.embedded
          ? await prepareScript(script.absolutePathname, "script")
          : script.absolutePathname,
      ],
      {
        shell: true,
        env,
        stdio: script.stdin === "inherit"
          ? "inherit"
          : ["inherit", "pipe", "pipe"],
      },
    );

    const output: string[] = [];

    // Log the script's stdout and stderr only if using piped stdio.
    if (script.stdin !== "inherit") {
      child.stdout?.on("data", (data) => {
        const lines = data.toString().trim().split("\n");
        output.push(...lines);

        for (const line of lines) {
          console.log(color(`[${script.pathname}]`), line);
        }
      });

      child.stderr?.on("data", (data) => {
        const lines = data.toString().trim().split("\n");

        for (const line of lines) {
          console.log(color(`[${script.pathname}]`), line);
        }
      });
    }

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

if (import.meta.main) {
  main().catch((error) => {
    if (error instanceof Error && error.name === "ExitPromptError") {
      // This was a CTRL-C, so exit without error.
    } else {
      // Rethrow unknown errors
      throw error;
    }
  });
}
