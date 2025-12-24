/**
 * Angular Linker Plugin for Vite
 *
 * This plugin handles Angular linking for both Vite's dev server (with rolldown/esbuild)
 * and production builds.
 *
 * @example
 * ```js
 * import { angularLinkerVitePlugin } from 'vite-plugin-angular-rust/vite';
 * import { defineConfig } from 'vite';
 *
 * export default defineConfig({
 *   plugins: [angularLinkerVitePlugin()],
 *   optimizeDeps: {
 *     exclude: [
 *       '@angular/core',
 *       '@angular/common',
 *       '@angular/platform-browser',
 *       '@angular/router',
 *       '@angular/forms',
 *     ],
 *     include: ['zone.js', 'rxjs', 'rxjs/operators'],
 *   },
 * });
 * ```
 */

import type { Plugin } from "vite";
import { createRequire } from "module";
import { dirname, join } from "path";
import { fileURLToPath } from "url";
import type { CompilerBinding, LinkerOptions } from "./types";
import {
  needsLinking,
  isAngularPackage,
  isJsFile,
  cleanModuleId,
  ANGULAR_PACKAGES,
  NON_ANGULAR_PACKAGES,
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

export interface ViteLinkerPluginOptions extends LinkerOptions {
  /**
   * Additional packages to exclude from pre-bundling
   */
  excludePackages?: string[];

  /**
   * Additional packages to include in pre-bundling (non-Angular packages)
   */
  includePackages?: string[];
}

/**
 * Creates a Vite plugin for Angular linker
 * Works with both rolldown-vite and standard Vite (esbuild)
 */
export function angularLinkerVitePlugin(
  options?: ViteLinkerPluginOptions
): Plugin {
  const debug = options?.debug ?? false;
  let compiler: CompilerBinding;

  return {
    name: "angular-linker-vite",
    enforce: "pre",

    config(config) {
      // Merge optimizeDeps configuration
      const excludePackages = [
        ...ANGULAR_PACKAGES,
        ...(options?.excludePackages ?? []),
      ];
      const includePackages = [
        ...NON_ANGULAR_PACKAGES,
        ...(options?.includePackages ?? []),
      ];

      return {
        optimizeDeps: {
          exclude: excludePackages,
          include: includePackages,
        },
      };
    },

    transform(code: string, id: string) {
      // Lazy initialize compiler
      if (!compiler) {
        compiler = getCompiler(options);
      }

      // Only process node_modules
      if (!id.includes("node_modules")) {
        return null;
      }

      // Only process Angular packages
      if (!isAngularPackage(id)) {
        return null;
      }

      // Only process JS files
      if (!isJsFile(id)) {
        return null;
      }

      // Check if file contains partial declarations
      if (!needsLinking(code)) {
        return null;
      }

      const cleanId = cleanModuleId(id);

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

/**
 * Get recommended Vite config for Angular with Rust linker
 */
export function getAngularViteConfig() {
  return {
    plugins: [angularLinkerVitePlugin()],
    optimizeDeps: {
      exclude: ANGULAR_PACKAGES,
      include: NON_ANGULAR_PACKAGES,
    },
  };
}

export { ANGULAR_PACKAGES, NON_ANGULAR_PACKAGES };
export default angularLinkerVitePlugin;
