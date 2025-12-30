//! ML Parser Tokens
//!
//! Corresponds to packages/compiler/src/ml_parser/tokens.ts (313 lines)

use crate::parse_util::ParseSourceSpan;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Token types for HTML/XML parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TokenType {
    TagOpenStart,
    TagOpenEnd,
    TagOpenEndVoid,
    TagClose,
    IncompleteTagOpen,
    Text,
    EscapableRawText,
    RawText,
    Interpolation,
    EncodedEntity,
    CommentStart,
    CommentEnd,
    CdataStart,
    CdataEnd,
    AttrName,
    AttrQuote,
    AttrValueText,
    AttrValueInterpolation,
    DocType,
    ExpansionFormStart,
    ExpansionCaseValue,
    ExpansionCaseExpStart,
    ExpansionCaseExpEnd,
    ExpansionFormEnd,
    BlockOpenStart,
    BlockOpenEnd,
    BlockClose,
    BlockParameter,
    IncompleteBlockOpen,
    LetStart,
    LetValue,
    LetEnd,
    IncompleteLet,
    ComponentOpenStart,
    ComponentOpenEnd,
    ComponentOpenEndVoid,
    ComponentClose,
    IncompleteComponentOpen,
    DirectiveName,
    DirectiveOpen,
    DirectiveClose,
    Eof,
}

/// Base token structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBase {
    pub token_type: TokenType,
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

/// All token variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Token {
    TagOpenStart(TagOpenStartToken),
    TagOpenEnd(TagOpenEndToken),
    TagOpenEndVoid(TagOpenEndVoidToken),
    TagClose(TagCloseToken),
    IncompleteTagOpen(IncompleteTagOpenToken),
    Text(TextToken),
    Interpolation(InterpolationToken),
    EncodedEntity(EncodedEntityToken),
    CommentStart(CommentStartToken),
    CommentEnd(CommentEndToken),
    CdataStart(CdataStartToken),
    CdataEnd(CdataEndToken),
    AttrName(AttributeNameToken),
    AttrQuote(AttributeQuoteToken),
    AttrValueText(AttributeValueTextToken),
    AttrValueInterpolation(AttributeValueInterpolationToken),
    DocType(DocTypeToken),
    ExpansionFormStart(ExpansionFormStartToken),
    ExpansionCaseValue(ExpansionCaseValueToken),
    ExpansionCaseExpStart(ExpansionCaseExpressionStartToken),
    ExpansionCaseExpEnd(ExpansionCaseExpressionEndToken),
    ExpansionFormEnd(ExpansionFormEndToken),
    Eof(EndOfFileToken),
    BlockParameter(BlockParameterToken),
    BlockOpenStart(BlockOpenStartToken),
    BlockOpenEnd(BlockOpenEndToken),
    BlockClose(BlockCloseToken),
    IncompleteBlockOpen(IncompleteBlockOpenToken),
    LetStart(LetStartToken),
    LetValue(LetValueToken),
    LetEnd(LetEndToken),
    IncompleteLet(IncompleteLetToken),
    ComponentOpenStart(ComponentOpenStartToken),
    ComponentOpenEnd(ComponentOpenEndToken),
    ComponentOpenEndVoid(ComponentOpenEndVoidToken),
    ComponentClose(ComponentCloseToken),
    IncompleteComponentOpen(IncompleteComponentOpenToken),
    DirectiveName(DirectiveNameToken),
    DirectiveOpen(DirectiveOpenToken),
    DirectiveClose(DirectiveCloseToken),
    RawText(RawTextToken),
    EscapableRawText(EscapableRawTextToken),
}

// Token type definitions

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagOpenStartToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagOpenEndToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagOpenEndVoidToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCloseToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteTagOpenToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpolationToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedEntityToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentStartToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentEndToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdataStartToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdataEndToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeNameToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeQuoteToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeValueTextToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeValueInterpolationToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocTypeToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionFormStartToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionCaseValueToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionCaseExpressionStartToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionCaseExpressionEndToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpansionFormEndToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndOfFileToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockParameterToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockOpenStartToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockOpenEndToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockCloseToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteBlockOpenToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetStartToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetValueToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LetEndToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteLetToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawTextToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscapableRawTextToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOpenStartToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOpenEndToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentOpenEndVoidToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentCloseToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteComponentOpenToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveNameToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveOpenToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectiveCloseToken {
    pub parts: Vec<Arc<str>>,
    pub source_span: ParseSourceSpan,
}

/// Type alias for interpolated text tokens
pub type InterpolatedTextToken = Token; // TextToken | InterpolationToken | EncodedEntityToken

/// Type alias for interpolated attribute tokens
pub type InterpolatedAttributeToken = Token; // AttributeValueTextToken | AttributeValueInterpolationToken | EncodedEntityToken
