#[cfg(test)]
mod tests {
    use crate::constant_pool::ConstantPool;
    use crate::render3::view::api::R3ComponentDeferMetadata;
    use crate::render3::view::template::parse_template;
    use crate::template::pipeline::ir;
    use crate::template::pipeline::src::compilation::TemplateCompilationMode;
    use crate::template::pipeline::src::ingest::ingest_component;

    #[test]
    fn test_structural_directive_nesting() {
        // A simple template with nested structural directives (*ngFor -> *ngIf)
        // This reproduces the structure where mismatch occurred (Element containing Template containing Element)
        let template_str = "<div *ngFor=\"let item of items\"><span *ngIf=\"item\"></span></div>";

        // Parse the template
        let parsed = parse_template(template_str, "test.html", Default::default());

        // Ingest
        // ingest_component creates the job and returns it.
        let job = ingest_component(
            "TestComp".to_string(),
            parsed.nodes,
            ConstantPool::default(),
            TemplateCompilationMode::Full,
            "test.ts".to_string(),
            false, // i18n_use_external_ids
            R3ComponentDeferMetadata::PerComponent {
                dependencies_fn: None,
            },
            None, // all_deferrable_deps_fn
            Some("test.html".to_string()),
            false,      // enable_debug_locations
            None,       // change_detection
            Vec::new(), // available_dependencies
        );

        // Verify Structure
        // Root view (0) should have one child view (ngFor)
        let root_xref = job.root.xref;

        // Find NgFor view. It should be a child of Root.
        // We look for a view whose parent is root_xref
        let ng_for_view = job.views.values().find(|v| v.parent == Some(root_xref));
        assert!(
            ng_for_view.is_some(),
            "Should have a view with Root as parent (NgFor)"
        );
        let ng_for_view = ng_for_view.unwrap();
        let ng_for_xref = ng_for_view.xref;

        // Find NgIf view. It should be a child of NgFor view.
        let ng_if_view = job.views.values().find(|v| v.parent == Some(ng_for_xref));
        assert!(
            ng_if_view.is_some(),
            "Should have a view with NgFor as parent (NgIf)"
        );
        let ng_if_view = ng_if_view.unwrap();
        let ng_if_xref = ng_if_view.xref;

        // Check contents of NgIf view
        // It should contain the span element (ElementStartOp)
        let has_span = ng_if_view.create.iter().any(|op| {
            if let Some(element_op) = op
                .as_any()
                .downcast_ref::<ir::ops::create::ElementStartOp>()
            {
                element_op.base.tag.as_deref() == Some("span")
            } else {
                false
            }
        });
        assert!(has_span, "NgIf view should contain span element");

        // Check contents of NgFor view
        // It should contain the div element
        let has_div = ng_for_view.create.iter().any(|op| {
            if let Some(element_op) = op
                .as_any()
                .downcast_ref::<ir::ops::create::ElementStartOp>()
            {
                element_op.base.tag.as_deref() == Some("div")
            } else {
                false
            }
        });
        assert!(has_div, "NgFor view should contain div element");

        // Check that div (in NgFor) contains the Template op for NgIf
        // We can iterate create ops of NgFor and look for TemplateOp
        let has_ng_if_template = ng_for_view.create.iter().any(|op| {
            if let Some(tmpl_op) = op.as_any().downcast_ref::<ir::ops::create::TemplateOp>() {
                // The template op should point to ng_if_xref
                tmpl_op.base.base.xref == ng_if_xref
            } else {
                false
            }
        });
        assert!(
            has_ng_if_template,
            "NgFor view should contain TemplateOp for NgIf child view"
        );

        println!(
            "Verified nesting: Root ({:?}) -> NgFor({:?}) -> NgIf({:?})",
            root_xref, ng_for_xref, ng_if_xref
        );
    }

