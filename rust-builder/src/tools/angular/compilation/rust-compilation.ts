import type * as ng from "@angular/compiler-cli";
import type ts from "typescript";
import {
  AngularCompilation,
  DiagnosticModes,
  EmitFileResult,
} from "./angular-compilation";
import { AngularHostOptions } from "../angular-host";
import { spawn } from "node:child_process";
import { resolve } from "node:path";

export class RustAngularCompilation extends AngularCompilation {
  constructor() {
    super();
  }

  async initialize(
    tsconfig: string,
    hostOptions: AngularHostOptions,
    compilerOptionsTransformer?: (
      compilerOptions: ng.CompilerOptions
    ) => ng.CompilerOptions
  ): Promise<{
    affectedFiles: ReadonlySet<ts.SourceFile>;
    compilerOptions: ng.CompilerOptions;
    referencedFiles: readonly string[];
    externalStylesheets?: ReadonlyMap<string, string>;
    templateUpdates?: ReadonlyMap<string, string>;
    componentResourcesDependencies?: ReadonlyMap<string, readonly string[]>;
  }> {
    // 1. Load Compiler CLI (we still need some types and basic utils from it)
    const { readConfiguration } = await AngularCompilation.loadCompilerCli();

    // 2. Load Configuration
    const config = readConfiguration(tsconfig);
    let compilerOptions = config.options;
    if (compilerOptionsTransformer) {
      compilerOptions = compilerOptionsTransformer(compilerOptions);
    }

    // 3. Initialize Rust Compiler
    console.log("Using Rust Compiler (ngc) for initialization...");
    // Hardcoded path for development. Ideally would be configured or found in PATH/bin
    const ngcPath = resolve(process.cwd(), "target/debug/ngc");

    return new Promise((resolvePromise, reject) => {
      console.log(`Spawning ${ngcPath} -p ${tsconfig}`);
      const child = spawn(ngcPath, ["-p", tsconfig], { stdio: "inherit" });

      child.on("close", (code) => {
        if (code === 0) {
          console.log("Rust compilation success");
          // Mock return to get the builder running, assuming ngc wrote files to disk
          // In a real implementation, we would return the affected source files and parsed options
          const affectedFiles = new Set<ts.SourceFile>();
          const referencedFiles: string[] = [];

          resolvePromise({
            affectedFiles,
            compilerOptions,
            referencedFiles,
            externalStylesheets: hostOptions.externalStylesheets,
          });
        } else {
          reject(new Error(`Rust compilation failed with code ${code}`));
        }
      });
      child.on("error", (err) => {
        reject(new Error(`Failed to start ngc: ${err.message}`));
      });
    });
  }

  async emitAffectedFiles(): Promise<Iterable<EmitFileResult>> {
    console.log(
      "Using Rust Compiler for emit (already handled in initialize for now)..."
    );
    // Since ngc produced output in initialize (whole compile), we might not need to do anything here
    // unless we want to return the content to esbuild.
    // For now, we return empty, assuming side-effects (file writing) are sufficient for verification
    // or that we need to implement reading back the files if esbuild needs them.
    return [];
  }

  protected async collectDiagnostics(
    modes: DiagnosticModes
  ): Promise<Iterable<ts.Diagnostic>> {
    console.log(
      "Using Rust Compiler for diagnostics (logs printed to stderr)..."
    );
    // ngc printed diagnostics to stderr. Return empty here to satisfy interface.
    // In future, parse ngc JSON output.
    return [];
  }
}
