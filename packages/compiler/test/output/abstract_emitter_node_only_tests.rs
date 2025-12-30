use angular_compiler::output::abstract_emitter::{EmitterVisitorContext, HasSourceSpan};
use angular_compiler::output::source_map::SourceMap as EmitterSourceMap;
use angular_compiler::parse_util::{ParseLocation, ParseSourceFile, ParseSourceSpan};

#[path = "source_map_util.rs"]
mod source_map_util;

use source_map_util::{original_position_for, SourceMap as UtilSourceMap};

// Mock struct implementing HasSourceSpan for testing
#[derive(Clone)]
struct MockSourceSpan(ParseSourceSpan);

impl HasSourceSpan for MockSourceSpan {
    fn source_span(&self) -> Option<&ParseSourceSpan> {
        Some(&self.0)
    }
}

// Function to create a source span
fn create_source_span(file: &ParseSourceFile, idx: usize) -> MockSourceSpan {
    let col = 2 * idx;
    let start = ParseLocation {
        file: file.clone().into(),
        col: col,
        line: 0,
        offset: col,
    };
    let end = ParseLocation {
        file: file.clone().into(),
        col: col + 2,
        line: 0,
        offset: col + 2,
    };
    MockSourceSpan(ParseSourceSpan {
        start,
        end,
        details: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to count segments per line
    fn nb_segments_per_line(ctx: &EmitterVisitorContext) -> Vec<usize> {
        let sm_opt = ctx.to_source_map_generator("o.ts", 0).to_json();
        let sm = sm_opt.expect("Source map should be generated");
        // Convert EmitterSourceMap to UtilSourceMap for parsing
        let util_sm = convert_to_util_source_map(&sm);
        util_sm
            .mappings
            .split(';')
            .map(|l| {
                if l.is_empty() {
                    return 1;
                }
                l.matches(',').count() + 1
            })
            .collect()
    }

    // Convert EmitterSourceMap to UtilSourceMap
    fn convert_to_util_source_map(sm: &EmitterSourceMap) -> UtilSourceMap {
        UtilSourceMap {
            version: sm.version,
            file: sm.file.clone(),
            source_root: Some(sm.source_root.clone()),
            sources: sm.sources.clone(),
            sources_content: Some(sm.sources_content.clone()),
            mappings: sm.mappings.clone(),
        }
    }

    // Helper for assertions
    fn expect_map(
        ctx: &EmitterVisitorContext,
        gen_line: u32,
        gen_col: u32,
        source: Option<&str>,
        src_line: Option<u32>,
        src_col: Option<u32>,
    ) {
        let sm_opt = ctx.to_source_map_generator("o.ts", 0).to_json();
        let sm = sm_opt.expect("Source map should be generated");
        let util_sm = convert_to_util_source_map(&sm);
        let orig_pos = original_position_for(&util_sm, gen_line, gen_col);

        assert_eq!(orig_pos.source.as_deref(), source, "Source mismatch");
        assert_eq!(
            orig_pos.line,
            src_line.map(|l| l + 1),
            "Line mismatch (expected 1-based)"
        ); // original_position_for returns 1-based line
        assert_eq!(orig_pos.column, src_col, "Column mismatch");
    }

    #[test]
    fn should_add_source_files_to_the_source_map() {
        let file_a = ParseSourceFile {
            content: "a0a1a2a3a4a5a6a7a8a9".to_string(),
            url: "a.js".to_string(),
        };
        let file_b = ParseSourceFile {
            content: "b0b1b2b3b4b5b6b7b8b9".to_string(),
            url: "b.js".to_string(),
        };

        let mut ctx = EmitterVisitorContext::create_root();

        ctx.print(Some(&create_source_span(&file_a, 0)), "o0", false);
        ctx.print(Some(&create_source_span(&file_a, 1)), "o1", false);
        ctx.print(Some(&create_source_span(&file_b, 0)), "o2", false);
        ctx.print(Some(&create_source_span(&file_b, 1)), "o3", false);

        let sm = ctx.to_source_map_generator("o.ts", 0).to_json().unwrap();
        // TypeScript test: expect(sm.sources).toEqual([fileA.url, fileB.url]);
        // Sources might be sorted in implementation
        let mut sources = sm.sources.clone();
        sources.sort();
        assert_eq!(sources, vec![file_a.url.clone(), file_b.url.clone()]);

        // TypeScript test: expect(sm.sourcesContent).toEqual([fileA.content, fileB.content]);
        // Check content - sources_content is Vec<Option<String>>, not Option<Vec<...>>
        assert_eq!(sm.sources_content.len(), 2);
        // Find a.js and b.js content in the vec
        let content_a = sm
            .sources_content
            .iter()
            .find(|c| c.as_ref().map(|s| s == &file_a.content).unwrap_or(false));
        let content_b = sm
            .sources_content
            .iter()
            .find(|c| c.as_ref().map(|s| s == &file_b.content).unwrap_or(false));
        assert!(content_a.is_some(), "a.js content not found");
        assert!(content_b.is_some(), "b.js content not found");
    }

    #[test]
    fn should_generate_a_valid_mapping() {
        let file_a = ParseSourceFile {
            content: "a0a1a2a3a4a5a6a7a8a9".to_string(),
            url: "a.js".to_string(),
        };
        let file_b = ParseSourceFile {
            content: "b0b1b2b3b4b5b6b7b8b9".to_string(),
            url: "b.js".to_string(),
        };
        let mut ctx = EmitterVisitorContext::create_root();

        ctx.print(Some(&create_source_span(&file_a, 0)), "fileA-0", false);
        ctx.println(Some(&create_source_span(&file_b, 1)), "fileB-1");
        ctx.print(Some(&create_source_span(&file_a, 2)), "fileA-2", false);

        expect_map(&ctx, 0, 0, Some("a.js"), Some(0), Some(0));
        expect_map(&ctx, 0, 7, Some("b.js"), Some(0), Some(2));
        expect_map(&ctx, 1, 0, Some("a.js"), Some(0), Some(4));
    }

    #[test]
    fn should_be_able_to_shift_the_content() {
        let file_a = ParseSourceFile {
            content: "a0a1a2a3a4a5a6a7a8a9".to_string(),
            url: "a.js".to_string(),
        };
        let mut ctx = EmitterVisitorContext::create_root();

        ctx.print(Some(&create_source_span(&file_a, 0)), "fileA-0", false);

        let sm_opt = ctx.to_source_map_generator("o.ts", 10).to_json();
        let sm = sm_opt.expect("Source map should be generated");
        let util_sm = convert_to_util_source_map(&sm);
        let orig_pos = original_position_for(&util_sm, 10, 0); // Line 11 (0-based 10)

        assert_eq!(orig_pos.source.as_deref(), Some("a.js"));
        assert_eq!(orig_pos.line, Some(1)); // 0+1
        assert_eq!(orig_pos.column, Some(0));
    }

    #[test]
    fn should_use_the_default_source_file_for_the_first_character() {
        let mut ctx = EmitterVisitorContext::create_root();
        ctx.print(None, "fileA-0", false);

        // This test expects 'o.ts' (gen file) as source when no source is provided?
        // or just no mapping?
        // TS Test: expectMap(ctx, 0, 0, 'o.ts', 0, 0);
        // This implies that if no source span is given, it maps to the generated file itself?
        // Or SourceMapGenerator does something special?
        // Let's check `SourceMapGenerator::to_source_map_generator` in `abstract_emitter.rs`.
        // It adds " " content for gen_file_path and adds a mapping (0, gen_file_path, 0, 0) IF !first_offset_mapped.
        // So yes, it maps to generated file.

        expect_map(&ctx, 0, 0, Some("o.ts"), Some(0), Some(0));
    }

    #[test]
    fn should_use_an_explicit_mapping_for_the_first_character() {
        let file_a = ParseSourceFile {
            content: "a0a1a2a3a4a5a6a7a8a9".to_string(),
            url: "a.js".to_string(),
        };
        let mut ctx = EmitterVisitorContext::create_root();
        ctx.print(Some(&create_source_span(&file_a, 0)), "fileA-0", false);

        expect_map(&ctx, 0, 0, Some("a.js"), Some(0), Some(0));
    }

    #[test]
    fn should_map_leading_segment_without_span() {
        let file_a = ParseSourceFile {
            content: "a0a1a2a3a4a5a6a7a8a9".to_string(),
            url: "a.js".to_string(),
        };
        let mut ctx = EmitterVisitorContext::create_root();

        ctx.print(None, "....", false);
        ctx.print(Some(&create_source_span(&file_a, 0)), "fileA-0", false);

        expect_map(&ctx, 0, 0, Some("o.ts"), Some(0), Some(0));
        expect_map(&ctx, 0, 4, Some("a.js"), Some(0), Some(0));
        assert_eq!(nb_segments_per_line(&ctx), vec![2]);
    }

    #[test]
    fn should_handle_indent() {
        let file_a = ParseSourceFile {
            content: "a0a1a2a3a4a5a6a7a8a9".to_string(),
            url: "a.js".to_string(),
        };
        let mut ctx = EmitterVisitorContext::create_root();

        ctx.inc_indent();
        ctx.println(Some(&create_source_span(&file_a, 0)), "fileA-0");
        ctx.inc_indent();
        ctx.println(Some(&create_source_span(&file_a, 1)), "fileA-1");
        ctx.dec_indent();
        ctx.println(Some(&create_source_span(&file_a, 2)), "fileA-2");

        // Line 0: "  fileA-0"
        // Indent is 2 spaces.
        // to_source_map_generator iterates parts.
        // It adds mapping for indent? No, indent is handled by `col0` setup but loop iterates `parts`.
        // `parts` contains the printed strings. Indent is prefixed when converting to string, but for source map...
        // `abstract_emitter.rs`: `let mut col0 = line.indent * INDENT_WITH.len();`
        // Then it iterates parts.
        // So the first mapping starts at col0 (indent length).
        // What about col 0 to indent?
        // `if !first_offset_mapped` block handles mapping 0 to o.ts if nothing mapped yet.
        // So:
        // Line 0: 0->2 (o.ts), 2->... (a.js)
        expect_map(&ctx, 0, 0, Some("o.ts"), Some(0), Some(0));
        expect_map(&ctx, 0, 2, Some("a.js"), Some(0), Some(0));

        // Line 1: "    fileA-1" (indent 4)
        // 0 -> 4 (o.ts - implicit? No, explicit mapping added for o.ts is only done ONCE globally if !first_offset_mapped)
        // Wait, `to_source_map_generator` logic:
        // `if !first_offset_mapped { ... add o.ts mapping ... first_offset_mapped = true }`
        // This is inside the loop over lines -> parts.
        // If line 0 triggered it, line 1 won't.
        // So line 1 col 0 has NO mapping?
        // Segments: `col0` starts at 4.
        // So first segment at 4.
        // So 0 to 4 is unmapped / inherits previous line?
        // Source maps usually inherit.
        // But TS test says: expectMap(ctx, 1, 0) -> undefined/null check?
        // TS Test:
        // expectMap(ctx, 1, 0); // No expectations provided -> source: null, line: null, col: null
        // expectMap(ctx, 1, 2); // No expectations
        // expectMap(ctx, 1, 4, 'a.js', 0, 2);

        expect_map(&ctx, 1, 0, None, None, None);
        expect_map(&ctx, 1, 2, None, None, None);
        expect_map(&ctx, 1, 4, Some("a.js"), Some(0), Some(2));

        expect_map(&ctx, 2, 0, None, None, None);
        expect_map(&ctx, 2, 2, Some("a.js"), Some(0), Some(4));

        assert_eq!(nb_segments_per_line(&ctx), vec![2, 1, 1]);
    }

    #[test]
    fn should_coalesce_identical_span() {
        let file_a = ParseSourceFile {
            content: "a0a1a2a3a4a5a6a7a8a9".to_string(),
            url: "a.js".to_string(),
        };
        let file_b = ParseSourceFile {
            content: "b0b1b2b3b4b5b6b7b8b9".to_string(),
            url: "b.js".to_string(),
        };
        let mut ctx = EmitterVisitorContext::create_root();

        let span_a0 = create_source_span(&file_a, 0);
        ctx.print(Some(&span_a0), "fileA-0", false);
        ctx.print(None, "...", false);
        ctx.print(Some(&span_a0), "fileA-0", false);
        ctx.print(Some(&create_source_span(&file_b, 0)), "fileB-0", false);

        expect_map(&ctx, 0, 0, Some("a.js"), Some(0), Some(0));
        expect_map(&ctx, 0, 7, Some("a.js"), Some(0), Some(0)); // ... inherits? No, "..." printed with None.
                                                                // If printed with None, does it add a segment?
                                                                // `abstract_emitter.rs`: `if let Some(Some(span)) = line.src_spans.get(i)`
                                                                // If `src_spans` has `None`, it DOES NOT add a segment.
                                                                // So it inherits the previous segment's mapping.
                                                                // Previous was `a.js` at 0,0.
                                                                // So valid check.

        expect_map(&ctx, 0, 10, Some("a.js"), Some(0), Some(0)); // Second fileA-0, same span.
                                                                 // Does implementation coalesce?
                                                                 // Logic just adds mapping. If it adds same mapping again?
                                                                 // `to_json` in `source_map.rs` iterates segments.
                                                                 // If it emits a segment identical to previous, it's redundant but valid.
                                                                 // TS `nbSegmentsPerLine` result `[2]` suggests only 2 segments were emitted.
                                                                 // 1. fileA-0 (start)
                                                                 // 2. fileB-0 (start)
                                                                 // Middle ones were skipped/coalesced?
                                                                 // `source_map.rs` doesn't seem to have coalesce logic?
                                                                 // Wait, `abstract_emitter.rs` adds mapping if span is present.
                                                                 // If span is same as previous, `SourceMapGenerator` adds it.
                                                                 // Does `SourceMapGenerator.to_json` coalesce?
                                                                 // It calculates deltas. `to_base64_vlq(segment.col0... - last_col0...)`.
                                                                 // If deltas are 0 (except column), it emits it.
                                                                 // Standard VLQ minimization might drop it?
                                                                 // "coalesce identical span" usually means checking if new segment maps to same source location.
                                                                 // If so, skip adding it.
                                                                 // Let's see if Rust `SourceMapGenerator` handles this? No visible logic.
                                                                 // Maybe TS implementation does?
                                                                 // If my Rust test fails on `nb_segments_per_line`, I'll know.

        expect_map(&ctx, 0, 17, Some("b.js"), Some(0), Some(0));

        // Note: The TS expect is `[2]`. If I get `[4]`, I will need to implement coalescing in `SourceMapGenerator` or `AbstractEmitter`.
        // Let's assert what we get and fix if needed. For now I'll match TS expectation.
        assert_eq!(nb_segments_per_line(&ctx), vec![2]);
    }
}
