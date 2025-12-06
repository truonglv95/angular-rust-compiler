//! I18n HTML Parser Module
//!
//! Corresponds to packages/compiler/src/i18n/i18n_html_parser.ts
//! HTML parser with i18n support

use crate::core::MissingTranslationStrategy;
use crate::ml_parser::html_parser::HtmlParser;
use crate::ml_parser::lexer::TokenizeOptions;
use crate::ml_parser::parser::ParseTreeResult;
use crate::i18n::digest::digest;
use crate::i18n::extractor_merger::merge_translations;
use crate::i18n::serializers::serializer::Serializer;
use crate::i18n::serializers::xliff::Xliff;
use crate::i18n::serializers::xliff2::Xliff2;
use crate::i18n::serializers::xmb::Xmb;
use crate::i18n::serializers::xtb::Xtb;
use crate::i18n::translation_bundle::TranslationBundle;
use std::collections::HashMap;
use std::rc::Rc;

/// HTML parser with internationalization support
pub struct I18NHtmlParser {
    html_parser: HtmlParser,
    translation_bundle: TranslationBundle,
}

impl I18NHtmlParser {
    pub fn new(
        html_parser: HtmlParser,
        translations: Option<String>,
        translations_format: Option<String>,
        missing_translation: MissingTranslationStrategy,
    ) -> Self {
        let translation_bundle = if let Some(trans) = translations {
            let serializer = create_serializer(translations_format.as_deref());
            TranslationBundle::load(
                &trans,
                "i18n",
                serializer.as_ref(),
                missing_translation,
            )
            .unwrap_or_else(|_| {
                TranslationBundle::new(
                    HashMap::new(),
                    None,
                    Rc::new(digest),
                    None,
                    missing_translation,
                )
            })
        } else {
            TranslationBundle::new(
                HashMap::new(),
                None,
                Rc::new(digest),
                None,
                missing_translation,
            )
        };

        I18NHtmlParser {
            html_parser,
            translation_bundle,
        }
    }

    pub fn parse(
        &mut self,
        source: &str,
        url: &str,
        options: TokenizeOptions,
    ) -> ParseTreeResult {
        let parse_result = self.html_parser.parse(source, url, Some(options));

        if !parse_result.errors.is_empty() {
            return ParseTreeResult {
                root_nodes: parse_result.root_nodes,
                errors: parse_result.errors,
            };
        }

        merge_translations(
            &parse_result.root_nodes,
            &mut self.translation_bundle,
            &[],
            &HashMap::new(),
        )
    }

    pub fn get_tag_definition(&self, tag_name: &str) -> Option<()> {
        // TODO: Implement tag definition lookup
        None
    }
}

fn create_serializer(format: Option<&str>) -> Box<dyn Serializer> {
    let format = format.unwrap_or("xlf").to_lowercase();

    match format.as_str() {
        "xmb" => Box::new(Xmb::new()),
        "xtb" => Box::new(Xtb::new()),
        "xliff2" | "xlf2" => Box::new(Xliff2::new()),
        "xliff" | "xlf" | _ => Box::new(Xliff::new()),
    }
}

