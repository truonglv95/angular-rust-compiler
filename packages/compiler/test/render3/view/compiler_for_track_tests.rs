
use indexmap::IndexMap;
use angular_compiler::output::output_ast as o;
use angular_compiler::render3::view::api::{
    R3ComponentMetadata, R3DirectiveMetadata, R3ComponentTemplate, R3ComponentDeferMetadata, DeclarationListEmitMode,
    R3HostMetadata, R3LifecycleMetadata, DeferBlockDepsEmitMode
};
use angular_compiler::render3::view::compiler::compile_component_from_metadata;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::parse_util::{ParseSourceSpan, ParseSourceFile, ParseLocation};
use angular_compiler::render3::util::R3Reference;
use angular_compiler::core::ViewEncapsulation;
use angular_compiler::template_parser::binding_parser::BindingParser;
use angular_compiler::expression_parser::parser::Parser;
use angular_compiler::schema::dom_element_schema_registry::DomElementSchemaRegistry;

#[path = "util.rs"]
mod util;
use util::{parse_r3, ParseR3Options};

fn compile_template(template: &str) -> (Vec<o::Statement>, ConstantPool) {
    let consts = parse_r3(template, ParseR3Options::default());
    
    // Create minimal metadata
    let source_file = ParseSourceFile::new("".to_string(), "test.ts".to_string());
    let start = ParseLocation::new(source_file.clone(), 0, 0, 0);
    let end = ParseLocation::new(source_file, 0, 0, 0);
    let type_span = ParseSourceSpan::new(start, end);
    
    // Initialize required registries/parsers for binding parser
    let parser = Parser::new();
    let schema_registry = DomElementSchemaRegistry::new();
    let binding_parser = BindingParser::new(&parser, &schema_registry, vec![]);
    
    let directive_meta = R3DirectiveMetadata {
        name: "TestComponent".to_string(),
        type_: R3Reference {
            value: *o::variable("TestComponent"),
            type_expr: *o::variable("TestComponent"), // Placeholder
        },
        type_argument_count: 0,
        type_source_span: type_span.clone(),
        deps: None,
        selector: Some("test-comp".to_string()),
        queries: vec![],
        view_queries: vec![],
        host: R3HostMetadata::default(),
        lifecycle: R3LifecycleMetadata::default(),
        inputs: IndexMap::new(),
        outputs: IndexMap::new(),
        uses_inheritance: false,
        export_as: None,
        providers: None,
        is_standalone: true,
        is_signal: false,
        host_directives: None,
    };

    let component_meta = R3ComponentMetadata {
        directive: directive_meta,
        template: R3ComponentTemplate {
            nodes: consts.nodes,
            ng_content_selectors: vec![],
            preserve_whitespaces: false,
        },
        declarations: vec![],
        defer: R3ComponentDeferMetadata::PerComponent { dependencies_fn: None },
        declaration_list_emit_mode: DeclarationListEmitMode::Direct,
        styles: vec![],
        external_styles: None,
        encapsulation: ViewEncapsulation::Emulated,
        animations: None,
        view_providers: None,
        relative_context_file_path: "test.ts".to_string(),
        i18n_use_external_ids: false,
        change_detection: None,
        relative_template_path: None,
        has_directive_dependencies: false,
        raw_imports: None,
    };

    let mut constant_pool = ConstantPool::new(false);
    let compiled = compile_component_from_metadata(&component_meta, &mut constant_pool, &binding_parser);
    
    let mut statements = constant_pool.statements.clone();
    if let o::Expression::InvokeFn(expr) = compiled.expression {
         // statements.push(o::Statement::Expression(o::ExpressionStatement { expr: Box::new(o::Expression::InvokeFn(expr)), source_span: None }));
         // In reality we just care about the constant pool outputs mostly for the trackBy function
    }
    
    (statements, constant_pool)
}

#[test]
fn should_use_zero_based_index_for_track_fn_name() {
    let template = "@for (item of items; track item) { {{ item }} }";
    let (statements, _) = compile_template(template);
    
    // Find declaration of _forTrack0
    let has_track0 = statements.iter().any(|stmt| {
        if let o::Statement::DeclareVar(decl) = stmt {
            decl.name == "_forTrack0"
        } else {
            false
        }
    });
    
    // Also check it's NOT using a large index (e.g. from decls)
    let has_large_index_track_fn = statements.iter().any(|stmt| {
         if let o::Statement::DeclareVar(decl) = stmt {
            decl.name.starts_with("_forTrack") && decl.name != "_forTrack0"
        } else {
            false
        }
    });

    assert!(has_track0, "Should have generated _forTrack0");
    assert!(!has_large_index_track_fn, "Should NOT have generated other _forTrack functions");
}

