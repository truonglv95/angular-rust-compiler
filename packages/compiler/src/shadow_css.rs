//! Shadow CSS
//!
//! Corresponds to packages/compiler/src/shadow_css.ts
//! CSS scoping for component view encapsulation
//!
//! This is a limited shim for ShadowDOM css styling.
//! https://dvcs.w3.org/hg/webcomponents/raw-file/tip/spec/shadow/index.html#styles

use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::{HashMap, HashSet};


// Constants
const POLYFILL_HOST: &str = "-shadowcsshost";
const POLYFILL_HOST_CONTEXT: &str = "-shadowcsscontext";
const POLYFILL_HOST_NO_COMBINATOR: &str = "-shadowcsshost-no-combinator";
const COMMENT_PLACEHOLDER: &str = "%COMMENT%";
const BLOCK_PLACEHOLDER: &str = "%BLOCK%";
const COMMA_IN_PLACEHOLDER: &str = "%COMMA_IN_PLACEHOLDER%";
const SEMI_IN_PLACEHOLDER: &str = "%SEMI_IN_PLACEHOLDER%";
const COLON_IN_PLACEHOLDER: &str = "%COLON_IN_PLACEHOLDER%";

// Animation keywords that should not be scoped
const ANIMATION_KEYWORDS: &[&str] = &[
    // global values
    "inherit", "initial", "revert", "unset",
    // animation-direction
    "alternate", "alternate-reverse", "normal", "reverse",
    // animation-fill-mode
    "backwards", "both", "forwards", "none",
    // animation-play-state
    "paused", "running",
    // animation-timing-function
    "ease", "ease-in", "ease-in-out", "ease-out", "linear", "step-start", "step-end",
    // steps() function
    "end", "jump-both", "jump-end", "jump-none", "jump-start", "start",
];

// Scoped at-rule identifiers
const SCOPED_AT_RULE_IDENTIFIERS: &[&str] = &[
    "@media", "@supports", "@document", "@layer", "@container", "@scope", "@starting-style",
];

// Regex patterns - using Lazy for compilation
static COMMENT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"/\*[\s\S]*?\*/").unwrap()
});

static COMMENT_WITH_HASH_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"/\*\s*#\s*source(Mapping)?URL=").unwrap()
});

static COMMENT_WITH_HASH_PLACEHOLDER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(COMMENT_PLACEHOLDER).unwrap()
});

static NEWLINES_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\r?\n").unwrap()
});

static COLON_HOST_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r":host").unwrap()
});

static COLON_HOST_CONTEXT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r":host-context").unwrap()
});

static POLYFILL_HOST_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"-shadowcsshost").unwrap()
});

static SHADOW_DEEP_SELECTORS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:>>>)|(?:\/deep\/)|(?:::ng-deep)").unwrap()
});

static CSS_COMMA_IN_PLACEHOLDER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(COMMA_IN_PLACEHOLDER).unwrap()
});

static CSS_SEMI_IN_PLACEHOLDER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(SEMI_IN_PLACEHOLDER).unwrap()
});

static CSS_COLON_IN_PLACEHOLDER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(COLON_IN_PLACEHOLDER).unwrap()
});

static RULE_RE: Lazy<Regex> = Lazy::new(|| {
    // Pattern: (\s*(?:%COMMENT%\s*)*)([^;\{\}]+?)(\s*)((?:{%BLOCK%}?\s*;?)|(?:\s*;))
    // Note: {%BLOCK%}? means the entire {%BLOCK%} is optional
    let comment_ph = regex::escape(COMMENT_PLACEHOLDER);
    let block_ph = regex::escape(BLOCK_PLACEHOLDER);
    // Escape { and } properly, and make the entire {%BLOCK%} optional with (?:...)?
    let pattern = format!(
        r"(\s*(?:{}?\s*)*)([^;{{}}]+?)(\s*)((?:\{{{}\}}?\s*;?)|(?:\s*;))",
        comment_ph, block_ph
    );
    Regex::new(&pattern).unwrap()
});

static CSS_COLON_HOST_RE: Lazy<Regex> = Lazy::new(|| {
    // Pattern: -shadowcsshost(?:\(([^)]+)\))?([^,{]*)
    let pattern = format!(r"{}(?:\(([^)]+)\))?([^,{{]*)", POLYFILL_HOST);
    Regex::new(&pattern).unwrap()
});

static CSS_COLON_HOST_CONTEXT_RE_GLOBAL: Lazy<Regex> = Lazy::new(|| {
    // Pattern: (:(where|is)\()?(-shadowcsscontext(?:\(([^)]+)\))?([^{]*))
    let css_scoped_pseudo_function_prefix = r"(:(where|is)\()?";
    // Build pattern - escape { as {{ in format string
    // Pattern: -shadowcsscontext(?:\(([^)]+)\))?([^{]*)
    // In format string: [^{] becomes [^{{{] (each { needs to be {{)
    // Use string concatenation to avoid format string escaping issues with [^{]
    let host_context_pattern = format!("{}(?:\\\\(([^)]+)\\\\))?([^", POLYFILL_HOST_CONTEXT) + "{" + "]*)";
    let pattern = format!("{}({})", css_scoped_pseudo_function_prefix, host_context_pattern);
    Regex::new(&pattern).unwrap()
});

static CSS_CONTENT_NEXT_SELECTOR_RE: Lazy<Regex> = Lazy::new(|| {
    // Note: Rust regex doesn't support backreferences, so we match both single and double quotes separately
    // Pattern: polyfill-next-selector[^}]*content:\s*?(['"])(.*?)\1[;\s]*}([^{]*?){
    // We'll match both '...' and "..." patterns
    // Use regular string instead of raw string to properly escape
    Regex::new("(?i)polyfill-next-selector[^}]*content:\\s*?((?:'(?:[^'\\\\]|\\\\.)*')|(?:\"(?:[^\"\\\\]|\\\\.)*\"))[;\\s]*}([^{]*?)\\{").unwrap()
});

static CSS_CONTENT_RULE_RE: Lazy<Regex> = Lazy::new(|| {
    // Pattern: (polyfill-rule)[^}]*(content:\s*(['"])(.*?)\3)[;\s]*[^}]*
    // Note: Rust regex doesn't support backreferences, so we match both single and double quotes separately
    // We capture: 1=polyfill-rule, 2=content:..., 3=single-quote-content, 4=double-quote-content
    Regex::new(r#"(?i)(polyfill-rule)[^}]*((?:content:\s*(?:'([^']*)'|"([^"]*)")))[;\s]*[^}]*}"#).unwrap()
});

static CSS_CONTENT_UNSCOPED_RULE_RE: Lazy<Regex> = Lazy::new(|| {
    // Pattern: (polyfill-unscoped-rule)[^}]*(content:\s*(['"])(.*?)\3)[;\s]*[^}]*
    // We capture: 1=polyfill-unscoped-rule, 2=content:..., 3=single-quote-content, 4=double-quote-content
    Regex::new(r#"(?i)(polyfill-unscoped-rule)[^}]*((?:content:\s*(?:'([^']*)'|"([^"]*)")))[;\s]*[^}]*}"#).unwrap()
});

static POLYFILL_HOST_NO_COMBINATOR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(&format!(r"{}([^\s,]*)", POLYFILL_HOST_NO_COMBINATOR)).unwrap()
});

