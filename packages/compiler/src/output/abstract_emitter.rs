//! Abstract Emitter Module
//!
//! Corresponds to packages/compiler/src/output/abstract_emitter.ts
//! Base emitter functionality for code generation

use crate::output::output_ast as o;
use crate::output::source_map::SourceMapGenerator;
use crate::parse_util::ParseSourceSpan;
use std::collections::HashMap;

const SINGLE_QUOTE_ESCAPE_STRING_RE: &str = r"'|\\|\n|\r|\$";
const LEGAL_IDENTIFIER_RE: &str = r"^[$A-Z_][0-9A-Z_$]*$";
const INDENT_WITH: &str = "  ";

#[derive(Debug, Clone)]
struct EmittedLine {
    parts_length: usize,
    parts: Vec<String>,
    src_spans: Vec<Option<ParseSourceSpan>>,
    indent: usize,
}

impl EmittedLine {
    fn new(indent: usize) -> Self {
        EmittedLine {
            parts_length: 0,
            parts: Vec::new(),
            src_spans: Vec::new(),
            indent,
        }
    }
}

lazy_static::lazy_static! {
    static ref BINARY_OPERATORS: HashMap<o::BinaryOperator, &'static str> = {
        let mut m = HashMap::new();
        m.insert(o::BinaryOperator::And, "&&");
        m.insert(o::BinaryOperator::Bigger, ">");
        m.insert(o::BinaryOperator::BiggerEquals, ">=");
        m.insert(o::BinaryOperator::BitwiseOr, "|");
        m.insert(o::BinaryOperator::BitwiseAnd, "&");
        m.insert(o::BinaryOperator::Divide, "/");
        m.insert(o::BinaryOperator::Assign, "=");
        m.insert(o::BinaryOperator::Equals, "==");
        m.insert(o::BinaryOperator::Identical, "===");
        m.insert(o::BinaryOperator::Lower, "<");
        m.insert(o::BinaryOperator::LowerEquals, "<=");
        m.insert(o::BinaryOperator::Minus, "-");
        m.insert(o::BinaryOperator::Modulo, "%");
        m.insert(o::BinaryOperator::Exponentiation, "**");
        m.insert(o::BinaryOperator::Multiply, "*");
        m.insert(o::BinaryOperator::NotEquals, "!=");
        m.insert(o::BinaryOperator::NotIdentical, "!==");
        m.insert(o::BinaryOperator::NullishCoalesce, "??");
        m.insert(o::BinaryOperator::Or, "||");
        m.insert(o::BinaryOperator::Plus, "+");
        m.insert(o::BinaryOperator::In, "in");
        m.insert(o::BinaryOperator::AdditionAssignment, "+=");
        m.insert(o::BinaryOperator::SubtractionAssignment, "-=");
        m.insert(o::BinaryOperator::MultiplicationAssignment, "*=");
        m.insert(o::BinaryOperator::DivisionAssignment, "/=");
        m.insert(o::BinaryOperator::RemainderAssignment, "%=");
        m.insert(o::BinaryOperator::ExponentiationAssignment, "**=");
        m.insert(o::BinaryOperator::AndAssignment, "&&=");
        m.insert(o::BinaryOperator::OrAssignment, "||=");
        m.insert(o::BinaryOperator::NullishCoalesceAssignment, "??=");
        m
    };
}

pub struct EmitterVisitorContext {
    lines: Vec<EmittedLine>,
    indent: usize,
}

impl EmitterVisitorContext {
    pub fn create_root() -> Self {
        EmitterVisitorContext::new(0)
    }

    pub fn new(indent: usize) -> Self {
        EmitterVisitorContext {
            lines: vec![EmittedLine::new(indent)],
            indent,
        }
    }

    fn current_line(&self) -> &EmittedLine {
        self.lines.last().unwrap()
    }

    fn current_line_mut(&mut self) -> &mut EmittedLine {
        self.lines.last_mut().unwrap()
    }

