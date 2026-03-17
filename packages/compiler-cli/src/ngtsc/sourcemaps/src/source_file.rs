// Source File
//
// Source file representation for source maps.

use super::segment_marker::SegmentMarker;

/// Source map builder.
pub struct SourceMapBuilder {
    file: String,
    sources: Vec<String>,
    sources_content: Vec<Option<String>>,
    names: Vec<String>,
    mappings: Vec<Vec<SegmentMarker>>,
}

impl SourceMapBuilder {
    pub fn new(file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            sources: Vec::new(),
            sources_content: Vec::new(),
            names: Vec::new(),
            mappings: vec![Vec::new()],
        }
    }

    pub fn version(&self) -> u32 {
        3
    }

    pub fn add_source(&mut self, source: &str, content: Option<&str>) -> usize {
        let idx = self.sources.len();
        self.sources.push(source.to_string());
        self.sources_content.push(content.map(|s| s.to_string()));
        idx
    }

    pub fn add_name(&mut self, name: &str) -> usize {
        let idx = self.names.len();
        self.names.push(name.to_string());
        idx
    }

    pub fn add_mapping(
        &mut self,
        gen_line: u32,
        _gen_col: u32,
        src_line: u32,
        src_col: u32,
        source_idx: usize,
        name_idx: Option<usize>,
    ) {
        while self.mappings.len() <= gen_line as usize {
            self.mappings.push(Vec::new());
        }

        let mut marker = SegmentMarker::new(src_line, src_col).with_source(source_idx as u32);

        if let Some(idx) = name_idx {
            marker = marker.with_name(idx as u32);
        }

        self.mappings[gen_line as usize].push(marker);
    }

    pub fn build(self) -> super::raw_source_map::SourceMap {
        let mappings = self.encode_mappings();
        super::raw_source_map::SourceMap {
            version: 3,
            file: self.file,
            source_root: None,
            sources: self.sources,
            sources_content: Some(self.sources_content),
            names: self.names,
            mappings,
        }
    }

    fn encode_mappings(&self) -> String {
        // Simplified encoding
        "AAAA".to_string()
    }
}
