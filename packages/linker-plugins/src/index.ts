/**
 * Angular Rust Plugins
 *
 * High-performance Angular linker and compiler plugins powered by Rust.
 * Supports Vite, esbuild, Rolldown, and more bundlers.
 *
 * @packageDocumentation
 *
 * @example
 * ```js
 * // Use linker + compiler together
 * import { angularLinkerVitePlugin } from 'angular-rust-plugins/linker/vite';
 * import { angularCompilerVitePlugin } from 'angular-rust-plugins/compiler/vite';
 *
 * export default defineConfig({
 *   plugins: [
 *     angularLinkerVitePlugin(),
 *     angularCompilerVitePlugin(),
 *   ],
 * });
 * ```
 */

// Re-export linker plugins
export {
  angularLinkerEsbuildPlugin,
  angularLinkerRolldownPlugin,
  angularLinkerVitePlugin,
  getAngularViteConfig,
  ANGULAR_PACKAGES,
  NON_ANGULAR_PACKAGES,
  needsLinking,
  isAngularPackage,
  isJsFile,
  cleanModuleId,
} from "./linker";

export type {
  LinkerOptions,
  LinkerResult,
  ViteLinkerPluginOptions,
} from "./linker";

// Re-export compiler plugins
export { angularCompilerVitePlugin } from "./compiler";

export type { CompilerOptions, CompilerBinding } from "./compiler";
