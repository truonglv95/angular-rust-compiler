//! I18n Module
//!
//! Corresponds to packages/compiler/src/i18n/
//! Internationalization support

pub mod digest;
pub mod i18n_ast;
pub mod i18n_parser;
pub mod message_bundle;
pub mod translation_bundle;
pub mod extractor_merger;
pub mod i18n_html_parser;
pub mod serializers;

// Re-export commonly used items (matching index.ts exports)
pub use digest::compute_msg_id;
pub use i18n_html_parser::I18NHtmlParser;
pub use message_bundle::MessageBundle;
pub use serializers::serializer::Serializer;
pub use serializers::{Xliff, Xliff2, Xmb, Xtb};

// Additional exports for internal use
pub use i18n_ast::{
    BlockPlaceholder, Container, I18nMeta, Icu, IcuPlaceholder, Message, MessagePlaceholder,
    MessageSpan, Node, Placeholder, TagPlaceholder, Text, Visitor,
};

pub use digest::{
    compute_decimal_digest, compute_digest, decimal_digest, digest, fingerprint, sha1,
};

pub use translation_bundle::TranslationBundle;
pub use extractor_merger::{extract_messages, merge_translations, ExtractionResult};
pub use i18n_parser::{create_i18n_message_factory, I18nMessageFactory};
pub use serializers::placeholder::PlaceholderRegistry;
pub use serializers::xml_helper::{escape_xml, serialize};

