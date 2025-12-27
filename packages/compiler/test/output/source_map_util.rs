use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct SourceMap {
    pub version: u32,
    pub file: Option<String>,
    #[serde(rename = "sourceRoot")]
    pub source_root: Option<String>,
    pub sources: Vec<String>,
    #[serde(rename = "sourcesContent")]
    pub sources_content: Option<Vec<Option<String>>>,
    pub mappings: String,
}

#[derive(Debug, PartialEq)]
pub struct SourceLocation {
    pub line: Option<u32>,   // 1-based
    pub column: Option<u32>, // 0-based
    pub source: Option<String>,
}

#[allow(dead_code)]
const B64_DIGITS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[allow(dead_code)]
fn b64_value(c: char) -> Option<i32> {
    B64_DIGITS.find(c).map(|v| v as i32)
}

#[allow(dead_code)]
fn decode_vlq(input: &str) -> (Vec<i32>, usize) {
    let mut values = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let mut value = 0;
        let mut shift = 0;
        let mut continuation = true;

        while continuation && i < chars.len() {
            let digit = b64_value(chars[i]).expect("Invalid base64 char");
            i += 1;
            continuation = (digit & 32) != 0;
            value += (digit & 31) << shift;
            shift += 5;
        }

        let should_negate = (value & 1) != 0;
        value >>= 1;
        if should_negate {
            value = -value;
        }
        values.push(value);
    }
    (values, i)
}

#[allow(dead_code)]
pub fn original_position_for(
    source_map: &SourceMap,
    gen_line: u32,
    gen_col: u32,
) -> SourceLocation {
    let mappings: Vec<&str> = source_map.mappings.split(';').collect();

    // State
    let mut current_gen_line = 0;
    // let mut current_gen_col = 0; // Reset every line? No, relative to previous segment
    let mut source_idx = 0;
    let mut source_line = 0; // 0-based in mappings
    let mut source_col = 0;
    // let mut name_idx = 0;

    // Iterate lines
    for (line_idx, line) in mappings.iter().enumerate() {
        let mut current_gen_col = 0;

        if (line_idx as u32) > gen_line {
            break;
        }

        if line.is_empty() {
            if (line_idx as u32) == gen_line {
                // No mappings for this line
                return SourceLocation {
                    line: None,
                    column: None,
                    source: None,
                };
            }
            continue;
        }

        let segments: Vec<&str> = line.split(',').collect();
        for segment in segments {
            let (values, _) = decode_vlq(segment);

            if values.is_empty() {
                continue;
            }

            // 0: gen col (relative)
            current_gen_col += values[0];

            let mut current_segment_matches = false;
            if (line_idx as u32) == gen_line {
                // We are on the target line.
                // Check if this segment covers the target column.
                // In source maps, a segment starts at `current_gen_col` and goes until the next segment.
                // However, `originalPositionFor` usually looks for the segment *at or before* the requested column.
                // But simplified: exact match or closest preceding?
                // The Mozilla implementation uses greatest lower bound.
                if (current_gen_col as u32) <= gen_col {
                    current_segment_matches = true;
                } else {
                    // This segment is after the target column, and we were looking for the greatest lower bound.
                    // So the PREVIOUS segment was the one suitable.
                    // But we simply update state and return the state at the end of loop if we find a later segment?
                    // Let's just track the "last matching segment".
                }
            }

            if values.len() >= 4 {
                // 1: source idx (relative)
                source_idx += values[1];
                // 2: source line (relative)
                source_line += values[2];
                // 3: source col (relative)
                source_col += values[3];

                // 4: name idx (relative) - optional
                // if values.len() >= 5 { name_idx += values[4]; }

                if (line_idx as u32) == gen_line && (current_gen_col as u32) <= gen_col {
                    // Potential candidate. Since we iterate in order, later segments on the same line overwrites earlier ones
                    // as long as they are <= gen_col.
                    // But we need to save this candidate.
                }
            }
        }

        // Re-implement logic properly:
        // We need to replay the whole map up to the target point to maintain state properly.
        // Or at least replay the state updates.
    }

    // Let's rewrite cleaner:
    // Reset state
    let mut state_source_idx = 0;
    let mut state_source_line = 0;
    let mut state_source_col = 0;
    // let mut state_name_idx = 0;

    let mut found_src_line = None;
    let mut found_src_col = None;
    let mut found_src_idx = None;

    for (line_idx, line) in mappings.iter().enumerate() {
        let mut state_gen_col = 0;
        let mut last_valid_for_this_line: Option<(usize, i32, i32)> = None; // idx, line, col

        if line.is_empty() {
            if (line_idx as u32) == gen_line {
                break; // Target line empty
            }
            continue;
        }

        let segments = line.split(',');
        for segment in segments {
            let (values, _) = decode_vlq(segment);
            if values.is_empty() {
                continue;
            }

            state_gen_col += values[0];

            if values.len() >= 4 {
                state_source_idx += values[1];
                state_source_line += values[2];
                state_source_col += values[3];
                // if values.len() >= 5 { state_name_idx += values[4]; }

                if (line_idx as u32) == gen_line {
                    if (state_gen_col as u32) <= gen_col {
                        last_valid_for_this_line = Some((
                            state_source_idx as usize,
                            state_source_line,
                            state_source_col,
                        ));
                    } else {
                        // Past the column
                        break;
                    }
                }
            } else if (line_idx as u32) == gen_line && (state_gen_col as u32) <= gen_col {
                // Segment without source info (e.g. unmapped code generated).
                // If we hit this, it might mean the position maps to nothing?
                // Or we keep the previous mapping?
                // Usually if a segment has 1 field, it resets the mapping to "generated code, no source".
                last_valid_for_this_line = None;
            }
        }

        if (line_idx as u32) == gen_line {
            if let Some((idx, line, col)) = last_valid_for_this_line {
                found_src_idx = Some(idx);
                found_src_line = Some(line);
                found_src_col = Some(col);
            }
            break;
        }
    }

    if let (Some(idx), Some(line), Some(col)) = (found_src_idx, found_src_line, found_src_col) {
        let source_url = if idx < source_map.sources.len() {
            Some(source_map.sources[idx].clone())
        } else {
            None
        };
        SourceLocation {
            line: Some((line + 1) as u32), // Convert to 1-based
            column: Some(col as u32),
            source: source_url,
        }
    } else {
        SourceLocation {
            line: None,
            column: None,
            source: None,
        }
    }
}