// Helper structs
#[derive(Clone, Debug)]
pub struct CssRule {
    pub selector: String,
    pub content: String,
}

impl CssRule {
    pub fn new(selector: String, content: String) -> Self {
        CssRule { selector, content }
    }
}

struct StringWithEscapedBlocks {
    escaped_string: String,
    blocks: Vec<String>,
}

struct SafeSelector {
    placeholders: Vec<String>,
    index: usize,
    content: String,
}

impl SafeSelector {
    fn new(selector: &str) -> Self {
        let mut placeholders = Vec::new();
        let mut index = 0;
        
        // Replace attribute selectors with placeholders
        let attr_re = Regex::new(r"(\[[^\]]*\])").unwrap();
        let mut content = attr_re.replace_all(selector, |caps: &regex::Captures| {
            let keep = caps.get(0).unwrap().as_str().to_string();
            let replace_by = format!("__ph-{}__", index);
            placeholders.push(keep);
            index += 1;
            replace_by
        }).to_string();
        
        // Replace escape sequences
        let escape_re = Regex::new(r"(\\.)").unwrap();
        content = escape_re.replace_all(&content, |caps: &regex::Captures| {
            let keep = caps.get(0).unwrap().as_str().to_string();
            let replace_by = format!("__esc-ph-{}__", index);
            placeholders.push(keep);
            index += 1;
            replace_by
        }).to_string();
        
        // Replace nth-child expressions
        let nth_re = Regex::new(r"(:nth-[-\w]+)(\([^)]+\))").unwrap();
        content = nth_re.replace_all(&content, |caps: &regex::Captures| {
            let pseudo = caps.get(1).unwrap().as_str();
            let exp = caps.get(2).unwrap().as_str();
            let replace_by = format!("{}__ph-{}__", pseudo, index);
            placeholders.push(format!("({})", &exp[1..exp.len()-1]));
            index += 1;
            replace_by
        }).to_string();
        
        SafeSelector {
            placeholders,
            index,
            content,
        }
    }
    
    fn restore(&self, content: String) -> String {
        let mut result = content;
        let ph_re = Regex::new(r"__(?:ph|esc-ph)-(\d+)__").unwrap();
        result = ph_re.replace_all(&result, |caps: &regex::Captures| {
            let idx: usize = caps.get(1).unwrap().as_str().parse().unwrap();
            if idx < self.placeholders.len() {
                self.placeholders[idx].clone()
            } else {
                caps.get(0).unwrap().as_str().to_string()
            }
        }).to_string();
        result
    }
    
    fn content(&self) -> &str {
        &self.content
    }
}

// Main ShadowCss struct
pub struct ShadowCss {
    safe_selector: Option<SafeSelector>,
    should_scope_indicator: Option<bool>,
}

impl ShadowCss {
    pub fn new() -> Self {
        ShadowCss {
            safe_selector: None,
            should_scope_indicator: None,
        }
    }

    /// Shim some cssText with the given selector. Returns cssText that can be included in the document
    ///
    /// The selector is the attribute added to all elements inside the host,
    /// The hostSelector is the attribute added to the host itself.
    pub fn shim_css_text(&self, css_text: &str, selector: &str, host_selector: &str) -> String {
        // Collect comments and replace them with a placeholder
        let mut comments = Vec::new();
        let css_with_comments = COMMENT_RE.replace_all(css_text, |caps: &regex::Captures| {
            let m = caps.get(0).unwrap().as_str();
            if COMMENT_WITH_HASH_RE.is_match(m) {
                comments.push(m.to_string());
            } else {
                // Replace non hash comments with newlines
                let newlines: String = NEWLINES_RE.find_iter(m)
                    .map(|m| m.as_str())
                    .collect();
                comments.push(format!("{}\n", newlines));
            }
            COMMENT_PLACEHOLDER
        });
        
        let css_text = css_with_comments.to_string();
        let css_text = self.insert_directives(&css_text);
        let scoped_css_text = self.scope_css_text(&css_text, selector, host_selector);
        
        // Add back comments at the original position
        let mut comment_idx = 0;
        COMMENT_WITH_HASH_PLACEHOLDER_RE.replace_all(&scoped_css_text, |_caps: &regex::Captures| {
            if comment_idx < comments.len() {
                let result = comments[comment_idx].clone();
                comment_idx += 1;
                result
            } else {
                COMMENT_PLACEHOLDER.to_string()
            }
        }).to_string()
    }

    fn insert_directives(&self, css_text: &str) -> String {
        let css_text = self.insert_polyfill_directives_in_css_text(css_text);
        self.insert_polyfill_rules_in_css_text(&css_text)
    }

    fn insert_polyfill_directives_in_css_text(&self, css_text: &str) -> String {
        CSS_CONTENT_NEXT_SELECTOR_RE.replace_all(css_text, |caps: &regex::Captures| {
            let quoted_selector = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            // Extract selector from quotes
            let selector = if quoted_selector.len() >= 2 {
                &quoted_selector[1..quoted_selector.len()-1]
            } else {
                quoted_selector
            };
            format!("{}{{", selector)
        }).to_string()
    }

    fn insert_polyfill_rules_in_css_text(&self, css_text: &str) -> String {
        CSS_CONTENT_RULE_RE.replace_all(css_text, |caps: &regex::Captures| {
            let full_match = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let polyfill_rule = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let content_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let selector = caps.get(3).or_else(|| caps.get(4)).map(|m| m.as_str()).unwrap_or("");
            
            // Match TypeScript: m[0].replace(m[1], '').replace(m[2], '') then m[4] + rule
            let rule = full_match.replace(polyfill_rule, "").replace(content_part, "");
            format!("{}{}", selector, rule)
        }).to_string()
    }

    fn scope_css_text(&self, css_text: &str, scope_selector: &str, host_selector: &str) -> String {
        let unscoped_rules = self.extract_unscoped_rules_from_css_text(css_text);
        // Remove polyfill-unscoped-rule from CSS text (they've been extracted)
        let mut css_text = CSS_CONTENT_UNSCOPED_RULE_RE.replace_all(css_text, "").to_string();
        css_text = self.insert_polyfill_host_in_css_text(&css_text);
        css_text = self.convert_colon_host(&css_text);
        css_text = self.convert_colon_host_context(&css_text);
        css_text = self.convert_shadow_dom_selectors(&css_text);
        
        if !scope_selector.is_empty() {
            css_text = self.scope_keyframes_related_css(&css_text, scope_selector);
            css_text = self.scope_selectors(&css_text, scope_selector, host_selector);
        }
        
        let result = format!("{}\n{}", css_text, unscoped_rules);
        result.trim().to_string()
    }

