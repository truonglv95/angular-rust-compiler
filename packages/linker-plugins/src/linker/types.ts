/**
 * Angular Linker Plugin Types
 */

export interface LinkerOptions {
  /**
   * Enable debug logging
   * @default false
   */
  debug?: boolean;

  /**
   * Custom path to the Angular Rust binding package
   * If not specified, will try to resolve @anthropic/angular-rust-binding
   */
  bindingPath?: string;
}

export interface LinkerResult {
  code: string;
  map: null | undefined;
}

export interface CompilerBinding {
  linkFile(filePath: string, code: string): string;
  compile?(filePath: string, code: string): string;
}

/**
 * Check if file contains Angular partial declarations that need linking
 */
export function needsLinking(code: string): boolean {
  return code.includes("ɵɵngDeclare");
}

/**
 * Check if file is an Angular package file
 */
export function isAngularPackage(id: string): boolean {
  return id.includes("@angular") || id.includes("/@angular/");
}

/**
 * Check if file is a JavaScript/MJS file
 */
export function isJsFile(id: string): boolean {
  const cleanId = id.split("?")[0];
  return cleanId.endsWith(".mjs") || cleanId.endsWith(".js");
}

/**
 * Clean module ID by removing query strings
 */
export function cleanModuleId(id: string): string {
  return id.split("?")[0];
}

/**
 * Default Angular packages to exclude from pre-bundling
 */
export const ANGULAR_PACKAGES = [
  "@angular/core",
  "@angular/common",
  "@angular/platform-browser",
  "@angular/platform-browser-dynamic",
  "@angular/router",
  "@angular/forms",
  "@angular/animations",
  "@angular/cdk",
  "@angular/material",
];

/**
 * Packages that don't need linking and should be included in pre-bundling
 */
export const NON_ANGULAR_PACKAGES = ["zone.js", "rxjs", "rxjs/operators"];
