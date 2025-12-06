//! Serializers Module
//!
//! Corresponds to packages/compiler/src/i18n/serializers/
//! Contains various i18n serialization formats

pub mod placeholder;
pub mod serializer;
pub mod xml_helper;
pub mod xliff;
pub mod xliff2;
pub mod xmb;
pub mod xtb;

// Re-export commonly used items
pub use placeholder::PlaceholderRegistry;
pub use serializer::{PlaceholderMapper, Serializer, SimplePlaceholderMapper};
pub use xml_helper::{serialize, Declaration, Doctype, Tag, Text, Node, CR};
pub use xliff::Xliff;
pub use xliff2::Xliff2;
pub use xmb::Xmb;
pub use xtb::Xtb;

