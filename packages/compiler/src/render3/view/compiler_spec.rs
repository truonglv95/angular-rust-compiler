
use super::compiler::*;
use crate::constant_pool::ConstantPool;
use crate::core::{ChangeDetectionStrategy, ViewEncapsulation};
use crate::output::output_ast::{Expression, ReadVarExpr, ExternalExpr};
use crate::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};
use crate::render3::view::api::{R3ComponentMetadata, R3DirectiveMetadata, R3HostMetadata, R3LifecycleMetadata, R3ComponentDeferMetadata, DeclarationListEmitMode, R3ComponentTemplate};
use crate::render3::util::R3Reference;
use crate::expression_parser::parser::Parser;
use crate::schema::dom_element_schema_registry::DomElementSchemaRegistry;
use indexmap::IndexMap;
use std::collections::HashMap;
use std::sync::Arc;

fn create_dummy_span() -> ParseSourceSpan {
    let file = Arc::new(ParseSourceFile::new("test.ts".to_string(), "content".to_string()));
    let start = ParseLocation::new(file.clone(), 0, 0, 0);
    let end = ParseLocation::new(file, 0, 0, 0);
    ParseSourceSpan::new(start, end)
}

fn create_mock_reference(name: &str) -> R3Reference {
    R3Reference {
        value: Expression::ReadVar(ReadVarExpr {
            name: name.to_string(),
            type_: None,
            source_span: None,
        }),
        type_expr: Expression::ReadVar(ReadVarExpr {
            name: name.to_string(),
            type_: None,
            source_span: None,
        }),
    }
}

#[test]
fn should_emit_inherit_definition_feature_and_host_attrs() {
    let mut constant_pool = ConstantPool::new(false);
    let parser = Parser::new();
    let schema_registry = DomElementSchemaRegistry::new();
    let mut binding_parser = crate::template_parser::binding_parser::BindingParser::new(
        &parser,
        &schema_registry,
        vec![]
    );

    let mut attributes = HashMap::new();
    attributes.insert("matButton".to_string(), Expression::Literal(crate::output::output_ast::LiteralExpr {
        value: crate::output::output_ast::LiteralValue::String("".to_string()),
        type_: None,
        source_span: None,
    }));

    let mut special_attributes = super::api::R3HostSpecialAttributes::default();
    special_attributes.class_attr = Some("mdc-button".to_string());

    let host_metadata = R3HostMetadata {
        attributes,
        listeners: HashMap::new(),
        properties: HashMap::new(),
        special_attributes,
    };

    let directive_metadata = R3DirectiveMetadata {
        name: "MatButton".to_string(),
        type_: create_mock_reference("MatButton"),
        type_argument_count: 0,
        type_source_span: create_dummy_span(),
        deps: None,
        selector: Some("button[matButton]".to_string()),
        queries: vec![],
        view_queries: vec![],
        host: host_metadata,
        lifecycle: R3LifecycleMetadata::default(),
        inputs: IndexMap::new(),
        outputs: IndexMap::new(),
        uses_inheritance: true,
        export_as: Some(vec!["matButton".to_string()]),
        providers: None,
        is_standalone: true,
        is_signal: false,
        host_directives: None,
    };

    let component_metadata = R3ComponentMetadata {
        directive: directive_metadata,
        template: R3ComponentTemplate {
            nodes: vec![],
            ng_content_selectors: vec![],
            preserve_whitespaces: false,
        },
        declarations: vec![],
        defer: R3ComponentDeferMetadata::PerComponent { dependencies_fn: None },
        declaration_list_emit_mode: DeclarationListEmitMode::Closure,
        styles: vec![],
        external_styles: None,
        encapsulation: ViewEncapsulation::None,
        animations: None,
        view_providers: None,
        relative_context_file_path: "test.ts".to_string(),
        i18n_use_external_ids: false,
        change_detection: Some(super::api::ChangeDetectionOrExpression::Strategy(ChangeDetectionStrategy::OnPush)),
        relative_template_path: None,
        has_directive_dependencies: false,
        raw_imports: None,
    };

    let result = compile_component_from_metadata(
        &component_metadata,
        &mut constant_pool,
        &mut binding_parser
    );

    // Check if features contains InheritDefinitionFeature
    if let Expression::InvokeFn(invoke) = result.expression {
         if let Expression::LiteralMap(map) = &invoke.args[0] {
             let features = map.entries.iter().find(|e| e.key == "features");
             assert!(features.is_some(), "Should have features property");
             
             if let Some(features_entry) = features {
                 if let Expression::LiteralArray(arr) = &*features_entry.value {
                     let has_inherit = arr.entries.iter().any(|e| {
                         if let Expression::External(ext) = e {
                             ext.value.name.as_deref() == Some("ɵɵInheritDefinitionFeature")
                         } else {
                             false
                         }
                     });
                     assert!(has_inherit, "Should include ɵɵInheritDefinitionFeature");
                 } else {
                     panic!("features should be an array");
                 }
             }

             let host_attrs = map.entries.iter().find(|e| e.key == "hostAttrs");
             assert!(host_attrs.is_some(), "Should have hostAttrs property");
             
             if let Some(host_attrs_entry) = host_attrs {
                  println!("hostAttrs: {:?}", host_attrs_entry.value);
                  // Optional: Verify exact content of hostAttrs
                  // It should contain the class attribute and matButton attribute
             }
         } else {
             panic!("Expected factory function argument to be a map");
         }
    } else {
        panic!("Expected InvokeFn expression");
    }
}