    fn extract_unscoped_rules_from_css_text(&self, css_text: &str) -> String {
        let mut result = String::new();
        for caps in CSS_CONTENT_UNSCOPED_RULE_RE.captures_iter(css_text) {
            let full_match = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let polyfill_unscoped_rule = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let content_part = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let selector = caps.get(3).or_else(|| caps.get(4)).map(|m| m.as_str()).unwrap_or("");
            
            // Match TypeScript: m[0].replace(m[2], '').replace(m[1], m[4])
            // Replace content_part first, then replace polyfill_unscoped_rule with selector
            let rule = full_match.replace(content_part, "").replace(polyfill_unscoped_rule, selector);
            result.push_str(&format!("{}\n\n", rule));
        }
        result
    }

    fn insert_polyfill_host_in_css_text(&self, selector: &str) -> String {
        let result = COLON_HOST_CONTEXT_RE.replace_all(selector, POLYFILL_HOST_CONTEXT);
        COLON_HOST_RE.replace_all(&result, POLYFILL_HOST).to_string()
    }

    fn convert_colon_host(&self, css_text: &str) -> String {
        CSS_COLON_HOST_RE.replace_all(css_text, |caps: &regex::Captures| {
            let host_selectors = caps.get(1).map(|m| m.as_str());
            let other_selectors = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            
            if let Some(host_sel) = host_selectors {
                let mut converted = Vec::new();
                for host_selector in self.split_on_top_level_commas(host_sel, true) {
                    let trimmed = host_selector.trim();
                    if trimmed.is_empty() {
                        break;
                    }
                    let cleaned = trimmed.replace(POLYFILL_HOST, "");
                    converted.push(format!("{}{}{}", POLYFILL_HOST_NO_COMBINATOR, cleaned, other_selectors));
                }
                converted.join(",")
            } else {
                format!("{}{}", POLYFILL_HOST_NO_COMBINATOR, other_selectors)
            }
        }).to_string()
    }

    fn split_on_top_level_commas(&self, text: &str, return_on_closing_paren: bool) -> Vec<String> {
        let mut result = Vec::new();
        let mut parens = 0;
        let mut prev = 0;
        let chars: Vec<char> = text.chars().collect();
        
        for (i, &ch) in chars.iter().enumerate() {
            if ch == '(' {
                parens += 1;
            } else if ch == ')' {
                parens -= 1;
                if parens < 0 && return_on_closing_paren {
                    result.push(chars[prev..i].iter().collect());
                    return result;
                }
            } else if ch == ',' && parens == 0 {
                result.push(chars[prev..i].iter().collect());
                prev = i + 1;
            }
        }
        
        result.push(chars[prev..].iter().collect());
        result
    }

    fn convert_colon_host_context(&self, css_text: &str) -> String {
        // Splits up the selectors on their top-level commas, processes the :host-context in them
        // individually and stitches them back together
        let parts: Vec<String> = self.split_on_top_level_commas(css_text, false);
        let results: Vec<String> = parts.iter()
            .map(|part| self.convert_colon_host_context_in_selector_part(part))
            .collect();
        results.join(",")
    }

    fn convert_colon_host_context_in_selector_part(&self, css_text: &str) -> String {
        CSS_COLON_HOST_CONTEXT_RE_GLOBAL.replace_all(css_text, |caps: &regex::Captures| {
            let selector_text = caps.get(0).map(|m| m.as_str()).unwrap_or("");
            let pseudo_prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            
            let mut context_selector_groups: Vec<Vec<String>> = vec![vec![]];
            let mut selector_text = selector_text.to_string();
            
            // Loop until every :host-context in the compound selector has been processed
            while let Some(start_index) = selector_text.find(POLYFILL_HOST_CONTEXT) {
                let after_prefix = &selector_text[start_index + POLYFILL_HOST_CONTEXT.len()..];
                
                if after_prefix.is_empty() || !after_prefix.starts_with('(') {
                    // Edge case of :host-context with no parens
                    selector_text = after_prefix.to_string();
                    continue;
                }
                
                // Extract comma-separated selectors between the parentheses
                let mut new_context_selectors = Vec::new();
                let mut end_index = 0;
                for selector in self.split_on_top_level_commas(&after_prefix[1..], true) {
                    end_index += selector.len() + 1;
                    let trimmed = selector.trim();
                    if !trimmed.is_empty() {
                        new_context_selectors.push(trimmed.to_string());
                    }
                }
                
                // Duplicate the current selector group for each of these new selectors
                let context_selector_groups_length = context_selector_groups.len();
                repeat_groups(&mut context_selector_groups, new_context_selectors.len());
                for (i, new_selector) in new_context_selectors.iter().enumerate() {
                    for j in 0..context_selector_groups_length {
                        context_selector_groups[j + i * context_selector_groups_length].push(new_selector.clone());
                    }
                }
                
                // Update the selector_text
                selector_text = after_prefix[end_index + 1..].to_string();
            }
            
            // Combine context selectors
            context_selector_groups.iter()
                .map(|context_selectors| {
                    combine_host_context_selectors(context_selectors, &selector_text, pseudo_prefix)
                })
                .collect::<Vec<_>>()
                .join(", ")
        }).to_string()
    }

    fn convert_shadow_dom_selectors(&self, css_text: &str) -> String {
        let mut result = css_text.to_string();
        // Replace ::shadow and ::content with space
        // Note: >>>, /deep/, ::ng-deep are NOT replaced here - they are handled in scope_selector
        result = result.replace("::shadow", " ");
        result = result.replace("::content", " ");
        result = result.replace("/shadow-deep/", " ");
        result = result.replace("/shadow/", " ");
        result
    }

    fn scope_keyframes_related_css(&self, css_text: &str, scope_selector: &str) -> String {
        // First pass: collect all unscoped keyframe names
        // We need to process rules and collect keyframes, but process_rules uses Fn, not FnMut
        // So we'll do two passes: first to collect, second to apply
        let mut unscoped_keyframes_set = HashSet::new();
        
        // Collect keyframes by manually processing the CSS
        // Match both quoted and unquoted keyframes
        
        // First pass: scope keyframe declarations
        let scoped_keyframes_css_text = process_rules(css_text, |rule: CssRule| {
            self.scope_local_keyframe_declarations(rule, scope_selector, &mut unscoped_keyframes_set)
        });
        
        // Second pass: scope animation rules using collected keyframes
        process_rules(&scoped_keyframes_css_text, |rule: CssRule| {
            self.scope_animation_rule(rule, scope_selector, &unscoped_keyframes_set)
        })
    }

