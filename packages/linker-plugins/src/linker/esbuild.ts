/**
 * Angular Linker Plugin for esbuild
 *
 * Use this plugin with Vite's optimizeDeps.esbuildOptions or standalone esbuild.
 *
 * @example
 * ```js
 * import { angularLinkerEsbuildPlugin } from 'vite-plugin-angular-rust/esbuild';
 *
 * // With esbuild
 * import esbuild from 'esbuild';
 *
 * esbuild.build({
 *   plugins: [angularLinkerEsbuildPlugin()],
 *   // ...
 * });
 *
 * // With Vite
 * import { defineConfig } from 'vite';
 *
 * export default defineConfig({
 *   optimizeDeps: {
 *     esbuildOptions: {
 *       plugins: [angularLinkerEsbuildPlugin()],
 *     },
 *   },
 * });
 * ```
 */

import type { Plugin } from "esbuild";
import { promises as fs } from "fs";
import { createRequire } from "module";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import type { CompilerBinding, LinkerOptions } from "./types";
import { needsLinking } from "./types";

let compilerInstance: CompilerBinding | null = null;

function getCompiler(options?: LinkerOptions): CompilerBinding {
  if (compilerInstance) {
    return compilerInstance;
  }

  try {
    let binding: { Compiler: new () => CompilerBinding };

    if (options?.bindingPath) {
      const require = createRequire(import.meta.url);
      binding = require(options.bindingPath);
    } else {
      // Load from bundled binding directory
      const currentFileUrl = import.meta.url;
      const currentFilePath = fileURLToPath(currentFileUrl);
      const currentDir = dirname(currentFilePath);
      const require = createRequire(currentFileUrl);

      // Try multiple possible binding locations
      const possiblePaths = [
        join(currentDir, "..", "binding"), // dist/linker/../binding
        join(currentDir, "binding"), // same directory
        join(currentDir, "..", "..", "binding"), // deeper nesting
      ];

      let loadedBinding: { Compiler: new () => CompilerBinding } | null = null;
      let lastError: unknown = null;

      for (const bindingPath of possiblePaths) {
        try {
          loadedBinding = require(bindingPath);
          break;
        } catch (e) {
          lastError = e;
        }
      }

      if (!loadedBinding) {
        throw (
          lastError ||
          new Error("Could not find binding in any expected location")
        );
      }

      binding = loadedBinding;
    }

    compilerInstance = new binding.Compiler();
    return compilerInstance;
  } catch (e) {
    throw new Error(`Failed to load Angular Rust binding. Error: ${e}`);
  }
}

/**
 * Creates an esbuild plugin for Angular linker
 */
export function angularLinkerEsbuildPlugin(options?: LinkerOptions): Plugin {
  return {
    name: "angular-linker-esbuild",
    setup(build) {
      const compiler = getCompiler(options);
      const debug = options?.debug ?? false;

      // Handle all .mjs and .js files in @angular packages
      build.onLoad({ filter: /@angular\/.*\.(mjs|js)$/ }, async (args) => {
        if (debug) {
          console.log(`[Angular Linker] Processing: ${args.path}`);
        }

        const code = await fs.readFile(args.path, "utf8");

        // Check if file contains partial declarations
        if (!needsLinking(code)) {
          return { contents: code, loader: "js" };
        }

        try {
          const result = compiler.linkFile(args.path, code);

          if (result.startsWith("/* Linker Error")) {
            if (debug) {
              console.error(`[Angular Linker Error] ${args.path}:\n${result}`);
            }
            return { contents: code, loader: "js" };
          }

          if (debug) {
            console.log(`[Angular Linker] Successfully linked: ${args.path}`);
          }

          return { contents: result, loader: "js" };
        } catch (e) {
          if (debug) {
            console.error(`[Angular Linker Failed] ${args.path}:`, e);
          }
          return { contents: code, loader: "js" };
        }
      });
    },
  };
}

export default angularLinkerEsbuildPlugin;
