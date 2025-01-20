import fs from "node:fs/promises";
import path from "node:path";

export type ScriptArg = {
  name: string;
  description: string;
};

export type Script = {
  name: string;
  description: string | undefined;
  /**
   * The scripts that should be run after this script. It will be a list of the
   * absolute paths to the scripts.
   */
  afterAbsolutePathnames: string[] | undefined;

  /**
   * The absolute path to the script.
   */
  absolutePathname: string;

  /**
   * The relative path to the script.
   */
  pathname: string;

  /**
   * The arguments that the script requires.
   */
  args: ScriptArg[] | undefined;
};

function getScriptAttribute(
  content: string,
  attribute: string
): string | undefined {
  return content.match(new RegExp(`\@vercel\\.${attribute}\\s+(.+)`))?.[1];
}

function getScriptArgs(content: string): ScriptArg[] | undefined {
  const matches = content.matchAll(
    /@vercel\.arg\s+(?<name>[A-Za-z0-9_]+)\s+(?<description>.+)$/gm
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
    // biome-ignore lint/style/noNonNullAssertion: checked the length above
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
  const scriptsDir = path.resolve(import.meta.dirname, "..", "scripts");
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
          `Script ${script} does not have a @vercel.name attribute`
        );
      }

      const description = getScriptAttribute(content, "description");
      const after = getScriptAttribute(content, "after");
      const args = getScriptArgs(content);

      return {
        name,
        description,
        afterAbsolutePathnames: after
          ?.split(" ")
          .map((a) => path.join(scriptsDir, a.trim())),
        absolutePathname: scriptPath,
        pathname: script,
        args,
      } satisfies Script;
    })
  );

  const sortedScripts = sortScripts(scripts);

  return sortedScripts;
}
