import { defineConfig } from "tsup";

export default defineConfig({
  entry: ["src/main.ts"],
  clean: true,
  outDir: "dist",
  format: "esm",
  platform: "node",
  dts: true,
  bundle: true,
});
