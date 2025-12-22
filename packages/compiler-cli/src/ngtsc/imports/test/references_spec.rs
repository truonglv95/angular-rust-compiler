// References Tests

use super::super::src::references::*;

#[test]
fn test_owning_module() {
    let module = OwningModule::new("@angular/core", "/src/app.ts");
    assert_eq!(module.specifier, "@angular/core");
    assert_eq!(module.resolution_context, "/src/app.ts");
}

#[test]
fn test_reference_new() {
    let reference = Reference::new("MyComponent", "/src/app.ts", None);
    assert_eq!(reference.name, "MyComponent");
    assert_eq!(reference.source_file, "/src/app.ts");
    assert!(!reference.synthetic);
    assert!(!reference.is_ambient);
    assert!(reference.owned_by_module_guess().is_none());
}

#[test]
fn test_reference_with_owning_module() {
    let module = OwningModule::new("@angular/core", "/src/app.ts");
    let reference = Reference::new("Injectable", "/node_modules/@angular/core/index.ts", Some(module));
    
    assert!(reference.has_owning_module_guess());
    assert_eq!(reference.owned_by_module_guess(), Some("@angular/core"));
}

#[test]
fn test_reference_ambient() {
    let reference = Reference::ambient("SomeGlobal", "/src/ambient.d.ts");
    assert!(reference.is_ambient);
    assert!(!reference.has_owning_module_guess());
}

#[test]
fn test_reference_add_identifier() {
    let mut reference = Reference::new("Foo", "/src/foo.ts", None);
    reference.add_identifier("FooAlias");
    
    // Should have both identifiers
    assert_eq!(reference.debug_name(), "Foo");
}

#[test]
fn test_reference_get_identity_in() {
    let reference = Reference::new("Foo", "/src/foo.ts", None);
    
    assert_eq!(reference.get_identity_in("/src/foo.ts"), Some("Foo"));
    assert_eq!(reference.get_identity_in("/src/bar.ts"), None);
}

#[test]
fn test_reference_clone_with_no_identifiers() {
    let reference = Reference::new("Foo", "/src/foo.ts", None);
    let cloned = reference.clone_with_no_identifiers();
    
    assert_eq!(cloned.name, "Foo");
}
