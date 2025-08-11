import { Command } from "@cliffy/command";
import path from "node:path";
import colors from "yoctocolors";
import { config } from "../config.ts";
import { Select } from "@cliffy/prompt";

export const removeScriptDirCommand = new Command()
  .name("remove-script-dir")
  .alias("rm-dir")
  .description("Remove a directory from script scanning")
  .arguments("[directory:string]")
  .action(async (_options, directory?: string) => {
    const scriptDirs = config.global.get("scriptDirs");

    if (scriptDirs.length === 0) {
      console.log(colors.yellow("No external directories configured"));
      return;
    }

    let dirToRemove: string | undefined;

    if (directory) {
      const absolutePath = path.resolve(directory);
      if (scriptDirs.includes(absolutePath)) {
        dirToRemove = absolutePath;
      } else {
        console.error(
          colors.red(`Directory ${absolutePath} is not configured`),
        );
        Deno.exit(1);
      }
    } else {
      dirToRemove = await Select.prompt({
        message: "Select a directory to remove:",
        options: scriptDirs.map((dir, index) => ({
          name: `[${index + 1}] ${dir}`,
          value: dir,
        })),
      });
    }

    if (!dirToRemove) return;

    const updatedDirs = scriptDirs.filter((dir) => dir !== dirToRemove);
    config.global.set("scriptDirs", updatedDirs);

    console.log(
      colors.green(`✓ Removed ${dirToRemove} from script directories`),
    );
  });
