import { defineConfig } from "tsup";

export default defineConfig({
  entry: {
    // Main index
    index: "src/index.ts",
    // Linker plugins
    "linker/index": "src/linker/index.ts",
    "linker/esbuild": "src/linker/esbuild.ts",
    "linker/rolldown": "src/linker/rolldown.ts",
    "linker/vite": "src/linker/vite.ts",
    // Compiler plugins
    "compiler/index": "src/compiler/index.ts",
    "compiler/vite": "src/compiler/vite.ts",
  },
  format: ["cjs", "esm"],
  dts: true,
  splitting: false,
  sourcemap: true,
  clean: true,
  // Mark bundler deps as external
  external: ["vite", "esbuild"],
  skipNodeModulesBundle: true,
});