    fn scope_local_keyframe_declarations(
        &self,
        rule: CssRule,
        scope_selector: &str,
        unscoped_keyframes_set: &mut HashSet<String>,
    ) -> CssRule {
        // Pattern: (^@(?:-webkit-)?keyframes(?:\s+))(['"]?)(.+)\2(\s*)$
        // Note: Rust regex doesn't support backreferences, so we match both quoted and unquoted separately
        // For quoted: match '...' or "..."
        let keyframes_re_quoted_single = Regex::new(r"(?s)(^@(?:-webkit-)?keyframes\s+)'((?:[^'\\]|\\.)*)'(\s*)$").unwrap();
        let keyframes_re_quoted_double = Regex::new(r#"(?s)(^@(?:-webkit-)?keyframes\s+)"((?:[^"\\]|\\.)*)"(\s*)$"#).unwrap();
        let keyframes_re_unquoted = Regex::new(r"(?s)(^@(?:-webkit-)?keyframes\s+)([^\s]+)(\s*)$").unwrap();
        
        let selector = if keyframes_re_quoted_single.is_match(&rule.selector) {
            keyframes_re_quoted_single.replace(&rule.selector, |caps: &regex::Captures| {
                let start = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let keyframe_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let end_spaces = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                
                let unescaped_name = unescape_quotes(keyframe_name, true);
                unscoped_keyframes_set.insert(unescaped_name.clone());
                
                format!("{}'{}_{}'{}", start, scope_selector, keyframe_name, end_spaces)
            })
        } else if keyframes_re_quoted_double.is_match(&rule.selector) {
            keyframes_re_quoted_double.replace(&rule.selector, |caps: &regex::Captures| {
                let start = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let keyframe_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let end_spaces = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                
                let unescaped_name = unescape_quotes(keyframe_name, true);
                unscoped_keyframes_set.insert(unescaped_name.clone());
                
                format!("{}\"{}_{}\"{}", start, scope_selector, keyframe_name, end_spaces)
            })
        } else {
            keyframes_re_unquoted.replace(&rule.selector, |caps: &regex::Captures| {
                let start = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let keyframe_name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let end_spaces = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                
                unscoped_keyframes_set.insert(keyframe_name.to_string());
                format!("{}{}_{}{}", start, scope_selector, keyframe_name, end_spaces)
            })
        };
        
        CssRule::new(selector.to_string(), rule.content)
    }

    fn scope_animation_keyframe(
        &self,
        keyframe: &str,
        scope_selector: &str,
        unscoped_keyframes_set: &HashSet<String>,
    ) -> String {
        // Pattern: ^(\s*)(['"]?)(.+?)\2(\s*)$
        // Note: Rust regex doesn't support backreferences, so we match both quoted and unquoted separately
        let re_quoted_single = Regex::new(r"(?s)^(\s*)'((?:[^'\\]|\\.)*)'(\s*)$").unwrap();
        let re_quoted_double = Regex::new(r#"(?s)^(\s*)"((?:[^"\\]|\\.)*)"(\s*)$"#).unwrap();
        let re_unquoted = Regex::new(r"(?s)^(\s*)([^\s,;]+)(\s*)$").unwrap();
        
        if re_quoted_single.is_match(keyframe.trim()) {
            re_quoted_single.replace_all(keyframe, |caps: &regex::Captures| {
                let spaces1 = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let spaces2 = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                
                let unescaped_name = unescape_quotes(name, true);
                // If unscoped_keyframes_set contains the name, it's defined in component, so scope it
                // Otherwise, it's not defined in component, so don't scope
                let scoped_name = if unscoped_keyframes_set.contains(&unescaped_name) {
                    format!("{}_{}", scope_selector, name)  // Scope using original name to preserve escapes
                } else {
                    name.to_string()  // Don't scope, keep original
                };
                
                format!("{}'{}'{}", spaces1, scoped_name, spaces2)
            }).to_string()
        } else if re_quoted_double.is_match(keyframe.trim()) {
            re_quoted_double.replace_all(keyframe, |caps: &regex::Captures| {
                let spaces1 = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let spaces2 = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                let unescaped_name = unescape_quotes(name, true);
                // If unscoped_keyframes_set contains the name, it's defined in component, so scope it
                // Otherwise, it's not defined in component, so don't scope
                let scoped_name = if unscoped_keyframes_set.contains(&unescaped_name) {
                    format!("{}_{}", scope_selector, name)  // Scope using original name to preserve escapes
                } else {
                    name.to_string()  // Don't scope, keep original
                };
                
                format!("{}\"{}\"{}", spaces1, scoped_name, spaces2)
            }).to_string()
        } else {
            re_unquoted.replace_all(keyframe, |caps: &regex::Captures| {
                let spaces1 = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                let spaces2 = caps.get(3).map(|m| m.as_str()).unwrap_or("");
                
                // If unscoped_keyframes_set contains the name, it's defined in component, so scope it
                // Otherwise, it's not defined in component, so don't scope
                let scoped_name = if unscoped_keyframes_set.contains(name) {
                    format!("{}_{}", scope_selector, name)  // Scope
                } else {
                    name.to_string()  // Don't scope
                };
                
                format!("{}{}{}", spaces1, scoped_name, spaces2)
            }).to_string()
        }
    }

    fn scope_animation_rule(
        &self,
        rule: CssRule,
        scope_selector: &str,
        unscoped_keyframes_set: &HashSet<String>,
    ) -> CssRule {
        // Pattern: (^|\s+|,) (?: (?: (['"]) ((?:\\\\|\\\2|(?!\2).)+) \2) | (-?[A-Za-z][\w\-]*))
        // We use (?:[^'\\]|\\.)* to match content effectively allowing escaped characters.
        // Group 1: Leading separator
        // Group 2: Single quoted content (inner)
        // Group 3: Double quoted content (inner)
        // Group 4: Unquoted identifier
        let animation_keyframes_re = Regex::new(r"(?s)(^|[,\s]+)(?:'((?:[^'\\]|\\.)*)'|\x22((?:[^\x22\\]|\\.)*)\x22|(-?[A-Za-z][\w\-]*))").unwrap();

        // Pattern for animation property: ((?:^|\s+|;)(?:-webkit-)?animation\s*:\s*),*([^;]+)
        let animation_re = Regex::new(r"((?:^|\s+|;)(?:-webkit-)?animation\s*:\s*),*([^;]+)").unwrap();
        let mut content = animation_re.replace_all(&rule.content, |caps: &regex::Captures| {
            let start = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let animation_declarations = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            
            
            let scoped_declarations = animation_keyframes_re.replace_all(animation_declarations, |caps: &regex::Captures| {
                // Check if the match is followed by a separator or end of string.
                // This mimics the lookahead (?=[,\s]|$) in the TypeScript regex.
                // Note: We also check for ';' as it can appear at the end of animation declarations
                let match_end = caps.get(0).unwrap().end();
                let is_followed_by_separator = if match_end >= animation_declarations.len() {
                    true
                } else {
                    let next_char = animation_declarations[match_end..].chars().next().unwrap();
                    next_char == ',' || next_char == ';' || next_char.is_whitespace()
                };
                
                if !is_followed_by_separator {
                    return caps.get(0).unwrap().as_str().to_string();
                }

                let leading_spaces = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let single_quoted_content = caps.get(2).map(|m| m.as_str());
                let double_quoted_content = caps.get(3).map(|m| m.as_str());
                let unquoted_name = caps.get(4).map(|m| m.as_str());
                
                if let Some(content) = single_quoted_content {
                    let scoped = self.scope_animation_keyframe(&format!("'{}'", content), scope_selector, unscoped_keyframes_set);
                    format!("{}{}", leading_spaces, scoped)
                } else if let Some(content) = double_quoted_content {
                    let scoped = self.scope_animation_keyframe(&format!("\"{}\"", content), scope_selector, unscoped_keyframes_set);
                    format!("{}{}", leading_spaces, scoped)
                } else if let Some(name) = unquoted_name {
                    // Check if it's an animation keyword
                    if ANIMATION_KEYWORDS.contains(&name) {
                        format!("{}{}", leading_spaces, name)
                    } else {
                        let scoped = self.scope_animation_keyframe(name, scope_selector, unscoped_keyframes_set);
                        format!("{}{}", leading_spaces, scoped)
                    }
                } else {
                     format!("{}", leading_spaces)
                }
            });
            
            format!("{}{}", start, scoped_declarations)
        }).to_string();
        
        // Pattern for animation-name property: ((?:^|\s+|;)(?:-webkit-)?animation-name(?:\s*):(?:\s*))([^;]+)
        let animation_name_re = Regex::new(r"((?:^|\s+|;)(?:-webkit-)?animation-name\s*:\s*)([^;]+)").unwrap();
        content = animation_name_re.replace_all(&content, |caps: &regex::Captures| {
            let start = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let animation_names = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            
            let scoped = animation_keyframes_re.replace_all(animation_names, |caps: &regex::Captures| {
                // Check if the match is followed by a separator or end of string.
                let match_end = caps.get(0).unwrap().end();
                let is_followed_by_separator = if match_end >= animation_names.len() {
                    true
                } else {
                    let next_char = animation_names[match_end..].chars().next().unwrap();
                    next_char == ',' || next_char.is_whitespace()
                };
                
                if !is_followed_by_separator {
                    return caps.get(0).unwrap().as_str().to_string();
                }

                let leading_spaces = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let single_quoted_content = caps.get(2).map(|m| m.as_str());
                let double_quoted_content = caps.get(3).map(|m| m.as_str());
                let unquoted_name = caps.get(4).map(|m| m.as_str());
                
                if let Some(content) = single_quoted_content {
                    let scoped = self.scope_animation_keyframe(&format!("'{}'", content), scope_selector, unscoped_keyframes_set);
                    format!("{}{}", leading_spaces, scoped)
                } else if let Some(content) = double_quoted_content {
                    let scoped = self.scope_animation_keyframe(&format!("\"{}\"", content), scope_selector, unscoped_keyframes_set);
                    format!("{}{}", leading_spaces, scoped)
                } else if let Some(name) = unquoted_name {
                    // Check if it's an animation keyword
                    if ANIMATION_KEYWORDS.contains(&name) {
                        format!("{}{}", leading_spaces, name)
                    } else {
                        let scoped = self.scope_animation_keyframe(name, scope_selector, unscoped_keyframes_set);
                        format!("{}{}", leading_spaces, scoped)
                    }
                } else {
                     format!("{}", leading_spaces)
                }
            });
            
            format!("{}{}", start, scoped)
        }).to_string();
        
        CssRule::new(rule.selector, content)
    }

    fn scope_selectors(&self, css_text: &str, scope_selector: &str, host_selector: &str) -> String {
        process_rules(css_text, |rule: CssRule| {
            let mut selector = rule.selector.clone();
            let mut content = rule.content.clone();
            
            if !selector.starts_with('@') {
                selector = self.scope_selector(&selector, scope_selector, host_selector, true, false);
            } else if SCOPED_AT_RULE_IDENTIFIERS.iter().any(|at_rule| selector.starts_with(at_rule)) {
                content = self.scope_selectors(&content, scope_selector, host_selector);
            } else if selector.starts_with("@font-face") || selector.starts_with("@page") {
                content = self.strip_scoping_selectors(&content);
            }
            
            CssRule::new(selector, content)
        })
    }

    fn strip_scoping_selectors(&self, css_text: &str) -> String {
        process_rules(css_text, |rule: CssRule| {
            let mut selector = rule.selector.clone();
            selector = SHADOW_DEEP_SELECTORS_RE.replace_all(&selector, " ").to_string();
            let polyfill_host_no_combinator_re = Regex::new(&format!(r"{}", POLYFILL_HOST_NO_COMBINATOR)).unwrap();
            selector = polyfill_host_no_combinator_re.replace_all(&selector, " ").to_string();
            CssRule::new(selector, rule.content)
        })
    }

    fn scope_selector(&self, selector: &str, scope_selector: &str, host_selector: &str, is_parent_selector: bool, inhibit_scoping: bool) -> String {
        // Split selector by comma (not inside parentheses)
        let parts = self.split_on_top_level_commas(selector, false);
        
        parts.iter()
            .map(|part| {
                let trimmed_part = part;
                // Split by ::ng-deep, /deep/, >>> to separate shallow and deep parts
                let deep_parts: Vec<&str> = SHADOW_DEEP_SELECTORS_RE.split(trimmed_part).collect();
                if deep_parts.is_empty() {
                    return String::new();
                }
                let shallow_part = deep_parts[0];
                let other_parts: Vec<&str> = deep_parts[1..].iter().map(|s| *s).collect();
                
                // Scope the shallow part as a whole (apply_selector_scope will handle splitting)
                let scoped_shallow = if self.selector_needs_scoping(shallow_part, scope_selector) {
                    self.apply_selector_scope(shallow_part, scope_selector, host_selector, is_parent_selector, inhibit_scoping)
                } else {
                    shallow_part.to_string()
                };
                
                // For other parts after ::ng-deep, /deep/, >>>, don't scope them
                let mut result = vec![scoped_shallow];
                result.extend(other_parts.iter().map(|s| s.to_string()));
                
                result.join(" ")
            })
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches(|c| c == ' ' || c == '\t').to_string())  // Trim trailing spaces/tabs only
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn selector_needs_scoping(&self, selector: &str, scope_selector: &str) -> bool {
        let re = self.make_scope_matcher(scope_selector);
        !re.is_match(selector)
    }

    fn make_scope_matcher(&self, scope_selector: &str) -> Regex {
        let escaped = scope_selector.replace('[', "\\[").replace(']', "\\]");
        let pattern = format!(r"^({})([>\s~+\[.,{{:][\s\S]*)?$", escaped);
        Regex::new(&pattern).unwrap()
    }

    fn apply_selector_scope(&self, selector: &str, scope_selector: &str, host_selector: &str, _is_parent_selector: bool, inhibit_scoping: bool) -> String {

        
        // Remove [is=...] from scope_selector
        let is_re = Regex::new(r"\[is=([^\]]*)\]").unwrap();
        let scope_selector_clean = is_re.replace_all(scope_selector, |caps: &regex::Captures| {
            caps.get(1).map(|m| m.as_str()).unwrap_or("").to_string()
        }).to_string();
        let _attr_name = format!("[{}]", scope_selector_clean);
        
        // Handle polyfill host - but we need to continue processing parts after host
        // So we'll handle host replacement but continue to scope other parts
        let _polyfill_host_re = Regex::new(&format!(r"{}", POLYFILL_HOST)).unwrap();
        let _polyfill_host_no_combinator_re = Regex::new(&format!(r"{}([^\s,]*)", POLYFILL_HOST_NO_COMBINATOR)).unwrap();
        
        // Check if selector contains POLYFILL_HOST_NO_COMBINATOR or POLYFILL_HOST
        // If it does, parts before it should not be scoped (they're considered part of :host-context)
        // Note: Check POLYFILL_HOST_NO_COMBINATOR first because POLYFILL_HOST is a prefix of it
        let host_start = selector.find(POLYFILL_HOST_NO_COMBINATOR)
            .or_else(|| selector.find(POLYFILL_HOST));
        let host_end = if let Some(_start) = host_start {
            // Find the end position of the host (after the host string)
            if let Some(no_combinator_pos) = selector.find(POLYFILL_HOST_NO_COMBINATOR) {
                Some(no_combinator_pos + POLYFILL_HOST_NO_COMBINATOR.len())
            } else if let Some(host_pos_only) = selector.find(POLYFILL_HOST) {
                Some(host_pos_only + POLYFILL_HOST.len())
            } else {
                None
            }
        } else {
            None
        };
        
        // Split selector by combinators (>, +, ~, space) but not inside parentheses or brackets
        let mut parens = 0;
        let mut brackets = 0;
        let chars: Vec<char> = selector.chars().collect();
        let mut parts_to_scope: Vec<(usize, usize)> = Vec::new(); // (start, end) indices
        let mut separators: Vec<(usize, usize, char)> = Vec::new(); // (pos, len, char) for combinators
        let mut start = 0;
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];
            
            if ch == '\\' {
                let mut is_hex = false;
                if i + 1 < chars.len() {
                    if chars[i+1].is_ascii_hexdigit() {
                        is_hex = true;
                    }
                }
                
                if is_hex {
                    // Hex escape: consume up to 6 hex digits
                    i += 1; // skip backslash
                    let mut consumed = 0;
                    while i < chars.len() && chars[i].is_ascii_hexdigit() && consumed < 6 {
                        i += 1;
                        consumed += 1;
                    }
                    // Optional whitespace: Only consume if followed by a hex digit?
                    // Angular parity: If followed by hex digit, consume space (merge).
                    // If followed by non-hex, keep space (terminator acts as combinator).
                    if i < chars.len() && chars[i] == ' ' {
                        let next_is_hex = if i + 1 < chars.len() {
                            chars[i+1].is_ascii_hexdigit()
                        } else {
                            false
                        };
                        
                        if next_is_hex {
                            i += 1;
                        }
                    }
                } else {
                    // Skip next character as it is escaped
                    i += 2;
                }
                continue;
            }

            if ch == '(' {
                parens += 1;
            } else if ch == ')' {
                if parens > 0 { parens -= 1; }
            } else if ch == '[' {
                brackets += 1;
            } else if ch == ']' {
                if brackets > 0 { brackets -= 1; }
            } else if parens == 0 && brackets == 0 && (ch == '>' || ch == '+' || ch == '~' || ch.is_whitespace()) {
                if i > start {
                    parts_to_scope.push((start, i));
                }
                
                if ch.is_whitespace() {
                    separators.push((i, 1, ch));
                } else {
                    separators.push((i, 1, ch));
                }
                
                start = i + 1;
            }
            i += 1;
        }
        
        if start < chars.len() {
            parts_to_scope.push((start, chars.len()));
        }
        
        // Scope each part and reconstruct with separators
        let mut result = String::new();
        let mut current_pos = 0;
        
        for (_idx, (part_start, part_end)) in parts_to_scope.iter().enumerate() {
            // Append separators/text before this part
            let text_before = &selector[current_pos..*part_start];
            // Normalize combinator spacing - trim and check if it's a combinator
            let trimmed = text_before.trim();
            if trimmed == ">" || trimmed == "+" || trimmed == "~" {
                result.push(' ');
                result.push_str(trimmed);
                result.push(' ');
            } else {
                result.push_str(text_before);
            }
            
            // Check if this part should be scoped
            // If has_host, only scope parts that contain host or start at or after host END position
            // Parts BEFORE host should not be scoped (they're considered part of :host-context)
            let part_str = &selector[*part_start..*part_end];
            let should_scope_this_part = if inhibit_scoping {
                // If scoping is inhibited by parent context, only scope if this part contains host
                part_str.contains(POLYFILL_HOST_NO_COMBINATOR) 
                    || part_str.contains(POLYFILL_HOST)
                    || part_str.contains(&format!("[{}]", host_selector))
            } else if let Some(host_end_pos) = host_end {
                // Scope if: part contains host, OR part starts at or after host END position
                // This ensures parts after host are scoped, but parts before are not
                // Note: [hostSelector] (e.g., [a-host]) should also be considered as host
                let contains_host = part_str.contains(POLYFILL_HOST_NO_COMBINATOR) 
                    || part_str.contains(POLYFILL_HOST)
                    || part_str.contains(&format!("[{}]", host_selector));
                let after_host = *part_start >= host_end_pos;
                contains_host || after_host
            } else {
                // If no host found, check if part contains [hostSelector]
                // If it does, don't scope it (it's a context selector from :host-context)
                let host_attr_in_part = format!("[{}]", host_selector);
                !part_str.contains(&host_attr_in_part)
            };
            
            // Scope this part
            let part = &selector[*part_start..*part_end];
            let scoped_part = self.scope_pseudo_function_aware_selector_part(
                part, &scope_selector_clean, host_selector, should_scope_this_part
            );
            result.push_str(&scoped_part);
            
            current_pos = *part_end;
        }
        
        // Append remaining separators/text
        if current_pos < selector.len() {
            result.push_str(&selector[current_pos..]);
        }
        
        
        result
    }
    
