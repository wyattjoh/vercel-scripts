import fs from "node:fs";

type ConfigOptions<T> = {
  file: string;
  defaults: T;
};

export class Config<T extends Record<string, unknown>> {
  private config: T;

  constructor(private readonly options: ConfigOptions<T>) {
    try {
      this.config = JSON.parse(fs.readFileSync(this.options.file, "utf8"));
    } catch {
      this.config = this.options.defaults;
    }
  }

  get<K extends keyof T>(key: K): T[K] {
    return this.config[key] ?? this.options.defaults[key];
  }

  set<K extends keyof T>(key: K, value: T[K]) {
    this.config[key] = value;
    fs.writeFileSync(this.options.file, JSON.stringify(this.config, null, 2));
  }
}
