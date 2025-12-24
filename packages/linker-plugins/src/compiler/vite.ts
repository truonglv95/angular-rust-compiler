/**
 * Angular Compiler Plugin for Vite
 *
 * This plugin compiles Angular TypeScript files using the Rust-based Angular compiler.
 * Use with the linker plugin for a complete Angular build solution.
 *
 * @example
 * ```js
 * import { angularCompilerVitePlugin } from 'angular-rust-plugins/compiler/vite';
 * import { angularLinkerVitePlugin } from 'angular-rust-plugins/linker/vite';
 * import { defineConfig } from 'vite';
 *
 * export default defineConfig({
 *   plugins: [
 *     angularLinkerVitePlugin(),
 *     angularCompilerVitePlugin(),
 *   ],
 * });
 * ```
 */

import type { Plugin, HmrContext } from "vite";
import { createRequire } from "module";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import type { CompilerBinding } from "./types";

let compilerInstance: CompilerBinding | null = null;

export interface CompilerOptions {
  /**
   * Enable debug logging
   * @default false
   */
  debug?: boolean;

  /**
   * Custom path to the Angular Rust binding package
   */
  bindingPath?: string;
}

function getCompiler(options?: CompilerOptions): CompilerBinding {
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
      // Use import.meta.url to get the actual location of this file
      const currentFileUrl = import.meta.url;
      const currentFilePath = fileURLToPath(currentFileUrl);
      const currentDir = dirname(currentFilePath);
      const require = createRequire(currentFileUrl);

      // Try multiple possible binding locations
      const possiblePaths = [
        join(currentDir, "..", "binding"), // dist/compiler/../binding
        join(currentDir, "..", "..", "binding"), // in case of deeper nesting
        join(currentDir, "binding"), // same directory
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
 * Creates a Vite plugin for Angular Rust compiler
 * Compiles .ts files (except .d.ts) using the Rust compiler
 */
export function angularCompilerVitePlugin(options?: CompilerOptions): Plugin {
  const debug = options?.debug ?? false;
  let compiler: CompilerBinding;

  return {
    name: "angular-rust-compiler",
    enforce: "pre",

    transform(code: string, id: string) {
      // Lazy initialize compiler
      if (!compiler) {
        compiler = getCompiler(options);
      }

      // Skip node_modules - those are handled by linker, not compiler
      if (id.includes("node_modules")) {
        return null;
      }

      // Only process TypeScript files, skip declaration files
      if (!id.endsWith(".ts") || id.endsWith(".d.ts")) {
        return null;
      }

      if (debug) {
        console.log(`[Angular Compiler] Compiling: ${id}`);
      }

      try {
        const result = compiler.compile(id, code);

        if (result.startsWith("/* Error")) {
          console.error(`[Angular Compiler Error] ${id}:\n${result}`);
          throw new Error(`Rust Compilation Failed for ${id}`);
        }

        if (debug) {
          console.log(`[Angular Compiler] Successfully compiled: ${id}`);
        }

        return { code: result, map: null };
      } catch (e) {
        console.error(`[Angular Compiler Failed] ${id}:`, e);
        throw e;
      }
    },

    handleHotUpdate({ file, server }: HmrContext) {
      // When HTML template changes, invalidate the corresponding TS file
      if (file.endsWith(".html")) {
        const tsFile = file.replace(/\.html$/, ".ts");

        if (debug) {
          console.log(`[HMR] HTML changed: ${file}`);
          console.log(`[HMR] Invalidating TS: ${tsFile}`);
        }

        const mod = server.moduleGraph.getModuleById(tsFile);
        if (mod) {
          server.moduleGraph.invalidateModule(mod);
          server.ws.send({ type: "full-reload", path: "*" });
          return [];
        } else {
          server.ws.send({ type: "full-reload", path: "*" });
          return [];
        }
      }
    },
  };
}

export default angularCompilerVitePlugin;