    fn scope_pseudo_function_aware_selector_part(&self, part: &str, scope_selector: &str, host_selector: &str, should_scope: bool) -> String {
        let attr_name = format!("[{}]", scope_selector);
        let pseudo_re = Regex::new(r"^:(where|is)\(").unwrap();
        let mut parts = Vec::new();
        let mut idx = 0;
        let mut matches_full_string = true;
        
        // Attempt to parse sequence of :where(...) or :is(...)
        while idx < part.len() {
            let remaining = &part[idx..];
            if let Some(mat) = pseudo_re.find(remaining) {
                // Found match at start of remaining
                let mut parens = 1; // We matched '(' in regex
                let mut match_len = mat.end(); // Length so far
                let mut found_end = false;
                
                // Scan for matching closing paren
                for ch in remaining[mat.end()..].chars() {
                    match_len += ch.len_utf8();
                    if ch == '(' {
                        parens += 1;
                    } else if ch == ')' {
                        if parens > 0 { parens -= 1; }
                        if parens == 0 {
                            found_end = true;
                            break;
                        }
                    }
                }
                
                if found_end {
                    parts.push(remaining[0..match_len].to_string());
                    idx += match_len;
                } else {
                    matches_full_string = false;
                    break;
                }
            } else {
                matches_full_string = false;
                break;
            }
        }
        
        if matches_full_string && !parts.is_empty() {
             let mut result = String::new();
             
             // Find the index of the first part containing :host
             let first_host_index = parts.iter().position(|p| {
                 let captures = pseudo_re.captures(p);
                 if let Some(caps) = captures {
                     let prefix_len = caps.get(0).unwrap().end();
                     let content = &p[prefix_len..p.len()-1];
                     content.contains(POLYFILL_HOST_NO_COMBINATOR) 
                         || content.contains(POLYFILL_HOST)
                         || content.contains(":host")
                 } else {
                     false
                 }
             });
             
             for (i, p) in parts.iter().enumerate() {
                 let captures = pseudo_re.captures(&p).unwrap();
                 let func_name = captures.get(1).unwrap().as_str();
                 // prefix is :func_name( - length is captures.get(0).len()
                 let prefix_len = captures.get(0).unwrap().end();
                 let content = &p[prefix_len..p.len()-1]; // Content inside parens
                 
                 // Determine if this specific part should be scoped
                 // If there's a host in the chain, only scope parts at or after it
                 let should_scope_inner = match first_host_index {
                     Some(host_idx) => i >= host_idx,
                     None => should_scope,  // No host in chain, use parent's decision
                 };
                 
                 // Recurse with appropriate inhibit flag
                 let scoped_content = self.scope_selector(content, scope_selector, host_selector, false, !should_scope_inner);
                 result.push_str(&format!(":{}({})", func_name, scoped_content));
             }
             return result;
        }
        
        // Fallback to simple scoping
        if should_scope {
            let scoped = self.scope_selector_part(part, scope_selector, host_selector);
            // If original part started with : and scoped result still starts with :,
            // we need to prepend the attribute (scope_simple doesn't add prefix)
            // UNLESS it's a :host-context result where [hosta] is DIRECTLY after the pseudo-selector
            let _host_marker = format!("[{}]", host_selector);
            let is_host_context_result = scoped.contains(&format!(")[{}]", host_selector)) || 
                                         scoped.contains(&format!(") [{}]", host_selector));
            if part.starts_with(':') && scoped.starts_with(':') && !is_host_context_result {
                format!("{}{}", attr_name, scoped)
            } else {
                scoped
            }
        } else {
            part.to_string()
        }
    }

