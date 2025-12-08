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

        for _ in 0..starts_at_line {
            map.add_line();
            if !first_offset_mapped {
                // Add a single space so that tools won't try to load the file from disk
                map.add_source(gen_file_path.to_string(), Some(" ".to_string()));
                let _ = map.add_mapping(0, Some(gen_file_path.to_string()), Some(0), Some(0));
                first_offset_mapped = true;
            }
        }

        for line in self.source_lines() {
            map.add_line();
            let mut col0 = line.indent * INDENT_WITH.len();

            for (i, part) in line.parts.iter().enumerate() {
                if let Some(Some(span)) = line.src_spans.get(i) {
                    if !first_offset_mapped {
                        // Add a single space so that tools won't try to load the file from disk
                        map.add_source(gen_file_path.to_string(), Some(" ".to_string()));
                        let _ = map.add_mapping(0, Some(gen_file_path.to_string()), Some(0), Some(0));
                        first_offset_mapped = true;
                    }
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
pub struct AbstractEmitterVisitor {
    pub print_types: bool,
}

impl AbstractEmitterVisitor {
    pub fn new(print_types: bool) -> Self {
        AbstractEmitterVisitor { print_types }
    }
}

impl o::ExpressionVisitor for AbstractEmitterVisitor {
    fn visit_read_var_expr(&mut self, expr: &o::ReadVarExpr, context: &mut dyn Any) -> Box<dyn Any> {
        let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
        let name = escape_identifier(&expr.name, false);
        ctx.print(Some(expr as &dyn HasSourceSpan), &name, false);
        Box::new(())
    }

    fn visit_write_var_expr(&mut self, expr: &o::WriteVarExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            let name = escape_identifier(&expr.name, false);
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
            let name = escape_identifier(&expr.name, false);
            ctx.print(Some(expr), &name, false);
            ctx.print(Some(expr), " = ", false);
        }
        expr.value.as_ref().visit_expression(self, context);
        Box::new(())
    }

    fn visit_invoke_function_expr(&mut self, expr: &o::InvokeFunctionExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        expr.fn_.as_ref().visit_expression(self, context);
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
        expr.class_expr.as_ref().visit_expression(self, context);
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
            o::LiteralValue::String(s) => escape_identifier(s, true),
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
            let name = escape_identifier(&expr.name, false);
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

    fn visit_function_expr(&mut self, expr: &o::FunctionExpr, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            if let Some(name) = &expr.name {
                ctx.print(Some(expr), "function ", false);
                let func_name = escape_identifier(name, false);
                ctx.print(Some(expr), &func_name, false);
            } else {
                ctx.print(Some(expr), "function", false);
            }
            ctx.print(Some(expr), "(", false);
            for (i, param) in expr.params.iter().enumerate() {
                if i > 0 {
                    ctx.print(Some(expr), ", ", false);
                }
                let param_name = escape_identifier(&param.name, false);
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
                let param_name = escape_identifier(&param.name, false);
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
                    escape_identifier(&entry.key, true)
                } else {
                    escape_identifier(&entry.key, false)
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
            let url = escape_identifier(&expr.url, true);
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

    fn visit_wrapped_node_expr(&mut self, _expr: &o::WrappedNodeExpr, _context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        // WrappedNodeExpr should not be emitted directly
        // This is typically used for TypeScript AST nodes that need special handling
        Box::new(())
    }
}

impl o::StatementVisitor for AbstractEmitterVisitor {
    fn visit_declare_var_stmt(&mut self, stmt: &o::DeclareVarStmt, context: &mut dyn std::any::Any) -> Box<dyn std::any::Any> {
        {
            let ctx = context.downcast_mut::<EmitterVisitorContext>().unwrap();
            ctx.print(Some(stmt), "var ", false);
            let name = escape_identifier(&stmt.name, false);
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
            let name = escape_identifier(&stmt.name, false);
            ctx.print(Some(stmt), &name, false);
            ctx.print(Some(stmt), "(", false);
            for (i, param) in stmt.params.iter().enumerate() {
                if i > 0 {
                    ctx.print(Some(stmt), ", ", false);
                }
                let param_name = escape_identifier(&param.name, false);
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





