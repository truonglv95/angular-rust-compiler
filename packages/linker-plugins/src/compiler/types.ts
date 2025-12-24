/**
 * Compiler Types
 */

export interface CompilerBinding {
  compile(filePath: string, code: string): string;
  linkFile(filePath: string, code: string): string;
}