    fn scope_selector_part(&self, part: &str, scope_selector: &str, host_selector: &str) -> String {
        let attr_name = format!("[{}]", scope_selector);
        if part.contains(POLYFILL_HOST_NO_COMBINATOR) {
            self.scope_simple_selector_part(part, scope_selector, host_selector)
        } else {
            self.scope_single_selector_part(part, &attr_name, host_selector)
        }
    }

    fn scope_simple_selector_part(&self, part: &str, scope_selector: &str, host_selector: &str) -> String {
        let polyfill_host_re = Regex::new(&format!(r"{}", POLYFILL_HOST)).unwrap();
        let attr_name = format!("[{}]", scope_selector);
        // Check if part contains POLYFILL_HOST (which includes POLYFILL_HOST_NO_COMBINATOR)
        if part.contains(POLYFILL_HOST) {
            let replace_by = format!("[{}]", host_selector);
            let mut result = part.to_string();
            let polyfill_host_no_combinator_re = Regex::new(&format!(r"{}([^\s,]*)", POLYFILL_HOST_NO_COMBINATOR)).unwrap();
            while polyfill_host_no_combinator_re.is_match(&result) {
                result = polyfill_host_no_combinator_re.replace_all(&result, |caps: &regex::Captures| {
                    let sel = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    // Apply pattern /([^:\)]*)(:*)(.*)/ to insert replaceBy before colon
                    let pseudo_match = Regex::new(r"([^:\)]*)(:*)(.*)").unwrap();
                    if let Some(pseudo_caps) = pseudo_match.captures(sel) {
                        let before = pseudo_caps.get(1).map(|m| m.as_str()).unwrap_or("");
                        let colon = pseudo_caps.get(2).map(|m| m.as_str()).unwrap_or("");
                        let after = pseudo_caps.get(3).map(|m| m.as_str()).unwrap_or("");
                        format!("{}{}{}{}", before, &replace_by, colon, after)
                    } else {
                        format!("{}{}", sel, &replace_by)
                    }
                }).to_string();
            }
            // Also replace POLYFILL_HOST if present
            result = polyfill_host_re.replace_all(&result, &replace_by).to_string();
            
            // Check if we need to add attrName
            let needs_attr_name = if let Some(pos) = part.find(POLYFILL_HOST_NO_COMBINATOR) {
                let after_pos = pos + POLYFILL_HOST_NO_COMBINATOR.len();
                let after_str = &part[after_pos..];
                if let Some(open_paren_pos) = after_str.find('(') {
                    let before_open = &after_str[..open_paren_pos];
                    if !before_open.contains(')') {
                         let trimmed_before = before_open.trim();
                         trimmed_before.is_empty()
                    } else {
                        false
                    }
                } else {
                    let before_str = &part[..pos];
                    if let Some(before_open_pos) = before_str.rfind('(') {
                        let between = &before_str[before_open_pos + 1..];
                        !between.contains(')')
                    } else {
                        false
                    }
                }
            } else {
                false
            };
            
            if needs_attr_name {
                let pseudo_match = Regex::new(r"([^:]*)(:*)([\s\S]*)").unwrap();
                if let Some(pseudo_caps) = pseudo_match.captures(&result) {
                    let before = pseudo_caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let colon = pseudo_caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    let after = pseudo_caps.get(3).map(|m| m.as_str()).unwrap_or("");
                    result = format!("{}{}{}{}", before, &attr_name, colon, after);
                }
            }
            result
        } else {
            format!("{} {}", scope_selector, part)
        }
    }

