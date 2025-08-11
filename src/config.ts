import fs from "node:fs";
import path from "node:path";
import os from "node:os";
import process from "node:process";

function loadConfigFile<T>(file: string, defaults: T) {
  try {
    return JSON.parse(fs.readFileSync(file, "utf8"));
  } catch {
    return defaults;
  }
}

export function createConfig<T>(file: string, defaults: T) {
  let cache: T;
  return {
    get<K extends keyof T>(key: K): T[K] {
      if (!cache) cache = loadConfigFile(file, defaults);
      return cache[key] ?? defaults[key];
    },
    set<K extends keyof T>(key: K, value: T[K]) {
      if (!cache) cache = loadConfigFile(file, defaults);
      cache[key] = value;
      fs.writeFileSync(file, JSON.stringify(cache, null, 2));
    },
  };
}

export const config = {
  global: createConfig<{
    args: Record<string, unknown>;
    scriptDirs: string[];
  }>(
    path.join(
      os.homedir(),
      ".vss.json",
    ),
    {
      args: {},
      scriptDirs: [],
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
