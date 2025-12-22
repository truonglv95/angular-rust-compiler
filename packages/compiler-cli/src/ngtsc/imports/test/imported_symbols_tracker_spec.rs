// Imported Symbols Tracker Tests

use super::super::src::imported_symbols_tracker::ImportedSymbolsTracker;

#[test]
fn test_tracker_new() {
    let tracker = ImportedSymbolsTracker::new();
    assert!(!tracker.has_named_import("/src/app.ts", "Injectable", "@angular/core"));
}

#[test]
fn test_register_named_import() {
    let mut tracker = ImportedSymbolsTracker::new();
    tracker.register_named_import("/src/app.ts", "@angular/core", "Injectable", "Injectable");
    
    assert!(tracker.has_named_import("/src/app.ts", "Injectable", "@angular/core"));
    assert!(!tracker.has_named_import("/src/app.ts", "Component", "@angular/core"));
}

#[test]
fn test_register_named_import_with_alias() {
    let mut tracker = ImportedSymbolsTracker::new();
    tracker.register_named_import("/src/app.ts", "@angular/core", "Injectable", "Inj");
    
    assert!(tracker.has_named_import("/src/app.ts", "Injectable", "@angular/core"));
    assert!(tracker.is_potential_reference_to_named_import("/src/app.ts", "Inj", "Injectable", "@angular/core"));
    assert!(!tracker.is_potential_reference_to_named_import("/src/app.ts", "Injectable", "Injectable", "@angular/core"));
}

#[test]
fn test_register_namespace_import() {
    let mut tracker = ImportedSymbolsTracker::new();
    tracker.register_namespace_import("/src/app.ts", "@angular/core", "ng");
    
    assert!(tracker.has_namespace_import("/src/app.ts", "@angular/core"));
    assert!(tracker.is_potential_reference_to_namespace_import("/src/app.ts", "ng", "@angular/core"));
    assert!(!tracker.is_potential_reference_to_namespace_import("/src/app.ts", "core", "@angular/core"));
}

#[test]
fn test_multiple_files() {
    let mut tracker = ImportedSymbolsTracker::new();
    tracker.register_named_import("/src/app.ts", "@angular/core", "Injectable", "Injectable");
    tracker.register_named_import("/src/other.ts", "@angular/core", "Component", "Component");
    
    assert!(tracker.has_named_import("/src/app.ts", "Injectable", "@angular/core"));
    assert!(!tracker.has_named_import("/src/app.ts", "Component", "@angular/core"));
    assert!(tracker.has_named_import("/src/other.ts", "Component", "@angular/core"));
}

#[test]
fn test_is_imported() {
    let mut tracker = ImportedSymbolsTracker::new();
    tracker.register_named_import("/src/app.ts", "@angular/core", "Injectable", "Injectable");
    
    assert!(tracker.is_imported("Injectable", "@angular/core"));
    assert!(!tracker.is_imported("Component", "@angular/core"));
}

#[test]
fn test_multiple_local_names() {
    let mut tracker = ImportedSymbolsTracker::new();
    // Same symbol imported with different names in different places
    tracker.register_named_import("/src/app.ts", "@angular/core", "Injectable", "Injectable");
    tracker.register_named_import("/src/app.ts", "@angular/core", "Injectable", "Inj");
    
    assert!(tracker.is_potential_reference_to_named_import("/src/app.ts", "Injectable", "Injectable", "@angular/core"));
    assert!(tracker.is_potential_reference_to_named_import("/src/app.ts", "Inj", "Injectable", "@angular/core"));
}
