import { Command } from "@cliffy/command";
import colors from "yoctocolors";

import { config } from "../config.ts";

export const listScriptDirsCommand = new Command()
  .name("list-script-dirs")
  .alias("list-dirs")
  .alias("dirs")
  .description("List all configured script directories")
  .action(() => {
    const scriptDirs = config.global.get("scriptDirs");

    console.log(colors.bold("Script Directories:"));

    if (scriptDirs.length === 0) {
      console.log(colors.gray("  No external directories configured"));
      console.log();
      console.log(
        colors.gray(
          "  Use 'vss add-script-dir <directory>' to add a directory",
        ),
      );
    } else {
      scriptDirs.forEach((dir, index) => {
        console.log(colors.blue(`  [${index + 1}] `) + dir);
      });
    }
  });