    pub fn println(&mut self, from: Option<&dyn HasSourceSpan>, last_part: &str) {
        self.print(from, last_part, true);
    }

    pub fn line_is_empty(&self) -> bool {
        self.current_line().parts.is_empty()
    }

    pub fn line_length(&self) -> usize {
        self.current_line().indent * INDENT_WITH.len() + self.current_line().parts_length
    }

    pub fn print(&mut self, from: Option<&dyn HasSourceSpan>, part: &str, new_line: bool) {
        if !part.is_empty() {
            let current = self.current_line_mut();
            current.parts.push(part.to_string());
            current.parts_length += part.len();
            current.src_spans.push(
                from.and_then(|f| f.source_span()).cloned()
            );
        }
        if new_line {
            self.lines.push(EmittedLine::new(self.indent));
        }
    }

    pub fn remove_empty_last_line(&mut self) {
        if self.line_is_empty() {
            self.lines.pop();
        }
    }

    pub fn inc_indent(&mut self) {
        self.indent += 1;
        if self.line_is_empty() {
            self.current_line_mut().indent = self.indent;
        }
    }

    pub fn dec_indent(&mut self) {
        self.indent -= 1;
        if self.line_is_empty() {
            self.current_line_mut().indent = self.indent;
        }
    }

    pub fn to_source(&self) -> String {
        self.source_lines()
            .iter()
            .map(|l| {
                if !l.parts.is_empty() {
                    format!("{}{}", create_indent(l.indent), l.parts.join(""))
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn to_source_map_generator(
        &self,
        gen_file_path: &str,
        starts_at_line: usize,
    ) -> SourceMapGenerator {
        let mut map = SourceMapGenerator::new(Some(gen_file_path.to_string()));

        let mut first_offset_mapped = false;
        let mut map_first_offset_if_needed = || {
            if !first_offset_mapped {
                // Add a single space so that tools won't try to load the file from disk
                map.add_source(gen_file_path.to_string(), Some(" ".to_string()));
                let _ = map.add_mapping(0, Some(gen_file_path.to_string()), Some(0), Some(0));
                first_offset_mapped = true;
            }
        };

        for _ in 0..starts_at_line {
            map.add_line();
            map_first_offset_if_needed();
        }

        for line in self.source_lines() {
            map.add_line();
            let mut col0 = line.indent * INDENT_WITH.len();

            for (i, part) in line.parts.iter().enumerate() {
                if let Some(Some(span)) = line.src_spans.get(i) {
                    map_first_offset_if_needed();
                    let _ = map.add_mapping(
                        col0,
                        Some(span.start.file.url.clone()),
                        Some(span.start.line),
                        Some(span.start.col),
                    );
                }
                col0 += part.len();
            }
        }

        map
    }

    fn source_lines(&self) -> &[EmittedLine] {
        &self.lines
    }
}

pub trait HasSourceSpan {
    fn source_span(&self) -> Option<&ParseSourceSpan>;
}

fn create_indent(count: usize) -> String {
    INDENT_WITH.repeat(count)
}

/// Escape identifier for safe use in generated code
pub fn escape_identifier(input: &str, quote: bool) -> String {
    // TODO: Implement proper escaping logic
    if quote {
        format!("'{}'", input.replace('\'', "\\'"))
    } else {
        input.to_string()
    }
}

/// Abstract base emitter visitor
/// TODO: Implement full visitor pattern for expressions and statements
pub struct AbstractEmitterVisitor {
    pub print_types: bool,
}

impl AbstractEmitterVisitor {
    pub fn new(print_types: bool) -> Self {
        AbstractEmitterVisitor { print_types }
    }

    // TODO: Implement visitor methods for all expression and statement types
    // - visitReadVarExpr
    // - visitWriteVarExpr
    // - visitBinaryOperatorExpr
    // - visitLiteralExpr
    // - visitDeclareVarStmt
    // - etc.
}