    fn scope_single_selector_part(&self, part: &str, attr_name: &str, host_selector: &str) -> String {
        // Remove :host if present
        let polyfill_host_re = Regex::new(&format!(r"{}", POLYFILL_HOST)).unwrap();
        let t = polyfill_host_re.replace_all(part, "").to_string();
        
        if t.is_empty() {
            return part.to_string();
        }
        
        // Check if part contains [hostSelector] (e.g., [a-host])
        // If it does, we should not scope the part before [hostSelector]
        // because it's a context selector (from :host-context)
        let host_attr_in_part = format!("[{}]", host_selector);
        if t.contains(&host_attr_in_part) {
            // Part contains [hostSelector], don't scope it
            // This is a context selector from :host-context
            return t;
        }
        
        // Match pattern: ([^:]*)(:*)(.*) to add [scopeSelector] before pseudo-classes
        // Update: Handle escaped colons by allowing escaped characters in the first group
        let pseudo_match = Regex::new(r"(?s)((?:[^:\\]|\\.)*)(:*)(.*)").unwrap();
        if let Some(caps) = pseudo_match.captures(&t) {
            let before = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let colon = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let after = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            format!("{}{}{}{}", before.trim_end(), attr_name, colon, after)
        } else {
            format!("{}{}", t, attr_name)
        }
    }
}

