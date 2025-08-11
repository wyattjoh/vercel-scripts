import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import colors from "yoctocolors";
import path from "node:path";

import { getScripts } from "../script.ts";

export const listScriptsCommand = new Command()
  .name("list-scripts")
  .alias("ls")
  .description("List all available scripts in a table format")
  .action(async () => {
    const scripts = await getScripts();

    if (scripts.length === 0) {
      console.log(colors.yellow("No scripts found."));
      console.log(
        colors.gray(
          "  Use 'vss add-script-dir <directory>' to add a directory with scripts",
        ),
      );
      return;
    }

    const table = new Table();

    table.maxColWidth(30);

    table.header([
      colors.bold("Name"),
      colors.bold("Description"),
      colors.bold("Source"),
      colors.bold("Arguments"),
      colors.bold("Options"),
    ]);

    for (const script of scripts) {
      const source = script.embedded
        ? colors.blue("embedded")
        : colors.green(path.dirname(script.absolutePathname));

      const args = script.args?.map((arg) => arg.name).join(", ") ||
        colors.gray("none");

      const opts = script.opts?.map((opt) => opt.name).join(", ") ||
        colors.gray("none");

      const description = script.description || colors.gray("No description");

      table.push([
        script.name,
        description,
        source,
        args,
        opts,
      ]);
    }

    table.border();
    table.padding(1);

    console.log(table.toString());
    console.log();
    console.log(
      colors.gray(
        `Total: ${scripts.length} script${scripts.length !== 1 ? "s" : ""}`,
      ),
    );
  });
