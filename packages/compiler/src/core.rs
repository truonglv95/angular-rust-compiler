//! Core Types
//!
//! Corresponds to packages/compiler/src/core.ts (330 lines)
//! Duplicates types from @angular/core to avoid circular dependency

use serde::{Deserialize, Serialize};

// Default value for emitDistinctChangesOnly
pub const EMIT_DISTINCT_CHANGES_ONLY_DEFAULT_VALUE: bool = true;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ViewEncapsulation {
    Emulated = 0,
    // Historically the 1 value was for `Native` encapsulation (removed in v11)
    None = 2,
    ShadowDom = 3,
    IsolatedShadowDom = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ChangeDetectionStrategy {
    OnPush = 0,
    Default = 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    pub alias: Option<String>,
    pub required: Option<bool>,
    pub is_signal: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputFlags {
    None = 0,
    SignalBased = 1 << 0,
    HasDecoratorInputTransform = 1 << 1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostBinding {
    pub host_property_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostListener {
    pub event_name: Option<String>,
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaMetadata {
    pub name: String,
}

// Note: In TypeScript these are const objects, in Rust we use functions
pub fn custom_elements_schema() -> SchemaMetadata {
    SchemaMetadata {
        name: "custom-elements".to_string(),
    }
}

#[allow(dead_code)]
pub fn no_errors_schema() -> SchemaMetadata {
    SchemaMetadata {
        name: "no-errors-schema".to_string(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
#[repr(u8)]
pub enum SecurityContext {
    NONE = 0,
    HTML = 1,
    STYLE = 2,
    SCRIPT = 3,
    URL = 4,
    RESOURCE_URL = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InjectFlags {
    Default = 0,
    Host = 1 << 0,
    Self_ = 1 << 1,
    SkipSelf = 1 << 2,
    Optional = 1 << 3,
    ForPipe = 1 << 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MissingTranslationStrategy {
    Error = 0,
    Warning = 1,
    Ignore = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectorFlags {
    NOT = 0b0001,
    ATTRIBUTE = 0b0010,
    ELEMENT = 0b0100,
    CLASS = 0b1000,
}

// R3 CSS Selector types
pub type R3CssSelector = Vec<String>;  // Simplified - actual implementation more complex
pub type R3CssSelectorList = Vec<R3CssSelector>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RenderFlags {
    Create = 0b01,
    Update = 0b10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AttributeMarker {
    NamespaceURI = 0,
    Classes = 1,
    Styles = 2,
    Bindings = 3,
    Template = 4,
    ProjectAs = 5,
    I18n = 6,
}
