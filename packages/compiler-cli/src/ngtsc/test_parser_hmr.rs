#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    #[test]
    fn test_parse_hmr_init() {
        let allocator = Allocator::default();
        let source_type = SourceType::mjs();
        let hmr_init = r#"(function () {
  const id = "feature/dashboard/dashboard.component.ts-DashboardComponent";
  function DashboardComponent_HmrLoad(t) {
    __vite_ignore_import(i0.ɵɵgetReplaceMetadataURL(id, t, import.meta.url)).then((m) => m.default && i0.ɵɵreplaceMetadata(DashboardComponent, m.default, [i0], [], import.meta, id));
  }
  (typeof ngDevMode === "undefined" || ngDevMode) && DashboardComponent_HmrLoad(Date.now());
  (typeof ngDevMode === "undefined" || ngDevMode) && (import.meta.hot && import.meta.hot.on("angular:component-update", (d) => d.id === id && DashboardComponent_HmrLoad(d.timestamp)));
})();
"#;
        let parser = Parser::new(&allocator, hmr_init, source_type);
        let parse_result = parser.parse();
        assert!(
            parse_result.errors.is_empty(),
            "Parse Error: {:#?}",
            parse_result.errors
        );
    }
}
