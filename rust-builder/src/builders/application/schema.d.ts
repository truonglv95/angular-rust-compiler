export interface Schema {
  browser: string;
  server?: string;
  outputPath:
    | string
    | { browser: string; server: string; media?: string; base?: string };
  index: string | { input: string; output?: string };
  tsConfig: string;
  inlineStyleLanguage?: string;
  assets?: (string | { glob: string; input: string; output: string })[];
  styles?: (
    | string
    | { input: string; inject?: boolean; bundleName?: string }
  )[];
  scripts?: (
    | string
    | { input: string; inject?: boolean; bundleName?: string }
  )[];
  polyfills?: string[];
  watch?: boolean;
  poll?: number;
  preserveSymlinks?: boolean;
  optimization?:
    | boolean
    | { scripts?: boolean; styles?: boolean; fonts?: boolean };
  sourceMap?:
    | boolean
    | {
        scripts?: boolean;
        styles?: boolean;
        hidden?: boolean;
        vendor?: boolean;
      };
  externalDependencies?: string[];
  allowedCommonJsDependencies?: string[];
  baseHref?: string;
  deployUrl?: string;
  verbose?: boolean;
  progress?: boolean;
  i18nMissingTranslation?: "warning" | "error" | "ignore";
  localize?: boolean | string[];
  aot?: boolean;
  jit?: boolean;
  serverEntryPoint?: string;
  prerender?:
    | boolean
    | { routes?: string[]; discoverRoutes?: boolean; routesFile?: string };
  appShell?: boolean;
  ssr?: boolean | { entry?: string };
  outputHashing?: "none" | "all" | "media" | "bundles";
  deleteOutputPath?: boolean;
  namedChunks?: boolean;
  subresourceIntegrity?: boolean;
  serviceWorker?: string;
  statsJson?: boolean;
  webWorkerTsConfig?: string;
  crossOrigin?: "none" | "anonymous" | "use-credentials";
  [key: string]: any;
}

export type OptimizationUnion = boolean | OptimizationClass;

export interface OptimizationClass {
  scripts?: boolean;
  styles?: boolean | StylesClass;
  fonts?: boolean | FontsClass;
}

export interface StylesClass {
  minify?: boolean;
  inlineCritical?: boolean;
  external?: boolean;
  removeSpecialComments?: boolean;
}

export interface FontsClass {
  inline?: boolean;
}

export type SourceMapUnion = boolean | SourceMapClass;

export interface SourceMapClass {
  scripts?: boolean;
  styles?: boolean;
  hidden?: boolean;
  vendor?: boolean;
  sourcesContent?: boolean;
}

export type AssetPattern = string | AssetPatternClass;

export interface AssetPatternClass {
  glob: string;
  input: string;
  output: string;
  ignore?: string[];
}

export enum OutputMode {
  Static = "static",
  Server = "server",
}

export interface Budget {
  type: Type;
  maximumWarning?: string;
  maximumError?: string;
  minimumWarning?: string;
  minimumError?: string;
  name?: string;
  baseline?: string;
}

export enum Type {
  All = "all",
  AllScript = "allScript",
  Any = "any",
  AnyComponentStyle = "anyComponentStyle",
  AnyScript = "anyScript",
  Bundle = "bundle",
  Initial = "initial",
}

export enum ExperimentalPlatform {
  Neutral = "neutral",
  Node = "node",
  Strict = "strict",
}

export type I18NTranslation = "warning" | "error" | "ignore";
export enum OutputHashing {
  None = "none",
  All = "all",
  Media = "media",
  Bundles = "bundles",
}
export type OutputPathClass = {
  browser: string;
  server: string;
  media?: string;
  base?: string;
};

export interface Budget {
  type: Type;
  maximumWarning?: string;
  maximumError?: string;
  minimumWarning?: string;
  minimumError?: string;
  warning?: string; // Add alias/legacy
  error?: string; // Add alias/legacy
  name?: string;
  baseline?: string;
}