#[test]
fn should_increment_track_fn_index_for_multiple_loops() {
    let template = "
      @for (item of items; track item) { {{ item }} }
      @for (other of others; track other) { {{ other }} }
    ";
    let (statements, _) = compile_template(template);
    
    let has_track0 = statements.iter().any(|stmt| {
        if let o::Statement::DeclareVar(decl) = stmt {
            decl.name == "_forTrack0"
        } else {
            false
        }
    });

    let has_track1 = statements.iter().any(|stmt| {
        if let o::Statement::DeclareVar(decl) = stmt {
            decl.name == "_forTrack1"
        } else {
            false
        }
    });

    assert!(has_track0, "Should have generated _forTrack0. Statements: {:?}", statements);
    assert!(has_track1, "Should have generated _forTrack1. Statements: {:?}", statements);
}


#[test]
fn should_handle_ngfor_nested_svg_attributes() {
    let template = r#"<div *ngFor="let item of items"><svg width="100" height="100"><g *ngFor="let sub of item.subs"><circle cx="50" cy="50" r="40" stroke="green" stroke-width="4" fill="yellow" /></g></svg></div>"#;
    
    // Inline setup from compile_template to access 'compiled'
    let consts = parse_r3(template, ParseR3Options::default());
    let source_file = ParseSourceFile::new("".to_string(), "test.ts".to_string());
    let start = ParseLocation::new(source_file.clone(), 0, 0, 0);
    let end = ParseLocation::new(source_file, 0, 0, 0);
    let type_span = ParseSourceSpan::new(start, end);
    let parser = Parser::new();
    let schema_registry = DomElementSchemaRegistry::new();
    let binding_parser = BindingParser::new(&parser, &schema_registry, vec![]);
    
    let directive_meta = R3DirectiveMetadata {
        name: "TestComponent".to_string(),
        type_: R3Reference {
            value: *o::variable("TestComponent"),
            type_expr: *o::variable("TestComponent"),
        },
        type_argument_count: 0,
        type_source_span: type_span.clone(),
        deps: None,
        selector: Some("test-comp".to_string()),
        queries: vec![],
        view_queries: vec![],
        host: R3HostMetadata::default(),
        lifecycle: R3LifecycleMetadata::default(),
        inputs: IndexMap::new(),
        outputs: IndexMap::new(),
        uses_inheritance: false,
        export_as: None,
        providers: None,
        is_standalone: true,
        is_signal: false,
        host_directives: None,
    };

    let component_meta = R3ComponentMetadata {
        directive: directive_meta,
        template: R3ComponentTemplate {
            nodes: consts.nodes,
            ng_content_selectors: vec![],
            preserve_whitespaces: false,
        },
        declarations: vec![],
        defer: R3ComponentDeferMetadata::PerComponent { dependencies_fn: None },
        declaration_list_emit_mode: DeclarationListEmitMode::Direct,
        styles: vec![],
        external_styles: None,
        encapsulation: ViewEncapsulation::Emulated,
        animations: None,
        view_providers: None,
        relative_context_file_path: "test.ts".to_string(),
        i18n_use_external_ids: false,
        change_detection: None,
        relative_template_path: None,
        has_directive_dependencies: false,
        raw_imports: None,
    };

    let mut constant_pool = ConstantPool::new(false);
    let compiled = compile_component_from_metadata(&component_meta, &mut constant_pool, &binding_parser);
    
    let compiled_str = format!("{:?}", compiled.expression);
    
    // Verify structure
    // Note: The template main function code is inside the component definition
    
    // Verify constants are present in the output expression (likely in the 'consts' property of defining instruction)
    assert!(compiled_str.contains("stroke"), "Output should contain 'stroke'. output: {}", compiled_str);
    assert!(compiled_str.contains("green"));
    assert!(compiled_str.contains("fill"));
    assert!(compiled_str.contains("yellow"));
}









