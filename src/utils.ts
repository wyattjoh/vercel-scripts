import path from "node:path";
import process from "node:process";

const IS_COMPILED = import.meta.url.startsWith("file:///var/folders/") ||
  import.meta.url.includes("/deno-compile-") ||
  import.meta.url.startsWith("file:///tmp/");

export function getSrcPath() {
  if (IS_COMPILED) {
    return path.resolve(process.cwd(), "src");
  }

  return path.dirname(new URL(import.meta.url).pathname);
}
