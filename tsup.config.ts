import { defineConfig } from "tsup";

export default defineConfig({
  entry: ["src/main.ts", "src/generate-readme.ts"],
  clean: true,
  outDir: "dist",
  format: "esm",
  platform: "node",
  dts: true,
  bundle: true,
});
