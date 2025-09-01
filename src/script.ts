import fs from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { z } from "zod";
import { fileURLToPath } from "node:url";

const ScriptArgSchema = z.object({
  /**
   * The name of the argument.
   */
  name: z.string(),

  /**
   * The description of the argument.
   */
  description: z.string(),
});

export type ScriptArg = z.infer<typeof ScriptArgSchema>;

const ScriptOptSchema = z.discriminatedUnion("type", [
  z.object({
    name: z.string(),
    description: z.string(),
    type: z.literal("boolean"),
    default: z.union([z.boolean(), z.null()]),
    optional: z.boolean().optional(),
  }),
  z.object({
    name: z.string(),
    description: z.string(),
    type: z.literal("worktree"),
    default: z.union([z.string(), z.null()]),
    baseDirArg: z.string(),
    optional: z.boolean().optional(),
  }),
  z.object({
    name: z.string(),
    description: z.string(),
    type: z.literal("string"),
    default: z.union([z.string(), z.null()]),
    optional: z.boolean().optional(),
    pattern: z.string().optional(),
    pattern_help: z.string().optional(),
  }),
]);

export type ScriptOpt = z.infer<typeof ScriptOptSchema>;

const ScriptSchema = z.object({
  /**
   * The name of the script.
   */
  name: z.string(),

  /**
   * The description of the script.
   */
  description: z.string().optional(),

  /**
   * The scripts that should be run after this script. It will be a list of the
   * relative names of the scripts. These are not absolute paths and will be
   * resolved in priority order.
   */
  after: z.array(z.string()).optional(),

  /**
   * The absolute path to the script.
   */
  absolutePathname: z.string(),

  /**
   * The relative path to the script.
   */
  pathname: z.string(),

  /**
   * Whether the script is embedded in the VSS package.
   */
  embedded: z.boolean().optional(),

  /**
   * The arguments that the script requires.
   */
  args: z.array(ScriptArgSchema).optional(),

  /**
   * The options that the script accepts.
   */
  opts: z.array(ScriptOptSchema).optional(),

  /**
   * Whether the script requires stdin to be inherited from the parent process.
   */
  stdin: z.literal("inherit").optional(),
});

export type Script = z.infer<typeof ScriptSchema>;

function getScriptAttribute(
  content: string,
  attribute: string,
): string | undefined {
  return content.match(new RegExp(`\@vercel\\.${attribute}\\s+(.+)`))?.[1];
}

function getScriptArgs(content: string): ScriptArg[] | undefined {
  const matches = content.matchAll(
    /@vercel\.arg\s+(?<name>[A-Za-z0-9_]+)\s+(?<description>.+)$/gm,
  );

  const args: ScriptArg[] = [];
  for (const match of matches) {
    if (!match.groups) {
      continue;
    }

    args.push({
      name: match.groups.name,
      description: match.groups.description,
    });
  }

  return args;
}

function getScriptOpts(content: string): ScriptOpt[] | undefined {
  const matches = content.matchAll(/@vercel\.opt\s+(?<json>.+)$/gm);

  const opts: ScriptOpt[] = [];
  for (const match of matches) {
    if (!match.groups) {
      continue;
    }

    opts.push(ScriptOptSchema.parse(JSON.parse(match.groups.json)));
  }

  return opts;
}

function getScriptStdin(content: string): "inherit" | undefined {
  return content.match(/@vercel\.stdin\s+inherit/)?.[0] ? "inherit" : undefined;
}

