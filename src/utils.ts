import path from "node:path";
import process from "node:process";

const IS_COMPILED = import.meta.url.startsWith("file:///var/folders/") ||
  import.meta.url.includes("/deno-compile-") ||
  import.meta.url.startsWith("file:///tmp/");

export function getSrcPath(): string {
  if (IS_COMPILED) {
    return path.resolve(process.cwd(), "src");
  }

  return import.meta.dirname!;
}
