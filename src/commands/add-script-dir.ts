import { Command } from "@cliffy/command";
import path from "node:path";
import fs from "node:fs/promises";
import colors from "yoctocolors";
import { config } from "../config.ts";

export const addScriptDirCommand = new Command()
  .name("add-script-dir")
  .description("Add a directory to scan for scripts")
  .arguments("<directory:string>")
  .action(async (_options, directory: string) => {
    const absolutePath = path.resolve(directory);

    try {
      const stats = await fs.stat(absolutePath);
      if (!stats.isDirectory()) {
        console.error(colors.red(`Error: ${absolutePath} is not a directory`));
        Deno.exit(1);
      }
    } catch {
      console.error(
        colors.red(`Error: Directory ${absolutePath} does not exist`),
      );
      Deno.exit(1);
    }

    const scriptDirs = config.global.get("scriptDirs");

    if (scriptDirs.includes(absolutePath)) {
      console.log(
        colors.yellow(`Directory ${absolutePath} is already configured`),
      );
      return;
    }

    scriptDirs.push(absolutePath);
    config.global.set("scriptDirs", scriptDirs);

    console.log(colors.green(`✓ Added ${absolutePath} to script directories`));
  });