// Implement topological sort for dependency ordering
async function sortScripts(
  scripts: Script[],
  directories: string[],
): Promise<Script[]> {
  // Create a map of script paths to their indices for quick lookup
  const scriptPathToIndex = new Map<string, number>();
  scripts.forEach((script, index) => {
    scriptPathToIndex.set(script.absolutePathname, index);
  });

  // Create adjacency list representation of the dependency graph
  const graph: Map<number, number[]> = new Map();
  const inDegree: number[] = new Array(scripts.length).fill(0);

  // Initialize graph
  scripts.forEach((_, index) => {
    graph.set(index, []);
  });

  const cache = new Map<string, string>();

  // First we need to resolve all the after scripts. So let's collect all the
  // after scripts and place them into the cache.
  const afters = new Map<string, string[]>();
  for (const script of scripts) {
    if (!script.after) continue;

    for (const after of script.after) {
      afters.set(after, [
        ...(afters.get(after) || []),
        script.absolutePathname,
      ]);
    }
  }

  // Then, resolve each of the after scripts at the same time to check to see
  // which directory they're in to resolve their absolute pathnames.
  await Promise.all(
    Array.from(afters.keys()).map(async (after) => {
      for (const directory of directories) {
        const absolutePathname = path.join(directory, after);
        const exists = await fs.stat(absolutePathname).then(() => true).catch(
          () => false,
        );
        if (exists) {
          cache.set(after, absolutePathname);
          break;
        }
      }

      if (!cache.has(after)) {
        throw new Error(
          `After script ${after} not found in any known script directory in ${
            afters.get(after)!.join(
              ", ",
            )
          }`,
        );
      }
    }),
  );

  // Build the graph and calculate in-degrees
  scripts.forEach((script, currentIndex) => {
    if (script.after) {
      for (const after of script.after) {
        const dependencyPath = cache.get(after)!;
        const dependencyIndex = scriptPathToIndex.get(dependencyPath);
        if (dependencyIndex !== undefined) {
          graph.get(dependencyIndex)?.push(currentIndex);
          inDegree[currentIndex]++;
        }
      }
    }
  });

  // Perform topological sort using Kahn's algorithm
  const queue: number[] = [];
  const result: Script[] = [];

  // Add all nodes with no dependencies to the queue
  inDegree.forEach((degree, index) => {
    if (degree === 0) {
      queue.push(index);
    }
  });

  // Process the queue
  while (queue.length > 0) {
    const current = queue.shift()!;
    result.push(scripts[current]);

    // For each dependent script
    for (const dependent of graph.get(current) || []) {
      inDegree[dependent]--;
      if (inDegree[dependent] === 0) {
        queue.push(dependent);
      }
    }
  }

  // Check for circular dependencies
  if (result.length !== scripts.length) {
    throw new Error(
      `Circular dependency detected in scripts ${
        result.map((s) => s.pathname).join(", ")
      }`,
    );
  }

  return result;
}

async function getScriptsFromDirectory(
  dir: string,
  embedded: boolean,
): Promise<Script[]> {
  try {
    const scriptFiles = await fs.readdir(dir);

    const scripts = await Promise.all(
      scriptFiles
        .filter((file) => file.endsWith(".sh"))
        .map(async (script) => {
          const scriptPath = path.join(dir, script);
          const content = await fs.readFile(scriptPath, "utf-8");

          const name = getScriptAttribute(content, "name") ?? script;
          const description = getScriptAttribute(content, "description");
          const after = getScriptAttribute(content, "after");
          const args = getScriptArgs(content);
          const opts = getScriptOpts(content);
          const stdin = getScriptStdin(content);

          return {
            name,
            description,
            after: after?.split(" "),
            absolutePathname: scriptPath,
            pathname: script,
            embedded,
            args,
            opts,
            stdin,
          } satisfies Script;
        }),
    );

    return scripts.filter((s) => s !== null) as Script[];
  } catch (error) {
    console.warn(`Failed to read scripts from ${dir}:`, error);
    return [];
  }
}

export async function getScripts(): Promise<Script[]> {
  const { config } = await import("./config.ts");

  const scriptsDirURL = import.meta.resolve("./scripts");
  const embeddedDir = fileURLToPath(scriptsDirURL);

  const embeddedScripts = await getScriptsFromDirectory(embeddedDir, true);

  const externalDirs = config.global.get("scriptDirs");
  const externalScripts = await Promise.all(
    externalDirs.map((dir) => getScriptsFromDirectory(dir, false)),
  );

  const allScripts = [...embeddedScripts, ...externalScripts.flat()];

  const sortedScripts = await sortScripts(allScripts, [
    embeddedDir,
    ...externalDirs,
  ]);

  const parsedScripts: Script[] = [];
  for (const script of sortedScripts) {
    try {
      parsedScripts.push(ScriptSchema.parse(script));
    } catch (error) {
      console.error("Failed to validate script", script.pathname);
      console.error(error);
    }
  }

  return parsedScripts;
}

export async function prepareScript(
  sourcePathname: string,
  prefix: string,
  executable: boolean = false,
): Promise<string> {
  // Create a temporary directory for the script (if it doesn't exist).
  const tmpDir = path.join(os.tmpdir(), "vss", prefix);
  const exists = await fs.stat(tmpDir).then(() => true).catch(() => false);
  if (!exists) {
    await fs.mkdir(tmpDir, { recursive: true });
  }

  const targetPathname = path.join(tmpDir, path.basename(sourcePathname));

  // Copy the source file to the temporary directory.
  await fs.copyFile(sourcePathname, targetPathname);

  // Make the script executable if requested.
  if (executable) {
    await fs.chmod(targetPathname, 0o755);
  }

  return targetPathname;
}