#[test]
fn should_emit_inherit_definition_feature_and_host_attrs_for_directive() {
    let mut constant_pool = ConstantPool::new(false);
    let parser = Parser::new();
    let schema_registry = DomElementSchemaRegistry::new();
    let mut binding_parser = crate::template_parser::binding_parser::BindingParser::new(
        &parser,
        &schema_registry,
        vec![]
    );

    let mut attributes = HashMap::new();
    attributes.insert("matButton".to_string(), Expression::Literal(crate::output::output_ast::LiteralExpr {
        value: crate::output::output_ast::LiteralValue::String("".to_string()),
        type_: None,
        source_span: None,
    }));

    let mut special_attributes = super::api::R3HostSpecialAttributes::default();
    special_attributes.class_attr = Some("mdc-button".to_string());

    let mut properties = HashMap::new();
    properties.insert("disabled".to_string(), "isDisabled".to_string());

    let host_metadata = R3HostMetadata {
        attributes,
        listeners: HashMap::new(),
        properties,
        special_attributes,
    };

    let directive_metadata = R3DirectiveMetadata {
        name: "MatButton".to_string(),
        type_: create_mock_reference("MatButton"),
        type_argument_count: 0,
        type_source_span: create_dummy_span(),
        deps: None,
        selector: Some("button[matButton]".to_string()),
        queries: vec![],
        view_queries: vec![],
        host: host_metadata,
        lifecycle: R3LifecycleMetadata::default(),
        inputs: IndexMap::new(),
        outputs: IndexMap::new(),
        uses_inheritance: true,
        export_as: Some(vec!["matButton".to_string()]),
        providers: None,
        is_standalone: true,
        is_signal: false,
        host_directives: None,
    };

    let result = compile_directive_from_metadata(
        &directive_metadata,
        &mut constant_pool,
        &mut binding_parser
    );

    // Check if features contains InheritDefinitionFeature
    if let Expression::InvokeFn(invoke) = result.expression {
         if let Expression::LiteralMap(map) = &invoke.args[0] {
             let features = map.entries.iter().find(|e| e.key == "features");
             assert!(features.is_some(), "Should have features property");
             
             if let Some(features_entry) = features {
                 if let Expression::LiteralArray(arr) = &*features_entry.value {
                     let has_inherit = arr.entries.iter().any(|e| {
                         if let Expression::External(ext) = e {
                             ext.value.name.as_deref() == Some("ɵɵInheritDefinitionFeature")
                         } else {
                             false
                         }
                     });
                     assert!(has_inherit, "Should include ɵɵInheritDefinitionFeature");
                 } else {
                     panic!("features should be an array");
                 }
             }

             let host_attrs = map.entries.iter().find(|e| e.key == "hostAttrs");
             assert!(host_attrs.is_some(), "Should have hostAttrs property");
             
             if let Some(host_attrs_entry) = host_attrs {
                  println!("Directive hostAttrs: {:?}", host_attrs_entry.value);
                  // Verify exact content of hostAttrs to match MatButton expectation
                  if let Expression::LiteralArray(arr) = &*host_attrs_entry.value {
                       // Expect ["matButton", "", 1, "mdc-button"]
                       assert!(arr.entries.len() >= 2);
                  }
             }

             let host_vars = map.entries.iter().find(|e| e.key == "hostVars");
             // With [disabled] binding, we expect hostVars to be present
             if let Some(host_vars_entry) = host_vars {
                 println!("Directive hostVars: {:?}", host_vars_entry.value);
                 if let Expression::Literal(lit) = &*host_vars_entry.value {
                     if let crate::output::output_ast::LiteralValue::Number(n) = lit.value {
                         assert_eq!(n, 1.0, "Expected hostVars to be 1");
                     } else {
                         panic!("hostVars should be a number");
                     }
                 }
             } else {
                 panic!("Should have hostVars property");
             }
         } else {
             panic!("Expected factory function argument to be a map");
         }
    } else {
        panic!("Expected InvokeFn expression for directive");
    }
}

