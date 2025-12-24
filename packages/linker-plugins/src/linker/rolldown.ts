/**
 * Angular Linker Plugin for Rolldown
 *
 * Use this plugin with rolldown-vite or standalone Rolldown.
 *
 * @example
 * ```js
 * import { angularLinkerRolldownPlugin } from 'vite-plugin-angular-rust/rolldown';
 * import { defineConfig } from 'vite';
 *
 * export default defineConfig({
 *   plugins: [angularLinkerRolldownPlugin()],
 *   optimizeDeps: {
 *     exclude: [
 *       '@angular/core',
 *       '@angular/common',
 *       '@angular/platform-browser',
 *       '@angular/router',
 *     ],
 *   },
 * });
 * ```
 */

import { createRequire } from "module";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import type { CompilerBinding, LinkerOptions, LinkerResult } from "./types";
import {
  needsLinking,
  isAngularPackage,
  isJsFile,
  cleanModuleId,
} from "./types";

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

export interface RolldownPlugin {
  name: string;
  transform(
    code: string,
    id: string
  ): Promise<LinkerResult | null> | LinkerResult | null;
}

/**
 * Creates a Rolldown-compatible plugin for Angular linker
 */
export function angularLinkerRolldownPlugin(
  options?: LinkerOptions
): RolldownPlugin {
  const debug = options?.debug ?? false;
  let compiler: CompilerBinding;

  return {
    name: "angular-linker-rolldown",
    async transform(code: string, id: string): Promise<LinkerResult | null> {
      // Lazy initialize compiler
      if (!compiler) {
        compiler = getCompiler(options);
      }

      // Only process @angular packages with .mjs or .js extensions
      const isInNodeModules = id.includes("node_modules");
      const cleanId = cleanModuleId(id);

      if (!isAngularPackage(id) || !isInNodeModules || !isJsFile(id)) {
        return null;
      }

      // Check if file contains partial declarations
      if (!needsLinking(code)) {
        return null;
      }

      if (debug) {
        console.log(`[Angular Linker] Linking: ${cleanId}`);
      }

      try {
        const result = compiler.linkFile(cleanId, code);

        if (result.startsWith("/* Linker Error")) {
          console.error(`[Angular Linker Error] ${id}:\n${result}`);
          return null;
        }

        if (debug) {
          console.log(`[Angular Linker] Successfully linked: ${cleanId}`);
        }

        return { code: result, map: null };
      } catch (e) {
        console.error(`[Angular Linker Failed] ${id}:`, e);
        return null;
      }
    },
  };
}

export default angularLinkerRolldownPlugin;
