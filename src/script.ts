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

const ScriptOptSchema = z.object({
  name: z.string(),
  description: z.string(),
  type: z.enum(["boolean", "worktree"]),
  default: z.union([z.boolean(), z.string(), z.null()]),
  baseDirArg: z.string().optional(),
  optional: z.boolean().optional(),
});

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
   * absolute paths to the scripts.
   */
  afterAbsolutePathnames: z.array(z.string()).optional(),

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
function sortScripts(scripts: Script[]): Script[] {
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

  // Build the graph and calculate in-degrees
  scripts.forEach((script, currentIndex) => {
    if (script.afterAbsolutePathnames) {
      for (const dependencyPath of script.afterAbsolutePathnames) {
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
    throw new Error("Circular dependency detected in scripts");
  }

  return result;
}

export async function getScripts(): Promise<Script[]> {
  const scriptsDirURL = import.meta.resolve("./scripts");
  const scriptsDir = fileURLToPath(scriptsDirURL);
  const scriptFiles = await fs.readdir(scriptsDir);

  const scripts = await Promise.all(
    scriptFiles.map(async (script) => {
      // Make the script path absolute and read the file.
      const scriptPath = path.join(scriptsDir, script);
      const content = await fs.readFile(scriptPath, "utf-8");

      // Parse the script content to extract the name, description, usage,
      // example, and script.
      const name = getScriptAttribute(content, "name");
      if (!name) {
        throw new Error(
          `Script ${script} does not have a @vercel.name attribute`,
        );
      }

      const description = getScriptAttribute(content, "description");
      const after = getScriptAttribute(content, "after");
      const args = getScriptArgs(content);
      const opts = getScriptOpts(content);
      const stdin = getScriptStdin(content);

      return {
        name,
        description,
        afterAbsolutePathnames: after
          ?.split(" ")
          .map((a) => path.join(scriptsDir, a.trim())),
        absolutePathname: scriptPath,
        pathname: script,
        embedded: true,
        args,
        opts,
        stdin,
      } satisfies Script;
    }),
  );

  const sortedScripts = sortScripts(scripts);

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
