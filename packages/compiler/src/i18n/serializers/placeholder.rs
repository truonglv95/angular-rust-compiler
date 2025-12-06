//! Placeholder Module
//!
//! Corresponds to packages/compiler/src/i18n/serializers/placeholder.ts
//! Creates unique names for placeholders with different content

use std::collections::HashMap;
use lazy_static::lazy_static;

lazy_static! {
    static ref TAG_TO_PLACEHOLDER_NAMES: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("A", "LINK");
        m.insert("B", "BOLD_TEXT");
        m.insert("BR", "LINE_BREAK");
        m.insert("EM", "EMPHASISED_TEXT");
        m.insert("H1", "HEADING_LEVEL1");
        m.insert("H2", "HEADING_LEVEL2");
        m.insert("H3", "HEADING_LEVEL3");
        m.insert("H4", "HEADING_LEVEL4");
        m.insert("H5", "HEADING_LEVEL5");
        m.insert("H6", "HEADING_LEVEL6");
        m.insert("HR", "HORIZONTAL_RULE");
        m.insert("I", "ITALIC_TEXT");
        m.insert("LI", "LIST_ITEM");
        m.insert("LINK", "MEDIA_LINK");
        m.insert("OL", "ORDERED_LIST");
        m.insert("P", "PARAGRAPH");
        m.insert("Q", "QUOTATION");
        m.insert("S", "STRIKETHROUGH_TEXT");
        m.insert("SMALL", "SMALL_TEXT");
        m.insert("SUB", "SUBSTRIPT");
        m.insert("SUP", "SUPERSCRIPT");
        m.insert("TBODY", "TABLE_BODY");
        m.insert("TD", "TABLE_CELL");
        m.insert("TFOOT", "TABLE_FOOTER");
        m.insert("TH", "TABLE_HEADER_CELL");
        m.insert("THEAD", "TABLE_HEADER");
        m.insert("TR", "TABLE_ROW");
        m.insert("TT", "MONOSPACED_TEXT");
        m.insert("U", "UNDERLINED_TEXT");
        m.insert("UL", "UNORDERED_LIST");
        m
    };
}

/// Creates unique names for placeholder with different content.
///
/// Returns the same placeholder name when the content is identical.
#[derive(Debug, Clone)]
pub struct PlaceholderRegistry {
    // Count the occurrence of the base name to generate a unique name
    place_holder_name_counts: HashMap<String, usize>,
    // Maps signature to placeholder names
    signature_to_name: HashMap<String, String>,
}

impl PlaceholderRegistry {
    pub fn new() -> Self {
        PlaceholderRegistry {
            place_holder_name_counts: HashMap::new(),
            signature_to_name: HashMap::new(),
        }
    }

    pub fn get_start_tag_placeholder_name(
        &mut self,
        tag: &str,
        attrs: &HashMap<String, String>,
        is_void: bool,
    ) -> String {
        let signature = self.hash_tag(tag, attrs, is_void);
        if let Some(name) = self.signature_to_name.get(&signature) {
            return name.clone();
        }

        let upper_tag = tag.to_uppercase();
        let base_name = TAG_TO_PLACEHOLDER_NAMES
            .get(upper_tag.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("TAG_{}", upper_tag));

        let name = if is_void {
            self.generate_unique_name(&base_name)
        } else {
            self.generate_unique_name(&format!("START_{}", base_name))
        };

        self.signature_to_name.insert(signature, name.clone());
        name
    }

    pub fn get_close_tag_placeholder_name(&mut self, tag: &str) -> String {
        let signature = self.hash_closing_tag(tag);
        if let Some(name) = self.signature_to_name.get(&signature) {
            return name.clone();
        }

        let upper_tag = tag.to_uppercase();
        let base_name = TAG_TO_PLACEHOLDER_NAMES
            .get(upper_tag.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("TAG_{}", upper_tag));

        let name = self.generate_unique_name(&format!("CLOSE_{}", base_name));

        self.signature_to_name.insert(signature, name.clone());
        name
    }

    pub fn get_placeholder_name(&mut self, name: &str, content: &str) -> String {
        let upper_name = name.to_uppercase();
        let signature = format!("PH: {}={}", upper_name, content);

        if let Some(cached_name) = self.signature_to_name.get(&signature) {
            return cached_name.clone();
        }

        let unique_name = self.generate_unique_name(&upper_name);
        self.signature_to_name.insert(signature, unique_name.clone());
        unique_name
    }

    pub fn get_unique_placeholder(&mut self, name: &str) -> String {
        self.generate_unique_name(&name.to_uppercase())
    }

    pub fn get_start_block_placeholder_name(&mut self, name: &str, parameters: &[String]) -> String {
        let signature = self.hash_block(name, parameters);
        if let Some(cached_name) = self.signature_to_name.get(&signature) {
            return cached_name.clone();
        }

        let placeholder = self.generate_unique_name(&format!("START_BLOCK_{}", self.to_snake_case(name)));
        self.signature_to_name.insert(signature, placeholder.clone());
        placeholder
    }

    pub fn get_close_block_placeholder_name(&mut self, name: &str) -> String {
        let signature = self.hash_closing_block(name);
        if let Some(cached_name) = self.signature_to_name.get(&signature) {
            return cached_name.clone();
        }

        let placeholder = self.generate_unique_name(&format!("CLOSE_BLOCK_{}", self.to_snake_case(name)));
        self.signature_to_name.insert(signature, placeholder.clone());
        placeholder
    }

    // Generate a hash for a tag - does not take attribute order into account
    fn hash_tag(&self, tag: &str, attrs: &HashMap<String, String>, is_void: bool) -> String {
        let start = format!("<{}", tag);
        let mut attr_keys: Vec<_> = attrs.keys().collect();
        attr_keys.sort();
        let str_attrs: String = attr_keys
            .iter()
            .map(|name| format!(" {}={}", name, attrs[*name]))
            .collect::<Vec<_>>()
            .join("");
        let end = if is_void {
            "/>"
        } else {
            &format!("></{}>", tag)
        };

        format!("{}{}{}", start, str_attrs, end)
    }

    fn hash_closing_tag(&self, tag: &str) -> String {
        self.hash_tag(&format!("/{}", tag), &HashMap::new(), false)
    }

    fn hash_block(&self, name: &str, parameters: &[String]) -> String {
        let mut params_sorted = parameters.to_vec();
        params_sorted.sort();
        let params = if parameters.is_empty() {
            String::new()
        } else {
            format!(" ({})", params_sorted.join("; "))
        };
        format!("@{}{} {{}}", name, params)
    }

    fn hash_closing_block(&self, name: &str) -> String {
        self.hash_block(&format!("close_{}", name), &[])
    }

    fn to_snake_case(&self, name: &str) -> String {
        name.to_uppercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect()
    }

    fn generate_unique_name(&mut self, base: &str) -> String {
        let seen = self.place_holder_name_counts.contains_key(base);
        if !seen {
            self.place_holder_name_counts.insert(base.to_string(), 1);
            return base.to_string();
        }

        let id = *self.place_holder_name_counts.get(base).unwrap();
        self.place_holder_name_counts.insert(base.to_string(), id + 1);
        format!("{}_{}", base, id)
    }
}

impl Default for PlaceholderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

