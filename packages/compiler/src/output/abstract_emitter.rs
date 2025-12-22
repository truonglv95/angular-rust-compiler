//! Abstract Emitter Module
//!
//! Corresponds to packages/compiler/src/output/abstract_emitter.ts
//! Base emitter functionality for code generation

use crate::output::output_ast as o;
use crate::output::output_ast::ExpressionTrait;
use crate::output::source_map::SourceMapGenerator;
use crate::parse_util::ParseSourceSpan;
use std::collections::HashMap;
use std::any::Any;

#[allow(dead_code)]
const SINGLE_QUOTE_ESCAPE_STRING_RE: &str = r"'|\\|\n|\r|\$";
const LEGAL_IDENTIFIER_RE: &str = r"^[a-zA-Z_$ɵ][0-9a-zA-Z_$ɵ]*$";
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
    pub static ref BINARY_OPERATORS: HashMap<o::BinaryOperator, &'static str> = {
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
        let mut last_file_url: Option<String> = None;
        let mut last_line: Option<usize> = None;
        let mut last_col: Option<usize> = None;

        for _ in 0..starts_at_line {
            map.add_line();
            if !first_offset_mapped {
                // Add a single space so that tools won't try to load the file from disk
                map.add_source(gen_file_path.to_string(), Some(" ".to_string()));
                let _ = map.add_mapping(0, Some(gen_file_path.to_string()), Some(0), Some(0));
                first_offset_mapped = true;
                last_file_url = Some(gen_file_path.to_string());
                last_line = Some(0);
                last_col = Some(0);
            }
        }

        let lines = self.source_lines();
        let effective_len = if !lines.is_empty() && lines.last().unwrap().parts.is_empty() {
             lines.len() - 1
        } else {
             lines.len()
        };

        for line in &lines[0..effective_len] {
            map.add_line();
            let mut col0 = line.indent * INDENT_WITH.len();

            for (i, part) in line.parts.iter().enumerate() {
                if !first_offset_mapped {
                    let has_span = line.src_spans.get(i).and_then(|s| s.as_ref()).is_some();
                    if !has_span || col0 > 0 {
                         // Add a single space so that tools won't try to load the file from disk
                        map.add_source(gen_file_path.to_string(), Some(" ".to_string()));
                        let _ = map.add_mapping(0, Some(gen_file_path.to_string()), Some(0), Some(0));
                        last_file_url = Some(gen_file_path.to_string());
                        last_line = Some(0);
                        last_col = Some(0);
                    }
                    first_offset_mapped = true;
                }

                if let Some(Some(span)) = line.src_spans.get(i) {
                     let url = span.start.file.url.clone();
                     let line = span.start.line;
                     let col = span.start.col;

                     // Coalesce identical spans
                     let is_identical = last_file_url.as_ref() == Some(&url) &&
                                        last_line == Some(line) &&
                                        last_col == Some(col);
                    
                     if !is_identical {
                        map.add_source(
                            url.clone(),
                            Some(span.start.file.content.clone()),
                        );
                        let _ = map.add_mapping(
                            col0,
                            Some(url.clone()),
                            Some(line),
                            Some(col),
                        );
                        last_file_url = Some(url);
                        last_line = Some(line);
                        last_col = Some(col);
                     }
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
pub fn escape_identifier(input: &str, escape_dollar: bool, always_quote: bool) -> String {
    if input.is_empty() {
        return "''".to_string();
    }
    
    let is_legal = regex::Regex::new(LEGAL_IDENTIFIER_RE).unwrap().is_match(input);
    if !always_quote && is_legal {
        return input.to_string();
    }
    
    let mut escaped = input.replace('\\', "\\\\");
    escaped = escaped.replace('\'', "\\'");
    escaped = escaped.replace('\n', "\\n");
    escaped = escaped.replace('\r', "\\r");
    if escape_dollar {
        escaped = escaped.replace('$', "\\$");
    }
    
    format!("'{}'", escaped)
}

/// Abstract base emitter visitor
pub struct AbstractEmitterVisitor {
    pub print_types: bool,
}

impl AbstractEmitterVisitor {
    pub fn new(print_types: bool) -> Self {
        AbstractEmitterVisitor { print_types }
    }
}

impl o::ExpressionVisitor for AbstractEmitterVisitor {
    fn visit_raw_code_expr(&mut self, expr: &o::RawCodeExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        ctx.print(Some(expr), &expr.code, false);
        Box::new(())
    }
    fn visit_read_var_expr(&mut self, expr: &o::ReadVarExpr, context: &mut dyn Any) -> Box<dyn Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        let name = escape_identifier(&expr.name, false, false);
        ctx.print(Some(expr as &dyn HasSourceSpan), &name, false);
        Box::new(())
    }

    fn visit_write_var_expr(&mut self, expr: &o::WriteVarExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            let name = escape_identifier(&expr.name, false, false);
            ctx.print(Some(expr), &name, false);
            ctx.print(Some(expr), " = ", false);
        }
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_write_key_expr(&mut self, expr: &o::WriteKeyExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "[", false);
        }
        expr.index.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "] = ", false);
        }
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_write_prop_expr(&mut self, expr: &o::WritePropExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ".", false);
            let name = escape_identifier(&expr.name, false, false);
            ctx.print(Some(expr), &name, false);
            ctx.print(Some(expr), " = ", false);
        }
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_invoke_function_expr(&mut self, expr: &o::InvokeFunctionExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        match &*expr.fn_ {
        o::Expression::ArrowFn(_) | o::Expression::Fn(_) => {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.print(Some(expr), "(", false);
            }
            expr.fn_.as_ref().visit_expression(self, context);
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.print(Some(expr), ")", false);
            }
        }
        _ => {
            expr.fn_.as_ref().visit_expression(self, context);
        }
    }
    {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        ctx.print(Some(expr), "(", false);
    }
        for (i, arg) in expr.args.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
            }
            arg.visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_tagged_template_expr(&mut self, _expr: &o::TaggedTemplateLiteralExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // TODO: Implement tagged template literal
        Box::new(())
    }

    fn visit_instantiate_expr(&mut self, expr: &o::InstantiateExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "new ", false);
        }
        // Wrap class expression in parentheses if it's a binary expression or conditional
        // to ensure correct operator precedence: `new (a || b)()` not `new a || b()`
        let needs_parens = matches!(
            expr.class_expr.as_ref(),
            o::Expression::BinaryOp(_) | o::Expression::Conditional(_)
        );
        if needs_parens {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        expr.class_expr.as_ref().visit_expression(self, context);
        if needs_parens {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        for (i, arg) in expr.args.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
            }
            arg.visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_literal_expr(&mut self, expr: &o::LiteralExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        let value_str = match &expr.value {
            o::LiteralValue::Null => "null".to_string(),
            o::LiteralValue::String(s) => escape_identifier(s, true, true),
            o::LiteralValue::Number(n) => n.to_string(),
            o::LiteralValue::Bool(b) => b.to_string(),
        };
        ctx.print(Some(expr), &value_str, false);
        Box::new(())
    }

    fn visit_localized_string(&mut self, _expr: &o::LocalizedString, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // TODO: Implement localized string
        Box::new(())
    }

    fn visit_external_expr(&mut self, expr: &o::ExternalExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        let ref_expr = &expr.value;
        if let Some(module_name) = &ref_expr.module_name {
            ctx.print(Some(expr), module_name, false);
            ctx.print(Some(expr), ".", false);
        }
        if let Some(name) = &ref_expr.name {
            ctx.print(Some(expr), name, false);
        }
        Box::new(())
    }

    fn visit_binary_operator_expr(&mut self, expr: &o::BinaryOperatorExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        expr.lhs.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            if let Some(op_str) = BINARY_OPERATORS.get(&expr.operator) {
                ctx.print(Some(expr), " ", false);
                ctx.print(Some(expr), op_str, false);
                ctx.print(Some(expr), " ", false);
            }
        }
        expr.rhs.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_read_prop_expr(&mut self, expr: &o::ReadPropExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ".", false);
            let name = escape_identifier(&expr.name, false, false);
            ctx.print(Some(expr), &name, false);
        }
        Box::new(())
    }

    fn visit_read_key_expr(&mut self, expr: &o::ReadKeyExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "[", false);
        }
        expr.index.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "]", false);
        }
        Box::new(())
    }

    fn visit_conditional_expr(&mut self, expr: &o::ConditionalExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        expr.condition.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), " ? ", false);
        }
        expr.true_case.as_ref().visit_expression(self, context);
        if let Some(false_case) = &expr.false_case {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.print(Some(expr), " : ", false);
            }
            false_case.as_ref().visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_unary_operator_expr(&mut self, expr: &o::UnaryOperatorExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            let op_str = match expr.operator {
                o::UnaryOperator::Minus => "-",
                o::UnaryOperator::Plus => "+",
            };
            ctx.print(Some(expr), op_str, false);
        }
        expr.expr.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_parenthesized_expr(&mut self, expr: &o::ParenthesizedExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
        }
        expr.expr.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_function_expr(&mut self, expr: &o::FunctionExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            if let Some(name) = &expr.name {
                ctx.print(Some(expr), "function ", false);
                let func_name = escape_identifier(name, false, false);
                ctx.print(Some(expr), &func_name, false);
            } else {
                ctx.print(Some(expr), "function", false);
            }
            ctx.print(Some(expr), "(", false);
            for (i, param) in expr.params.iter().enumerate() {
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
                let param_name = escape_identifier(&param.name, false, false);
                ctx.print(Some(expr), &param_name, false);
            }
            ctx.println(Some(expr), ") {");
            ctx.inc_indent();
        }
        for statement in &expr.statements {
            statement.visit_statement(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.dec_indent();
            ctx.println(Some(expr), "}");
        }
        Box::new(())
    }

    fn visit_arrow_function_expr(&mut self, expr: &o::ArrowFunctionExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "(", false);
            for (i, param) in expr.params.iter().enumerate() {
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
                let param_name = escape_identifier(&param.name, false, false);
                ctx.print(Some(expr), &param_name, false);
            }
            ctx.print(Some(expr), ") => ", false);
        }
        match &expr.body {
            o::ArrowFunctionBody::Expression(e) => {
                e.as_ref().visit_expression(self, context);
            }
            o::ArrowFunctionBody::Statements(stmts) => {
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.println(Some(expr), "{");
                    ctx.inc_indent();
                }
                for statement in stmts {
                    statement.visit_statement(self, context);
                }
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.dec_indent();
                    ctx.println(Some(expr), "}");
                }
            }
        }
        Box::new(())
    }

    fn visit_literal_array_expr(&mut self, expr: &o::LiteralArrayExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "[", false);
        }
        for (i, entry) in expr.entries.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
            }
            entry.visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "]", false);
        }
        Box::new(())
    }

    fn visit_literal_map_expr(&mut self, expr: &o::LiteralMapExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "{", false);
        }
        for (i, entry) in expr.entries.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
                let key = if entry.quoted {
                    escape_identifier(&entry.key, true, true)
                } else {
                    escape_identifier(&entry.key, false, false)
                };
                ctx.print(Some(expr), &key, false);
                ctx.print(Some(expr), ": ", false);
            }
            entry.value.as_ref().visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "}", false);
        }
        Box::new(())
    }

    fn visit_comma_expr(&mut self, expr: &o::CommaExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        for (i, part) in expr.parts.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
            }
            part.visit_expression(self, context);
        }
        Box::new(())
    }

    fn visit_typeof_expr(&mut self, expr: &o::TypeofExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "typeof ", false);
        }
        expr.expr.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_void_expr(&mut self, expr: &o::VoidExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "void ", false);
        }
        expr.expr.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_not_expr(&mut self, expr: &o::NotExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "!", false);
        }
        expr.condition.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_if_null_expr(&mut self, expr: &o::IfNullExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        expr.condition.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), " ?? ", false);
        }
        expr.null_case.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_assert_not_null_expr(&mut self, expr: &o::AssertNotNullExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        expr.condition.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "!", false);
        }
        Box::new(())
    }

    fn visit_cast_expr(&mut self, expr: &o::CastExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // TypeScript-style cast: <Type>value or value as Type
        // For JavaScript, we just emit the value
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_dynamic_import_expr(&mut self, expr: &o::DynamicImportExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "import(", false);
            let url = escape_identifier(&expr.url, true, true);
            ctx.print(Some(expr), &url, false);
            ctx.print(Some(expr), ")", false);
        }
        Box::new(())
    }

    fn visit_template_literal_expr(&mut self, expr: &o::TemplateLiteralExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "`", false);
        }
        for (i, element) in expr.elements.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.print(Some(expr), &element.text, false);
            }
            if i < expr.expressions.len() {
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(Some(expr), "${", false);
                }
                expr.expressions[i].visit_expression(self, context);
                {
                    let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                    ctx.print(Some(expr), "}", false);
                }
            }
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(expr), "`", false);
        }
        Box::new(())
    }

    fn visit_regular_expression_literal(&mut self, expr: &o::RegularExpressionLiteralExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        ctx.print(Some(expr), "/", false);
        ctx.print(Some(expr), &expr.pattern, false);
        ctx.print(Some(expr), "/", false);
        if !expr.flags.is_empty() {
             ctx.print(Some(expr), &expr.flags, false);
        }
        Box::new(())
    }

    fn visit_wrapped_node_expr(&mut self, _expr: &o::WrappedNodeExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // WrappedNodeExpr should not be emitted directly
        // This is typically used for TypeScript AST nodes that need special handling
        Box::new(())
    }
    
    // IR Expression visitor methods
    // Note: IR expressions are internal and should be converted to regular expressions
    // in the `reify` phase before emission. These implementations handle cases where
    // IR expressions might still be present during emission (which should be rare).
    
    fn visit_lexical_read_expr(&mut self, expr: &crate::template::pipeline::ir::expression::LexicalReadExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // LexicalReadExpr should be converted to ReadVarExpr before emission
        // For now, emit as a variable read
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        let name = escape_identifier(&expr.name, false, false);
        // IR expressions don't implement HasSourceSpan, so pass None for source span
        ctx.print(None, &name, false);
        Box::new(())
    }
    
    fn visit_reference_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::ReferenceExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // ReferenceExpr should be converted before emission
        panic!("ReferenceExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_context_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::ContextExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // ContextExpr should be converted before emission
        // Emit as a context variable (typically "ctx")
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        // IR expressions don't implement HasSourceSpan, so pass None for source span
        ctx.print(None, "ctx", false);
        Box::new(())
    }
    
    fn visit_next_context_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::NextContextExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("NextContextExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_get_current_view_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::GetCurrentViewExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("GetCurrentViewExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_restore_view_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::RestoreViewExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("RestoreViewExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_reset_view_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::ResetViewExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("ResetViewExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_read_variable_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::ReadVariableExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("ReadVariableExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_pure_function_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::PureFunctionExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("PureFunctionExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_pure_function_parameter_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::PureFunctionParameterExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("PureFunctionParameterExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_pipe_binding_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::PipeBindingExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("PipeBindingExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_pipe_binding_variadic_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::PipeBindingVariadicExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("PipeBindingVariadicExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_safe_property_read_expr(&mut self, expr: &crate::template::pipeline::ir::expression::SafePropertyReadExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // SafePropertyReadExpr should be expanded to null check before emission
        // For now, emit as a regular property read (unsafe)
        expr.receiver.as_ref().visit_expression(self, context);
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        // IR expressions don't implement HasSourceSpan, so pass None for source span
        ctx.print(None, ".", false);
        let name = escape_identifier(&expr.name, false, false);
        ctx.print(None, &name, false);
        Box::new(())
    }
    
    fn visit_safe_keyed_read_expr(&mut self, expr: &crate::template::pipeline::ir::expression::SafeKeyedReadExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // SafeKeyedReadExpr should be expanded to null check before emission
        // For now, emit as a regular keyed read (unsafe)
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            // IR expressions don't implement HasSourceSpan, so pass None for source span
            ctx.print(None, "[", false);
        }
        expr.index.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(None, "]", false);
        }
        Box::new(())
    }
    
    fn visit_safe_invoke_function_expr(&mut self, expr: &crate::template::pipeline::ir::expression::SafeInvokeFunctionExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // SafeInvokeFunctionExpr should be expanded to null check before emission
        // For now, emit as a regular function call (unsafe)
        expr.receiver.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            // IR expressions don't implement HasSourceSpan, so pass None for source span
            ctx.print(None, "(", false);
        }
        for (i, arg) in expr.args.iter().enumerate() {
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                if i > 0 {
                    ctx.print(None, ", ", false);
                }
            }
            arg.visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(None, ")", false);
        }
        Box::new(())
    }
    
    fn visit_safe_ternary_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::SafeTernaryExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("SafeTernaryExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_empty_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::EmptyExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // EmptyExpr should be stripped before emission, but if it reaches here, emit nothing
        Box::new(())
    }
    
    fn visit_assign_temporary_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::AssignTemporaryExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("AssignTemporaryExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_read_temporary_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::ReadTemporaryExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("ReadTemporaryExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_slot_literal_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::SlotLiteralExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("SlotLiteralExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_conditional_case_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::ConditionalCaseExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("ConditionalCaseExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_const_collected_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::ConstCollectedExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("ConstCollectedExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_two_way_binding_set_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::TwoWayBindingSetExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("TwoWayBindingSetExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_context_let_reference_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::ContextLetReferenceExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("ContextLetReferenceExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_store_let_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::StoreLetExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("StoreLetExpr should not be emitted directly - must be reified first");
    }
    
    fn visit_track_context_expr(&mut self, _expr: &crate::template::pipeline::ir::expression::TrackContextExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        panic!("TrackContextExpr should not be emitted directly - must be reified first");
    }
}

impl o::StatementVisitor for AbstractEmitterVisitor {
    fn visit_declare_var_stmt(&mut self, stmt: &o::DeclareVarStmt, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            // Use "const" for Final modifier, otherwise "var"
            let keyword = match stmt.modifiers {
                o::StmtModifier::Final => "const ",
                _ => "var ",
            };
            ctx.print(Some(stmt), keyword, false);
            let name = escape_identifier(&stmt.name, false, false);
            ctx.print(Some(stmt), &name, false);
            if let Some(_value) = &stmt.value {
                ctx.print(Some(stmt), " = ", false);
            }
        }
        if let Some(value) = &stmt.value {
            value.as_ref().visit_expression(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.println(Some(stmt), ";");
        }
        Box::new(())
    }

    fn visit_declare_function_stmt(&mut self, stmt: &o::DeclareFunctionStmt, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), "function ", false);
            let name = escape_identifier(&stmt.name, false, false);
            ctx.print(Some(stmt), &name, false);
            ctx.print(Some(stmt), "(", false);
            for (i, param) in stmt.params.iter().enumerate() {
                if i > 0 {
                    ctx.print(Some(stmt), ", ", false);
                }
                let param_name = escape_identifier(&param.name, false, false);
                ctx.print(Some(stmt), &param_name, false);
            }
            ctx.println(Some(stmt), ") {");
            ctx.inc_indent();
        }
        for statement in &stmt.statements {
            statement.visit_statement(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.dec_indent();
            ctx.println(Some(stmt), "}");
        }
        Box::new(())
    }

    fn visit_expression_stmt(&mut self, stmt: &o::ExpressionStatement, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        stmt.expr.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.println(Some(stmt), ";");
        }
        Box::new(())
    }

    fn visit_return_stmt(&mut self, stmt: &o::ReturnStatement, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), "return ", false);
        }
        stmt.value.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.println(Some(stmt), ";");
        }
        Box::new(())
    }

    fn visit_if_stmt(&mut self, stmt: &o::IfStmt, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), "if (", false);
        }
        stmt.condition.as_ref().visit_expression(self, context);
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.println(Some(stmt), ") {");
            ctx.inc_indent();
        }
        for statement in &stmt.true_case {
            statement.visit_statement(self, context);
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.dec_indent();
            if !stmt.false_case.is_empty() {
                ctx.println(Some(stmt), "} else {");
                ctx.inc_indent();
            }
        }
        if !stmt.false_case.is_empty() {
            for statement in &stmt.false_case {
                statement.visit_statement(self, context);
            }
            {
                let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
                ctx.dec_indent();
            }
        }
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.println(Some(stmt), "}");
        }
        Box::new(())
    }
}