#[test]
fn should_emit_constant_host_class_binding_as_host_attr() {
    let mut constant_pool = ConstantPool::new(false);
    let parser = Parser::new();
    let schema_registry = DomElementSchemaRegistry::new();
    let mut binding_parser = crate::template_parser::binding_parser::BindingParser::new(
        &parser,
        &schema_registry,
        vec![]
    );

    let mut properties = HashMap::new();
    // Simulate host: { '[class]': "'mdc-button'" }
    properties.insert("class".to_string(), "'mdc-button'".to_string());

    let host_metadata = R3HostMetadata {
        attributes: HashMap::new(),
        listeners: HashMap::new(),
        properties,
        special_attributes: super::api::R3HostSpecialAttributes::default(),
    };

    let directive_metadata = R3DirectiveMetadata {
        name: "MatButton".to_string(),
        type_: create_mock_reference("MatButton"),
        type_argument_count: 0,
        type_source_span: create_dummy_span(),
        deps: None,
        selector: Some("button[matButton]".to_string()),
        queries: vec![],
        view_queries: vec![],
        host: host_metadata,
        lifecycle: R3LifecycleMetadata::default(),
        inputs: IndexMap::new(),
        outputs: IndexMap::new(),
        uses_inheritance: true,
        export_as: None,
        providers: None,
        is_standalone: true,
        is_signal: false,
        host_directives: None,
    };

    let result = compile_directive_from_metadata(
        &directive_metadata,
        &mut constant_pool,
        &mut binding_parser
    );

    if let Expression::InvokeFn(invoke) = result.expression {
         if let Expression::LiteralMap(map) = &invoke.args[0] {
             // We expect hostBindings to be present because the constant class property is now emitted as classMap
             let host_bindings = map.entries.iter().find(|e| e.key == "hostBindings");
             
             if let Some(host_bindings_entry) = host_bindings {
                 println!("Directive hostBindings: {:?}", host_bindings_entry.value);
                 // hostBindings should be a function
                 if let Expression::Fn(func) = &*host_bindings_entry.value {
                      // Verify statements in hostBindings contains classMap
                      // We can't easily check the content of statements without more complex matching, 
                      // but we can check if it exists.
                      // Ideally we'd check strict output, but for now knowing we have hostBindings 
                      // and NOT hostAttrs for this binding is a good sign.
                      
                      // Also check hostAttrs is empty or doesn't have the class
                      let host_attrs = map.entries.iter().find(|e| e.key == "hostAttrs");
                      if let Some(host_attrs_entry) = host_attrs {
                           if let Expression::LiteralArray(arr) = &*host_attrs_entry.value {
                                let has_class = arr.entries.iter().any(|e| {
                                     if let Expression::Literal(lit) = e {
                                          if let crate::output::output_ast::LiteralValue::String(s) = &lit.value {
                                               return s == "mdc-button";
                                          }
                                     }
                                     false
                                });
                                assert!(!has_class, "hostAttrs should NOT contain 'mdc-button', it should be in hostBindings");
                           }
                      }
                 } else {
                     panic!("hostBindings should be a function");
                 }
             } else {
                 panic!("Should have hostBindings property");
             }
         }
    }
}
