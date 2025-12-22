// Core Import Rewriter Tests

use super::super::src::core::*;

#[test]
fn test_noop_import_rewriter_symbol() {
    let rewriter = NoopImportRewriter::new();
    assert_eq!(rewriter.rewrite_symbol("Injectable", "@angular/core"), "Injectable");
}

#[test]
fn test_noop_import_rewriter_specifier() {
    let rewriter = NoopImportRewriter::new();
    assert_eq!(rewriter.rewrite_specifier("@angular/core", "/src/app.ts"), "@angular/core");
}

#[test]
fn test_noop_import_rewriter_namespace() {
    let rewriter = NoopImportRewriter::new();
    assert_eq!(rewriter.rewrite_namespace_import_identifier("ng", "@angular/core"), "ng");
}

#[test]
fn test_r3_symbols_rewriter_non_core_symbol() {
    let rewriter = R3SymbolsImportRewriter::new("/node_modules/@angular/core/r3_symbols.ts");
    assert_eq!(rewriter.rewrite_symbol("SomeSymbol", "some-package"), "SomeSymbol");
}

#[test]
fn test_r3_symbols_rewriter_core_symbol() {
    let rewriter = R3SymbolsImportRewriter::new("/node_modules/@angular/core/r3_symbols.ts");
    assert_eq!(rewriter.rewrite_symbol("ɵɵdefineInjectable", "@angular/core"), "ɵɵdefineInjectable");
}

#[test]
fn test_r3_symbols_rewriter_rewrite_core_symbol() {
    let rewriter = R3SymbolsImportRewriter::new("/node_modules/@angular/core/r3_symbols.ts");
    // ɵsetClassMetadata gets rewritten to setClassMetadata
    assert_eq!(rewriter.rewrite_symbol("ɵsetClassMetadata", "@angular/core"), "setClassMetadata");
}

#[test]
fn test_validate_and_rewrite_core_symbol_valid() {
    assert_eq!(validate_and_rewrite_core_symbol("ɵɵdefineInjectable"), "ɵɵdefineInjectable");
    assert_eq!(validate_and_rewrite_core_symbol("ɵsetClassMetadata"), "setClassMetadata");
    assert_eq!(validate_and_rewrite_core_symbol("ɵNgModuleFactory"), "NgModuleFactory");
}

#[test]
fn test_validate_and_rewrite_core_symbol_unknown() {
    // Unknown symbols should be returned as-is
    assert_eq!(validate_and_rewrite_core_symbol("UnknownSymbol"), "UnknownSymbol");
}
