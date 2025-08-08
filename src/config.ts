import fs from "node:fs";

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
