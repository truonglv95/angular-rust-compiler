// References Tests

use super::super::src::references::*;
use std::path::PathBuf;

#[test]
fn test_owning_module() {
    let module = OwningModule::new("@angular/core", "/src/app.ts");
    assert_eq!(module.specifier, "@angular/core");
    assert_eq!(module.resolution_context, "/src/app.ts");
}

#[test]
fn test_reference_from_name() {
    let reference = Reference::from_name("MyComponent", Some(PathBuf::from("/src/app.ts")));
    assert_eq!(reference.debug_name(), "MyComponent");
    assert_eq!(reference.source_file, Some(PathBuf::from("/src/app.ts")));
    assert!(!reference.synthetic);
    assert!(!reference.is_ambient);
    assert!(reference.owned_by_module_guess().is_none());
}

#[test]
fn test_reference_with_owning_module() {
    let module = OwningModule::new("@angular/core", "/src/app.ts");
    let mut reference = Reference::from_name(
        "Injectable", 
        Some(PathBuf::from("/node_modules/@angular/core/index.ts"))
    );
    reference.best_guess_owning_module = Some(module);
    
    assert!(reference.has_owning_module_guess());
    assert_eq!(reference.owned_by_module_guess(), Some("@angular/core"));
}

#[test]
fn test_reference_add_identifier() {
    let mut reference = Reference::from_name("Foo", Some(PathBuf::from("/src/foo.ts")));
    reference.add_identifier("FooAlias");
    
    // Should have both identifiers
    assert_eq!(reference.debug_name(), "Foo");
}

#[test]
fn test_reference_clone_with_no_identifiers() {
    let reference = Reference::from_name("Foo", Some(PathBuf::from("/src/foo.ts")));
    let cloned = reference.clone_with_no_identifiers();
    
    assert_eq!(cloned.debug_name(), "Foo");
}
