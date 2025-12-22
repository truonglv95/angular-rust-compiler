#[cfg(test)]
mod tests {
    
use std::path::PathBuf;

use crate::ngtsc::metadata::{OxcMetadataReader, MetadataReader};
use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

#[test]
fn test_extract_selector() {
    let source = r#"
        import { Component } from '@angular/core';

        @Component({
            selector: 'app-root',
            template: '<div></div>',
            standalone: true,
            inputs: ['foo', 'bar: baz'],
            outputs: ['click'],
            exportAs: 'myApp'
        })
        export class AppComponent {}
    "#;

    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_module(true);
    let parser = Parser::new(&allocator, source, source_type);
    let ret = parser.parse();

    assert!(ret.errors.is_empty(), "Parser errors: {:?}", ret.errors);

    let reader = OxcMetadataReader;
    let directives = reader.get_directive_metadata(&ret.program, &PathBuf::from("test.ts"));

    assert_eq!(directives.len(), 1);
    let meta = &directives[0];
    assert_eq!(meta.name, "AppComponent");
    assert_eq!(meta.selector, Some("app-root".to_string()));
    assert!(meta.is_component);
    assert!(meta.is_standalone);
    assert_eq!(meta.export_as, Some(vec!["myApp".to_string()]));
    
    // Check inputs
    let foo_input = meta.inputs.get_by_class_property_name("foo");
    assert!(foo_input.is_some());
    assert_eq!(foo_input.unwrap().binding_property_name, "foo");

    let bar_input = meta.inputs.get_by_class_property_name("bar");
    assert!(bar_input.is_some());
    assert_eq!(bar_input.unwrap().binding_property_name, "baz");

    // Check outputs
    let click_output = meta.outputs.get_by_class_property_name("click");
    assert!(click_output.is_some());
    assert_eq!(click_output.unwrap().binding_property_name, "click");
    
    // Check template
    assert_eq!(meta.template, Some("<div></div>".to_string()));
}

#[test]
fn test_extract_component_assets() {
    let source = r#"
        import { Component } from '@angular/core';

        @Component({
            selector: 'app-assets',
            templateUrl: './assets.component.html',
            styleUrls: ['./assets.component.css', './other.css'],
            styles: ['div { color: red; }']
        })
        export class AssetsComponent {}
    "#;

    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_module(true);
    let parser = Parser::new(&allocator, source, source_type);
    let ret = parser.parse();

    assert!(ret.errors.is_empty(), "Parser errors: {:?}", ret.errors);

    let reader = OxcMetadataReader;
    let directives = reader.get_directive_metadata(&ret.program, &PathBuf::from("test.ts"));

    assert_eq!(directives.len(), 1);
    let meta = &directives[0];
    
    assert_eq!(meta.name, "AssetsComponent");
    assert_eq!(meta.template_url, Some("./assets.component.html".to_string()));
    assert_eq!(meta.styles, Some(vec!["div { color: red; }".to_string()]));
    assert_eq!(meta.style_urls, Some(vec!["./assets.component.css".to_string(), "./other.css".to_string()]));
}

#[test]
fn test_extract_directive_selector() {
    let source = r#"
        import { Directive } from '@angular/core';

        @Directive({
            selector: 'app-test',
            inputs: { 'val': 'value' },
            outputs: { 'change': 'onChange' },
            standalone: false
        })
        export class TestDirective {}
    "#;

    let allocator = Allocator::default();
    let source_type = SourceType::default().with_typescript(true).with_module(true);
    let parser = Parser::new(&allocator, source, source_type);
    let ret = parser.parse();

    assert!(ret.errors.is_empty(), "Parser errors: {:?}", ret.errors);

    let reader = OxcMetadataReader;
    let directives = reader.get_directive_metadata(&ret.program, &PathBuf::from("test.ts"));

    assert_eq!(directives.len(), 1);
    let meta = &directives[0];
    assert_eq!(meta.name, "TestDirective");
    assert_eq!(meta.selector, Some("app-test".to_string()));
    assert!(!meta.is_component);
    assert!(!meta.is_standalone);

    // Check inputs object syntax
    let val_input = meta.inputs.get_by_class_property_name("val");
    assert!(val_input.is_some());
    assert_eq!(val_input.unwrap().binding_property_name, "value");

    // Check outputs object syntax
    let change_output = meta.outputs.get_by_class_property_name("change");
    assert!(change_output.is_some());
    assert_eq!(change_output.unwrap().binding_property_name, "onChange");
}
}
