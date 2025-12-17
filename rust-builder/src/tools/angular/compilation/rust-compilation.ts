import type * as ng from "@angular/compiler-cli";
import type ts from "typescript";
import {
  AngularCompilation,
  DiagnosticModes,
  EmitFileResult,
} from "./angular-compilation";
import { AngularHostOptions } from "../angular-host";

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
    const ts = await AngularCompilation.loadTypescript();

    // 2. Load Configuration using standard CLI helper (for now)
    // TODO: Delegate this to Rust in future
    const config = readConfiguration(tsconfig);
    const compilerOptions = config.options;

    // 3. Initialize Rust Compiler
    // TODO: Call actual Rust binding here
    console.log("Using Rust Compiler for initialization...");

    // Mock return for now to get the builder running
    const affectedFiles = new Set<ts.SourceFile>();
    const referencedFiles: string[] = [];

    return {
      affectedFiles,
      compilerOptions,
      referencedFiles,
      externalStylesheets: hostOptions.externalStylesheets,
    };
  }

  async emitAffectedFiles(): Promise<Iterable<EmitFileResult>> {
    console.log("Using Rust Compiler for emit...");
    // TODO: Call Rust compiler to get emit results
    return [];
  }

  protected async collectDiagnostics(
    modes: DiagnosticModes
  ): Promise<Iterable<ts.Diagnostic>> {
    console.log("Using Rust Compiler for diagnostics...");
    // TODO: Call Rust compiler get diagnostics
    return [];
  }
}
