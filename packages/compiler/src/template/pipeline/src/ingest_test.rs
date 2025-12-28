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
}
