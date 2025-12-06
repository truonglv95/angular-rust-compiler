//! Source Map Module
//!
//! Corresponds to packages/compiler/src/output/source_map.ts
//! Source map generation for compiled code

use crate::util::utf8_encode;
use std::collections::HashMap;

// https://docs.google.com/document/d/1U1RGAehQwRypUTovF1KRlpiOFze0b-_2gc6fAH0KY0k/edit
const VERSION: u32 = 3;
const JS_B64_PREFIX: &str = "# sourceMappingURL=data:application/json;base64,";

#[derive(Debug, Clone)]
struct Segment {
    col0: usize,
    source_url: Option<String>,
    source_line0: Option<usize>,
    source_col0: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SourceMap {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(rename = "sourceRoot")]
    pub source_root: String,
    pub sources: Vec<String>,
    #[serde(rename = "sourcesContent")]
    pub sources_content: Vec<Option<String>>,
    pub mappings: String,
}

pub struct SourceMapGenerator {
    file: Option<String>,
    sources_content: HashMap<String, Option<String>>,
    lines: Vec<Vec<Segment>>,
    last_col0: usize,
    has_mappings: bool,
}

impl SourceMapGenerator {
    pub fn new(file: Option<String>) -> Self {
        SourceMapGenerator {
            file,
            sources_content: HashMap::new(),
            lines: Vec::new(),
            last_col0: 0,
            has_mappings: false,
        }
    }

    /// The content is `None` when the content is expected to be loaded using the URL
    pub fn add_source(&mut self, url: String, content: Option<String>) -> &mut Self {
        self.sources_content.entry(url).or_insert(content);
        self
    }

    pub fn add_line(&mut self) -> &mut Self {
        self.lines.push(Vec::new());
        self.last_col0 = 0;
        self
    }

    pub fn add_mapping(
        &mut self,
        col0: usize,
        source_url: Option<String>,
        source_line0: Option<usize>,
        source_col0: Option<usize>,
    ) -> Result<&mut Self, String> {
        if self.current_line().is_none() {
            return Err("A line must be added before mappings can be added".to_string());
        }

        if let Some(url) = &source_url {
            if !self.sources_content.contains_key(url) {
                return Err(format!("Unknown source file \"{}\"", url));
            }
        }

        if col0 < self.last_col0 {
            return Err("Mapping should be added in output order".to_string());
        }

        if source_url.is_some() && (source_line0.is_none() || source_col0.is_none()) {
            return Err("The source location must be provided when a source url is provided".to_string());
        }

        self.has_mappings = true;
        self.last_col0 = col0;

        if let Some(line) = self.current_line_mut() {
            line.push(Segment {
                col0,
                source_url,
                source_line0,
                source_col0,
            });
        }

        Ok(self)
    }

    fn current_line(&self) -> Option<&Vec<Segment>> {
        self.lines.last()
    }

    fn current_line_mut(&mut self) -> Option<&mut Vec<Segment>> {
        self.lines.last_mut()
    }

    pub fn to_json(&self) -> Option<SourceMap> {
        if !self.has_mappings {
            return None;
        }

        let mut sources_index: HashMap<String, usize> = HashMap::new();
        let mut sources: Vec<String> = Vec::new();
        let mut sources_content: Vec<Option<String>> = Vec::new();

        let mut sorted_keys: Vec<_> = self.sources_content.keys().collect();
        sorted_keys.sort();

        for (i, url) in sorted_keys.iter().enumerate() {
            sources_index.insert((*url).clone(), i);
            sources.push((*url).clone());
            sources_content.push(self.sources_content.get(*url).and_then(|c| c.clone()));
        }

        let mut mappings = String::new();
        let mut last_col0 = 0;
        let mut last_source_index = 0;
        let mut last_source_line0 = 0;
        let mut last_source_col0 = 0;

        for segments in &self.lines {
            last_col0 = 0;

            let seg_strs: Vec<String> = segments
                .iter()
                .map(|segment| {
                    // zero-based starting column of the line in the generated code
                    let mut seg_as_str = to_base64_vlq(segment.col0 as i32 - last_col0 as i32);
                    last_col0 = segment.col0;

                    if let Some(ref source_url) = segment.source_url {
                        // zero-based index into the "sources" list
                        let source_idx = *sources_index.get(source_url).unwrap();
                        seg_as_str += &to_base64_vlq(source_idx as i32 - last_source_index as i32);
                        last_source_index = source_idx;

                        // the zero-based starting line in the original source
                        seg_as_str += &to_base64_vlq(segment.source_line0.unwrap() as i32 - last_source_line0 as i32);
                        last_source_line0 = segment.source_line0.unwrap();

                        // the zero-based starting column in the original source
                        seg_as_str += &to_base64_vlq(segment.source_col0.unwrap() as i32 - last_source_col0 as i32);
                        last_source_col0 = segment.source_col0.unwrap();
                    }

                    seg_as_str
                })
                .collect();

            mappings += &seg_strs.join(",");
            mappings.push(';');
        }

        // Remove trailing semicolon
        mappings.pop();

        Some(SourceMap {
            file: self.file.clone(),
            version: VERSION,
            source_root: String::new(),
            sources,
            sources_content,
            mappings,
        })
    }

    pub fn to_js_comment(&self) -> String {
        if self.has_mappings {
            if let Some(source_map) = self.to_json() {
                let json = serde_json::to_string(&source_map).unwrap_or_default();
                return format!("//{}{}", JS_B64_PREFIX, to_base64_string(&json));
            }
        }
        String::new()
    }
}

pub fn to_base64_string(value: &str) -> String {
    let encoded = utf8_encode(value);
    let mut b64 = String::new();
    let mut i = 0;

    while i < encoded.len() {
        let i1 = encoded[i];
        i += 1;
        let i2 = if i < encoded.len() { Some(encoded[i]) } else { None };
        if i2.is_some() {
            i += 1;
        }
        let i3 = if i < encoded.len() { Some(encoded[i]) } else { None };
        if i3.is_some() {
            i += 1;
        }

        b64.push(to_base64_digit(i1 >> 2));
        b64.push(to_base64_digit(
            ((i1 & 3) << 4) | (i2.unwrap_or(0) >> 4),
        ));
        b64.push(if i2.is_none() {
            '='
        } else {
            to_base64_digit(((i2.unwrap() & 15) << 2) | (i3.unwrap_or(0) >> 6))
        });
        b64.push(if i2.is_none() || i3.is_none() {
            '='
        } else {
            to_base64_digit(i3.unwrap() & 63)
        });
    }

    b64
}

fn to_base64_vlq(mut value: i32) -> String {
    value = if value < 0 {
        (-value << 1) + 1
    } else {
        value << 1
    };

    let mut out = String::new();
    loop {
        let mut digit = value & 31;
        value >>= 5;
        if value > 0 {
            digit |= 32;
        }
        out.push(to_base64_digit(digit as u8));

        if value <= 0 {
            break;
        }
    }

    out
}

const B64_DIGITS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn to_base64_digit(value: u8) -> char {
    if value >= 64 {
        panic!("Can only encode value in the range [0, 63]");
    }
    B64_DIGITS.chars().nth(value as usize).unwrap()
}