    #[test]
    fn test_ng_template_ingest() {
        let template_str = r#"<ng-template let-formFieldId="id">
  <div
    class="mat-mdc-autocomplete-panel mdc-menu-surface mdc-menu-surface--open"
    role="listbox"
    [id]="id"
    [class]="_classList"
    #panel>
    <ng-content></ng-content>
  </div>
</ng-template>"#;
        let parsed = parse_template(
            template_str,
            "test.html",
            crate::render3::view::template::ParseTemplateOptions {
                preserve_whitespaces: Some(false),
                ..Default::default()
            },
        );

        eprintln!("DEBUG: Parsed nodes: {:?}", parsed.nodes);

        let job = ingest_component(
            "TestComp".to_string(),
            parsed.nodes.clone(),
            ConstantPool::default(),
            TemplateCompilationMode::Full,
            "test.ts".to_string(),
            false,
            R3ComponentDeferMetadata::PerComponent {
                dependencies_fn: None,
            },
            None,
            Some("test.html".to_string()),
            false,
            None,
            Vec::new(),
        );

        // Run phases
        let mut job = job;
        crate::template::pipeline::src::phases::run(&mut job);

        // Create explicit source file and locations for span
        let source_file = std::sync::Arc::new(crate::parse_util::ParseSourceFile::new(
            "".to_string(),
            "".to_string(),
        ));
        let start = crate::parse_util::ParseLocation::new(source_file.clone(), 0, 0, 0);
        let end = crate::parse_util::ParseLocation::new(source_file.clone(), 0, 0, 0);

        // Emit
        let meta = crate::render3::view::api::R3ComponentMetadata {
            directive: crate::render3::view::api::R3DirectiveMetadata {
                name: "TestComp".to_string(),
                selector: Some("test-comp".to_string()),
                export_as: None,
                inputs: Default::default(),
                outputs: Default::default(),
                host: Default::default(),
                lifecycle: Default::default(),
                queries: Default::default(),
                view_queries: Default::default(),
                uses_inheritance: false,
                type_argument_count: 0,
                type_source_span: crate::parse_util::ParseSourceSpan::new(start, end),
                is_standalone: true,
                is_signal: false,
                deps: None,
                providers: None,
                type_: crate::render3::util::R3Reference {
                    value: crate::output::output_ast::Expression::Literal(
                        crate::output::output_ast::LiteralExpr {
                            value: crate::output::output_ast::LiteralValue::Null,
                            type_: None,
                            source_span: None,
                        },
                    ),
                    type_expr: crate::output::output_ast::Expression::Literal(
                        crate::output::output_ast::LiteralExpr {
                            value: crate::output::output_ast::LiteralValue::Null,
                            type_: None,
                            source_span: None,
                        },
                    ),
                },
                host_directives: None,
            },
            template: crate::render3::view::api::R3ComponentTemplate {
                nodes: parsed.nodes,
                ng_content_selectors: Vec::new(),
                preserve_whitespaces: false,
            },
            declarations: Vec::new(),
            change_detection: None,
            encapsulation: crate::core::ViewEncapsulation::Emulated,
            relative_context_file_path: "test.ts".to_string(),
            relative_template_path: Some("test.html".to_string()),
            i18n_use_external_ids: false,
            defer: crate::render3::view::api::R3ComponentDeferMetadata::PerComponent {
                dependencies_fn: None,
            },
            styles: Vec::new(),
            external_styles: None,
            animations: None,
            view_providers: None,
            has_directive_dependencies: false,
            raw_imports: None,
            declaration_list_emit_mode: crate::render3::view::api::DeclarationListEmitMode::Direct,
        };

        let compiled = crate::template::pipeline::src::emit::emit_component(&job, &meta, None);

        // Convert expression to string to check output
        // We verify that it contains "ng-template" string argument
        let output = format!("{:?}", compiled.expression);
        let output = format!("{:?}", compiled.expression);
        assert!(
            output.contains("\"ng-template\""),
            "Output should contain 'ng-template' argument in domTemplate instruction: {}",
            output
        );
    }
}