#[allow(dead_code)]
pub fn extract_source_map(source: &str) -> Option<SourceMap> {
    if let Some(pos) = source.rfind("\n//#") {
        let comment_part = &source[pos..];
        let lines: Vec<&str> = comment_part.splitn(2, '\n').collect();
        if lines.len() > 1 {
            let sm_comment = lines[1].trim();
            if let Some(b64_start) =
                sm_comment.find("sourceMappingURL=data:application/json;base64,")
            {
                let b64 = &sm_comment
                    [b64_start + "sourceMappingURL=data:application/json;base64,".len()..];
                // Simple B64 decode using an external crate or custom?
                // Since this is a test utility, I can use `base64` crate if available or minimal impl.
                // The `source_map.rs` used `to_base64_string` but we need decode.
                // Let's use a minimal decode or just assume `serde_json` and `base64` are NOT available?
                // `serde` and `serde_json` ARE available in Cargo.toml.
                // `base64` is NOT explicitly in dependencies list seen earlier (only `serde`, `serde_json`, `indexmap`, `regex`, `once_cell`, `anyhow`, `thiserror`, `rayon`, `smallvec`).
                // I'll implement a minimal base64 decoder.
                if let Ok(json_str) = minimal_base64_decode(b64) {
                    return serde_json::from_str(&json_str).ok();
                }
            }
        }
    }
    None
}

fn minimal_base64_decode(input: &str) -> Result<String, ()> {
    // This is a rough decoder for testing purposes
    let mut output = Vec::new();
    let mut buffer = 0;
    let mut bits = 0;

    for c in input.chars() {
        if c == '=' {
            break;
        }
        if let Some(val) = B64_DIGITS.find(c) {
            buffer = (buffer << 6) | val;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                output.push((buffer >> bits) as u8);
                buffer &= (1 << bits) - 1;
            }
        }
    }
    String::from_utf8(output).map_err(|_| ())
}
