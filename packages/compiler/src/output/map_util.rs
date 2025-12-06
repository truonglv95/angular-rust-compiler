//! Map Utility Module
//!
//! Corresponds to packages/compiler/src/output/map_util.ts
//! Helper utilities for creating map literals

use crate::output::output_ast as o;

/// Map entry structure
#[derive(Debug, Clone)]
pub struct MapEntry {
    pub key: String,
    pub quoted: bool,
    pub value: Box<o::Expression>,
}

/// Type alias for map literal (array of map entries)
pub type MapLiteral = Vec<MapEntry>;

/// Creates a map entry with the given key and value
pub fn map_entry(key: impl Into<String>, value: Box<o::Expression>) -> MapEntry {
    MapEntry {
        key: key.into(),
        value,
        quoted: false,
    }
}

/// Creates a literal map expression from an object-like structure
pub fn map_literal(
    obj: std::collections::HashMap<String, Box<o::Expression>>,
    quoted: bool,
) -> Box<o::Expression> {
    let entries: Vec<o::LiteralMapEntry> = obj
        .into_iter()
        .map(|(key, value)| o::LiteralMapEntry {
            key,
            quoted,
            value,
        })
        .collect();

    o::literal_map(entries)
}