impl Default for ShadowCss {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions
fn escape_in_strings(input: &str) -> String {
    let mut result = input.to_string();
    let mut current_quote_char: Option<char> = None;
    let mut i = 0;
    
    while i < result.len() {
        let char = result.chars().nth(i).unwrap();
        if char == '\\' {
            i += 2;
            continue;
        }
        
        if let Some(quote) = current_quote_char {
            if char == quote {
                current_quote_char = None;
            } else {
                let placeholder = match char {
                    ';' => Some(SEMI_IN_PLACEHOLDER),
                    ',' => Some(COMMA_IN_PLACEHOLDER),
                    ':' => Some(COLON_IN_PLACEHOLDER),
                    _ => None,
                };
                if let Some(ph) = placeholder {
                    result.replace_range(i..i+1, ph);
                    i += ph.len();
                    continue;
                }
            }
        } else if char == '\'' || char == '"' {
            current_quote_char = Some(char);
        }
        i += 1;
    }
    
    result
}

fn unescape_in_strings(input: &str) -> String {
    let mut result = input.to_string();
    result = CSS_COMMA_IN_PLACEHOLDER_RE.replace_all(&result, ",").to_string();
    result = CSS_SEMI_IN_PLACEHOLDER_RE.replace_all(&result, ";").to_string();
    result = CSS_COLON_IN_PLACEHOLDER_RE.replace_all(&result, ":").to_string();
    result
}

fn escape_blocks(
    input: &str,
    char_pairs: &HashMap<char, char>,
    placeholder: &str,
) -> StringWithEscapedBlocks {
    let mut result_parts = Vec::new();
    let mut escaped_blocks = Vec::new();
    let mut open_char_count = 0;
    let mut non_block_start_index = 0;
    let mut block_start_index: Option<usize> = None;
    let mut open_char: Option<char> = None;
    let mut close_char: Option<char> = None;
    
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        let char = chars[i];
        if char == '\\' {
            i += 2;
            continue;
        }
        
        if let Some(close) = close_char {
            if char == close {
                open_char_count -= 1;
                if open_char_count == 0 {
                    if let Some(start) = block_start_index {
                        escaped_blocks.push(chars[start..i].iter().collect());
                        result_parts.push(placeholder.to_string());
                        non_block_start_index = i;
                        block_start_index = None;
                        open_char = None;
                        close_char = None;
                    }
                }
            } else if char == open_char.unwrap() {
                open_char_count += 1;
            }
        } else if open_char_count == 0 {
            if let Some(&close) = char_pairs.get(&char) {
                open_char = Some(char);
                close_char = Some(close);
                open_char_count = 1;
                block_start_index = Some(i + 1);
                result_parts.push(chars[non_block_start_index..=i].iter().collect());
            }
        }
        i += 1;
    }
    
    if let Some(start) = block_start_index {
        escaped_blocks.push(chars[start..].iter().collect());
        result_parts.push(placeholder.to_string());
    } else {
        result_parts.push(chars[non_block_start_index..].iter().collect());
    }
    
    StringWithEscapedBlocks {
        escaped_string: result_parts.join(""),
        blocks: escaped_blocks,
    }
}

fn unescape_quotes(str: &str, is_quoted: bool) -> String {
    if !is_quoted {
        return str.to_string();
    }
    // Unescape quotes: remove backslashes before any character (matching TS behavior)
    // Pattern: \\(.)
    let re = Regex::new(r"\\(.)").unwrap();
    re.replace_all(str, "$1").into_owned()
}

/// Process CSS rules by applying a callback to each rule
pub fn process_rules<F>(input: &str, mut rule_callback: F) -> String
where
    F: FnMut(CssRule) -> CssRule,
{
    let escaped = escape_in_strings(input);
    let mut char_pairs = HashMap::new();
    char_pairs.insert('{', '}');
    let input_with_escaped_blocks = escape_blocks(&escaped, &char_pairs, BLOCK_PLACEHOLDER);
    
    let mut next_block_index = 0;
    let escaped_result = RULE_RE.replace_all(&input_with_escaped_blocks.escaped_string, |caps: &regex::Captures| {
        let prefix = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let selector = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let middle = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        let suffix = caps.get(4).map(|m| m.as_str()).unwrap_or("");
        
        let mut content = String::new();
        let mut content_prefix = "";
        let mut final_suffix = suffix;
        
        if suffix.starts_with(&format!("{{{}}}", BLOCK_PLACEHOLDER)) {
            if next_block_index < input_with_escaped_blocks.blocks.len() {
                content = input_with_escaped_blocks.blocks[next_block_index].clone();
                next_block_index += 1;
            }
            final_suffix = &suffix[BLOCK_PLACEHOLDER.len() + 1..];
            content_prefix = "{";
        }
        
        let rule = rule_callback(CssRule::new(selector.to_string(), content));
        format!("{}{}{}{}{}{}", prefix, rule.selector, middle, content_prefix, rule.content, final_suffix)
    });
    
    unescape_in_strings(&escaped_result)
}

/// Combine the contextSelectors with the hostMarker and the otherSelectors
/// to create a selector that matches the same as :host-context()
fn combine_host_context_selectors(
    context_selectors: &[String],
    other_selectors: &str,
    pseudo_prefix: &str,
) -> String {
    let host_marker = POLYFILL_HOST_NO_COMBINATOR;
    let other_selectors_has_host = POLYFILL_HOST_RE.is_match(other_selectors);
    
    // If there are no context selectors then just output a host marker
    if context_selectors.is_empty() {
        return format!("{}{}", host_marker, other_selectors);
    }
    
    let mut combined = vec![context_selectors[context_selectors.len() - 1].clone()];
    for i in (0..context_selectors.len() - 1).rev() {
        let context_selector = context_selectors[i].clone();
        let length = combined.len();
        
        // Expand the combined array to accommodate new selectors
        // We need: length (same element) + length (ancestor) + length (descendant) = length * 3
        // Initialize with empty strings
        let mut new_combined = vec![String::new(); length * 3];
        
        for j in 0..length {
            let previous_selectors = combined[j].clone();
            // Add the new selector to act on the same element as the previous selectors
            new_combined[j] = format!("{}{}", context_selector, previous_selectors);
            // Add the new selector as an ancestor of the previous selectors
            new_combined[length + j] = format!("{} {}", context_selector, previous_selectors);
            // Add the new selector as a descendant of the previous selectors
            new_combined[length * 2 + j] = format!("{} {}", previous_selectors, context_selector);
        }
        
        combined = new_combined;
    }
    
    // Finally connect the selector to the hostMarkers
    combined.iter()
        .map(|s| {
            if other_selectors_has_host {
                format!("{}{}{}", pseudo_prefix, s, other_selectors)
            } else {
                format!("{}{}{}{}, {}{} {}{}", 
                    pseudo_prefix, s, host_marker, other_selectors,
                    pseudo_prefix, s, host_marker, other_selectors)
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// Mutate the given `groups` array so that there are `multiples` clones of the original array
pub fn repeat_groups(groups: &mut Vec<Vec<String>>, multiples: usize) {
    let length = groups.len();
    // Clone the original groups first
    let original_groups: Vec<Vec<String>> = groups[0..length].iter().map(|g| g.clone()).collect();
    
    for _i in 1..multiples {
        for j in 0..length {
            groups.push(original_groups[j].clone());
        }
    }
}
