/**
 * Angular Linker Plugins - Index
 *
 * Re-exports all linker plugins for different bundlers.
 */

export { angularLinkerEsbuildPlugin } from "./esbuild";
export { angularLinkerRolldownPlugin } from "./rolldown";
export {
  angularLinkerVitePlugin,
  getAngularViteConfig,
  ANGULAR_PACKAGES,
  NON_ANGULAR_PACKAGES,
} from "./vite";
export type { ViteLinkerPluginOptions } from "./vite";
export type { LinkerOptions, LinkerResult } from "./types";
export {
  needsLinking,
  isAngularPackage,
  isJsFile,
  cleanModuleId,
} from "./types";
