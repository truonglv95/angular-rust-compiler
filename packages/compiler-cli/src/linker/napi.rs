#![cfg(feature = "napi-bindings")]
#![deny(clippy::all)]
use std::any::Any;
use std::collections::HashMap;

use crate::linker::ast_value::AstValue;
use crate::linker::oxc_ast_host::{OxcAstHost, OxcNode};
use crate::linker::partial_linkers::partial_linker_selector::PartialLinkerSelector;
use angular_compiler::constant_pool::ConstantPool;
use angular_compiler::output::abstract_emitter::EmitterVisitorContext;
use angular_compiler::output::abstract_js_emitter::AbstractJsEmitterVisitor;
use angular_compiler::output::output_ast as o;
use angular_compiler::output::output_ast::ExpressionTrait;
use napi::{Error, Result, Status};
use napi_derive::napi;
use oxc_allocator::Allocator;
use oxc_ast::ast::{self, Expression};
use oxc_parser::Parser;
use oxc_span::SourceType;

#[napi]
pub fn link_file(source_code: String, filename: String) -> Result<String> {
    eprintln!("[Linker NAPI Debug] link_file: {}", filename);
    let allocator = Allocator::default();
    let clean_filename = filename.split('?').next().unwrap_or(&filename);
    let source_type = SourceType::from_path(clean_filename).unwrap_or_default();

    let parser = Parser::new(&allocator, &source_code, source_type);
    let ret = parser.parse();

    if !ret.errors.is_empty() {
        let mut log_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/linker.log")
            .unwrap();
        writeln!(
            log_file,
            "Parse error in {}: {:?}",
            filename,
            ret.errors.first().unwrap()
        )
        .unwrap();
        return Err(Error::new(
            Status::GenericFailure,
            format!("Parse error: {:?}", ret.errors.first().unwrap()),
        ));
    }

    let program = ret.program;

    // Collect imports
    let mut imports = HashMap::new(); // module -> alias
    let mut alias_imports = HashMap::new(); // alias -> module
    let mut named_imports = HashMap::new(); // (module, export_name) -> local_name

    for stmt in &program.body {
        if let ast::Statement::ImportDeclaration(decl) = stmt {
            if let Some(specifiers) = &decl.specifiers {
                for spec in specifiers {
                    match spec {
                        ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(ns) => {
                            let module = decl.source.value.as_str();
                            let alias = ns.local.name.as_str();
                            imports.insert(module.to_string(), alias.to_string());
                            alias_imports.insert(alias.to_string(), module.to_string());
                            eprintln!("[Linker Debug] Namespace Import: {} -> {}", module, alias);
                        }
                        ast::ImportDeclarationSpecifier::ImportSpecifier(s) => {
                            let module = decl.source.value.as_str();
                            let local = s.local.name.as_str();
                            let imported = match &s.imported {
                                ast::ModuleExportName::IdentifierName(id) => id.name.as_str(),
                                ast::ModuleExportName::IdentifierReference(id) => id.name.as_str(),
                                ast::ModuleExportName::StringLiteral(sl) => sl.value.as_str(),
                            };
                            named_imports.insert(
                                (module.to_string(), imported.to_string()),
                                local.to_string(),
                            );
                            eprintln!(
                                "[Linker Debug] Named Import: {}::{} -> {}",
                                module, imported, local
                            );
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // Visitor to collect class metadata
    struct ClassMetadataVisitor<'a> {
        metadata: HashMap<String, &'a ast::Expression<'a>>,
    }

    impl<'a> ClassMetadataVisitor<'a> {
        fn new() -> Self {
            Self {
                metadata: HashMap::new(),
            }
        }

        fn visit_program(&mut self, program: &ast::Program<'a>) {
            for stmt in &program.body {
                self.visit_statement(stmt);
            }
        }

        fn visit_statement(&mut self, stmt: &ast::Statement<'a>) {
            match stmt {
                ast::Statement::ExpressionStatement(s) => self.visit_expression(&s.expression),
                ast::Statement::BlockStatement(s) => {
                    for st in &s.body {
                        self.visit_statement(st);
                    }
                }
                ast::Statement::VariableDeclaration(s) => {
                    for decl in &s.declarations {
                        if let Some(init) = &decl.init {
                            self.visit_expression(init);
                        }
                    }
                }
                _ => {}
            }
        }

        fn visit_expression(&mut self, expr: &ast::Expression<'a>) {
            match expr {
                ast::Expression::CallExpression(e) => self.visit_call_expression(e),
                ast::Expression::ParenthesizedExpression(e) => self.visit_expression(&e.expression),
                ast::Expression::SequenceExpression(e) => {
                    for ex in &e.expressions {
                        self.visit_expression(ex);
                    }
                }
                ast::Expression::UnaryExpression(e) => {
                    self.visit_expression(&e.argument);
                }
                _ => {}
            }
        }

        fn visit_call_expression(&mut self, expr: &ast::CallExpression<'a>) {
            // Check for IIFE
            match &expr.callee {
                ast::Expression::FunctionExpression(f) => {
                    if let Some(body) = &f.body {
                        for s in &body.statements {
                            self.visit_statement(s);
                        }
                    }
                }
                ast::Expression::ArrowFunctionExpression(f) => {
                    for s in &f.body.statements {
                        self.visit_statement(s);
                    }
                }
                ast::Expression::ParenthesizedExpression(e) => {
                    if let ast::Expression::FunctionExpression(f) = &e.expression {
                        if let Some(body) = &f.body {
                            for s in &body.statements {
                                self.visit_statement(s);
                            }
                        }
                    }
                }
                _ => {}
            }

            // Check for i0.ɵɵngDeclareClassMetadata(...)
            if let Expression::StaticMemberExpression(member) = &expr.callee {
                if member.property.name == "ɵɵngDeclareClassMetadata" {
                    if let Some(arg) = expr.arguments.first() {
                        if let Some(arg_expr) = arg.as_expression() {
                            if let ast::Expression::ObjectExpression(obj) = arg_expr {
                                for prop in &obj.properties {
                                    if let ast::ObjectPropertyKind::ObjectProperty(p) = prop {
                                        if let ast::PropertyKey::StaticIdentifier(key) = &p.key {
                                            if key.name == "type" {
                                                if let ast::Expression::Identifier(id) = &p.value {
                                                    let class_name = id.name.as_str().to_string();
                                                    // SAFETY: extending lifetime to 'a for storage in metadata map.
                                                    // The AST nodes are arena/heap allocated and live for 'a.
                                                    let arg_expr_a = unsafe {
                                                        std::mem::transmute::<
                                                            &_,
                                                            &'a ast::Expression<'a>,
                                                        >(
                                                            arg_expr
                                                        )
                                                    };
                                                    self.metadata.insert(class_name, arg_expr_a);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Visitor to find calls
    struct LinkerVisitor<'a> {
        host: OxcAstHost<'a>,
        selector: PartialLinkerSelector<'a, OxcNode<'a>>,
        replacements: Vec<(u32, u32, String)>,
        errors: Vec<String>,
        imports: HashMap<String, String>,
        alias_imports: HashMap<String, String>,
        named_imports: HashMap<(String, String), String>, // New field: (module, exported) -> local
        class_metadata: HashMap<String, &'a ast::Expression<'a>>,
        source_url: &'a str,
        logs: Vec<String>,
    }

    impl<'a> LinkerVisitor<'a> {
        fn new(
            source_code: &'a str,
            imports: HashMap<String, String>,
            alias_imports: HashMap<String, String>,
            named_imports: HashMap<(String, String), String>,
            class_metadata: HashMap<String, &'a ast::Expression<'a>>,
            source_url: &'a str,
        ) -> Self {
            Self {
                host: OxcAstHost::new(source_code),
                selector: PartialLinkerSelector::new(),
                replacements: Vec::new(),
                errors: Vec::new(),
                imports,
                alias_imports,
                named_imports,
                class_metadata,
                source_url,
                logs: Vec::new(),
            }
        }

        fn emit_expression(&self, expr: &o::Expression) -> String {
            let expr = self.transform_expression(expr.clone());
            let mut visitor = AbstractJsEmitterVisitor::new();
            let mut ctx = EmitterVisitorContext::new(0); // 0 indent
            expr.visit_expression(&mut visitor, &mut ctx);
            ctx.to_source()
        }

        fn emit_statements(&self, stmts: Vec<o::Statement>) -> String {
            let stmts = self.transform_statements(stmts);
            let mut visitor = AbstractJsEmitterVisitor::new();
            let mut ctx = EmitterVisitorContext::new(0);
            for stmt in stmts {
                stmt.visit_statement(&mut visitor, &mut ctx);
            }
            ctx.to_source()
        }

        fn transform_expression(&self, expr: o::Expression) -> o::Expression {
            match expr {
                o::Expression::External(e) => {
                    if let Some(module) = &e.value.module_name {
                        // Check for named import first
                        if let Some(prop) = &e.value.name {
                            if let Some(local) =
                                self.named_imports.get(&(module.clone(), prop.clone()))
                            {
                                return o::Expression::ReadVar(o::ReadVarExpr {
                                    name: local.clone(),
                                    type_: None,
                                    source_span: None,
                                });
                            }

                            // Fallback: Check if we have any named import with the same export name.
                            // This handles cases where the External module path (e.g. resolved file path)
                            // differs from the import specifier (e.g. package name).
                            for ((_mod, export_name), local) in &self.named_imports {
                                if export_name == prop {
                                    eprintln!("[Linker Debug] Fuzzy match for External: {}::{} -> local {}", module, prop, local);
                                    return o::Expression::ReadVar(o::ReadVarExpr {
                                        name: local.clone(),
                                        type_: None,
                                        source_span: None,
                                    });
                                }
                            }
                        }

                        if let Some(alias) = self.imports.get(module) {
                            let mut _name = alias.clone();
                            if let Some(prop) = &e.value.name {
                                let alias_expr = o::Expression::ReadVar(o::ReadVarExpr {
                                    name: alias.clone(),
                                    type_: None,
                                    source_span: None,
                                });

                                // Internal Angular properties starting with ɵ are valid identifiers
                                // but the abstract emitter's regex doesn't account for unicode characters
                                // causing it to quote the name (e.g. i0.'ɵɵdefineComponent').
                                // Use bracket access (ReadKeyExpr) for these cases.
                                if prop.contains('ɵ') {
                                    return o::Expression::ReadKey(o::ReadKeyExpr {
                                        receiver: Box::new(alias_expr),
                                        index: Box::new(o::Expression::Literal(o::LiteralExpr {
                                            value: o::LiteralValue::String(prop.clone()),
                                            type_: None,
                                            source_span: None,
                                        })),
                                        type_: None,
                                        source_span: None,
                                    });
                                }

                                return o::Expression::ReadProp(o::ReadPropExpr {
                                    receiver: Box::new(alias_expr),
                                    name: prop.clone(),
                                    type_: None,
                                    source_span: None,
                                });
                            } else {
                                return o::Expression::ReadVar(o::ReadVarExpr {
                                    name: alias.clone(),
                                    type_: None,
                                    source_span: None,
                                });
                            }
                        }
                    }
                    o::Expression::External(e)
                }
                o::Expression::InvokeFn(mut e) => {
                    e.fn_ = Box::new(self.transform_expression(*e.fn_));
                    e.args = e
                        .args
                        .into_iter()
                        .map(|arg| self.transform_expression(arg))
                        .collect();
                    o::Expression::InvokeFn(e)
                }
                o::Expression::ReadProp(mut e) => {
                    e.receiver = Box::new(self.transform_expression(*e.receiver));
                    o::Expression::ReadProp(e)
                }
                o::Expression::ReadKey(mut e) => {
                    e.receiver = Box::new(self.transform_expression(*e.receiver));
                    e.index = Box::new(self.transform_expression(*e.index));
                    o::Expression::ReadKey(e)
                }
                o::Expression::LiteralArray(mut e) => {
                    e.entries = e
                        .entries
                        .into_iter()
                        .map(|entry| self.transform_expression(entry))
                        .collect();
                    o::Expression::LiteralArray(e)
                }
                o::Expression::LiteralMap(mut e) => {
                    for entry in &mut e.entries {
                        entry.value = Box::new(self.transform_expression(*entry.value.clone()));
                    }
                    o::Expression::LiteralMap(e)
                }
                o::Expression::Parens(mut e) => {
                    e.expr = Box::new(self.transform_expression(*e.expr));
                    o::Expression::Parens(e)
                }
                o::Expression::Fn(mut e) => {
                    e.statements = self.transform_statements(e.statements);
                    o::Expression::Fn(e)
                }
                o::Expression::ArrowFn(mut e) => {
                    match e.body {
                        o::ArrowFunctionBody::Expression(expr) => {
                            e.body = o::ArrowFunctionBody::Expression(Box::new(
                                self.transform_expression(*expr),
                            ));
                        }
                        o::ArrowFunctionBody::Statements(stmts) => {
                            e.body =
                                o::ArrowFunctionBody::Statements(self.transform_statements(stmts));
                        }
                    }
                    o::Expression::ArrowFn(e)
                }
                o::Expression::Instantiate(mut e) => {
                    e.class_expr = Box::new(self.transform_expression(*e.class_expr));
                    e.args = e
                        .args
                        .into_iter()
                        .map(|arg| self.transform_expression(arg))
                        .collect();
                    o::Expression::Instantiate(e)
                }
                // Add other recursive variants as needed
                o::Expression::BinaryOp(mut e) => {
                    e.lhs = Box::new(self.transform_expression(*e.lhs));
                    e.rhs = Box::new(self.transform_expression(*e.rhs));
                    let is_assignment = matches!(
                        e.operator,
                        o::BinaryOperator::Assign
                            | o::BinaryOperator::AdditionAssignment
                            | o::BinaryOperator::SubtractionAssignment
                            | o::BinaryOperator::MultiplicationAssignment
                            | o::BinaryOperator::DivisionAssignment
                            | o::BinaryOperator::RemainderAssignment
                            | o::BinaryOperator::ExponentiationAssignment
                            | o::BinaryOperator::AndAssignment
                            | o::BinaryOperator::OrAssignment
                            | o::BinaryOperator::NullishCoalesceAssignment
                    );
                    let res = o::Expression::BinaryOp(e);
                    if is_assignment {
                        o::Expression::Parens(o::ParenthesizedExpr {
                            expr: Box::new(res),
                            type_: None,
                            source_span: None,
                        })
                    } else {
                        res
                    }
                }
                o::Expression::Conditional(mut e) => {
                    e.condition = Box::new(self.transform_expression(*e.condition));
                    e.true_case = Box::new(self.transform_expression(*e.true_case));
                    if let Some(false_case) = e.false_case {
                        e.false_case = Some(Box::new(self.transform_expression(*false_case)));
                    }
                    let res = o::Expression::Conditional(e);
                    o::Expression::Parens(o::ParenthesizedExpr {
                        expr: Box::new(res),
                        type_: None,
                        source_span: None,
                    })
                }
                o::Expression::NotExpr(mut e) => {
                    e.condition = Box::new(self.transform_expression(*e.condition));
                    o::Expression::NotExpr(e)
                }
                o::Expression::Unary(mut e) => {
                    e.expr = Box::new(self.transform_expression(*e.expr));
                    o::Expression::Unary(e)
                }
                o::Expression::WriteVar(mut e) => {
                    e.value = Box::new(self.transform_expression(*e.value));
                    let res = o::Expression::WriteVar(e);
                    o::Expression::Parens(o::ParenthesizedExpr {
                        expr: Box::new(res),
                        type_: None,
                        source_span: None,
                    })
                }
                o::Expression::WriteKey(mut e) => {
                    e.receiver = Box::new(self.transform_expression(*e.receiver));
                    e.index = Box::new(self.transform_expression(*e.index));
                    e.value = Box::new(self.transform_expression(*e.value));
                    let res = o::Expression::WriteKey(e);
                    o::Expression::Parens(o::ParenthesizedExpr {
                        expr: Box::new(res),
                        type_: None,
                        source_span: None,
                    })
                }
                o::Expression::WriteProp(mut e) => {
                    e.receiver = Box::new(self.transform_expression(*e.receiver));
                    e.value = Box::new(self.transform_expression(*e.value));
                    let res = o::Expression::WriteProp(e);
                    o::Expression::Parens(o::ParenthesizedExpr {
                        expr: Box::new(res),
                        type_: None,
                        source_span: None,
                    })
                }
                o::Expression::CommaExpr(mut e) => {
                    e.parts = e
                        .parts
                        .into_iter()
                        .map(|p| self.transform_expression(p))
                        .collect();
                    o::Expression::CommaExpr(e)
                }
                o::Expression::TypeOf(mut e) => {
                    e.expr = Box::new(self.transform_expression(*e.expr));
                    o::Expression::TypeOf(e)
                }
                o::Expression::Void(mut e) => {
                    e.expr = Box::new(self.transform_expression(*e.expr));
                    o::Expression::Void(e)
                }
                other => other,
            }
        }

        fn transform_statements(&self, stmts: Vec<o::Statement>) -> Vec<o::Statement> {
            stmts
                .into_iter()
                .map(|stmt| self.transform_statement(stmt))
                .collect()
        }

        fn transform_statement(&self, stmt: o::Statement) -> o::Statement {
            match stmt {
                o::Statement::Return(mut s) => {
                    s.value = Box::new(self.transform_expression(*s.value));
                    o::Statement::Return(s)
                }
                o::Statement::Expression(mut s) => {
                    s.expr = Box::new(self.transform_expression(*s.expr));
                    o::Statement::Expression(s)
                }
                o::Statement::DeclareVar(mut s) => {
                    if let Some(val) = s.value {
                        s.value = Some(Box::new(self.transform_expression(*val)));
                    }
                    o::Statement::DeclareVar(s)
                }
                o::Statement::IfStmt(mut s) => {
                    s.condition = Box::new(self.transform_expression(*s.condition));
                    s.true_case = self.transform_statements(s.true_case);
                    s.false_case = self.transform_statements(s.false_case);
                    o::Statement::IfStmt(s)
                }
                other => other,
            }
        }

        fn visit_program(&mut self, program: &ast::Program<'a>) {
            for stmt in &program.body {
                self.visit_statement(stmt);
            }
        }

        fn visit_statement(&mut self, stmt: &ast::Statement<'a>) {
            match stmt {
                ast::Statement::ExpressionStatement(s) => self.visit_expression(&s.expression),
                ast::Statement::BlockStatement(s) => {
                    for st in &s.body {
                        self.visit_statement(st);
                    }
                }
                ast::Statement::IfStatement(s) => {
                    self.visit_expression(&s.test);
                    self.visit_statement(&s.consequent);
                    if let Some(alt) = &s.alternate {
                        self.visit_statement(alt);
                    }
                }
                ast::Statement::ReturnStatement(s) => {
                    if let Some(arg) = &s.argument {
                        self.visit_expression(arg);
                    }
                }
                ast::Statement::VariableDeclaration(s) => {
                    for decl in &s.declarations {
                        if let Some(init) = &decl.init {
                            self.visit_expression(init);
                        }
                    }
                }
                ast::Statement::FunctionDeclaration(s) => {
                    if let Some(body) = &s.body {
                        for st in &body.statements {
                            self.visit_statement(st);
                        }
                    }
                }
                ast::Statement::ClassDeclaration(s) => {
                    for el in &s.body.body {
                        match el {
                            ast::ClassElement::MethodDefinition(m) => {
                                if let Some(body) = &m.value.body {
                                    for st in &body.statements {
                                        self.visit_statement(st);
                                    }
                                }
                            }
                            ast::ClassElement::PropertyDefinition(p) => {
                                if let Some(val) = &p.value {
                                    self.visit_expression(val);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                ast::Statement::ExportDefaultDeclaration(s) => match &s.declaration {
                    ast::ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                        if let Some(body) = &f.body {
                            for st in &body.statements {
                                self.visit_statement(st);
                            }
                        }
                    }
                    ast::ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                        for el in &c.body.body {
                            match el {
                                ast::ClassElement::MethodDefinition(m) => {
                                    if let Some(body) = &m.value.body {
                                        for st in &body.statements {
                                            self.visit_statement(st);
                                        }
                                    }
                                }
                                ast::ClassElement::PropertyDefinition(p) => {
                                    if let Some(val) = &p.value {
                                        self.visit_expression(val);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    target => {
                        if let Some(e) = target.as_expression() {
                            self.visit_expression(e);
                        }
                    }
                },
                ast::Statement::ExportNamedDeclaration(s) => {
                    if let Some(decl) = &s.declaration {
                        // crude handling reusing visit_statement by converting strictly if possible or just manual
                        // Declaration is Statement-like but wrapped.
                        // ast::Declaration is an enum.
                        match decl {
                            ast::Declaration::VariableDeclaration(v) => {
                                for d in &v.declarations {
                                    if let Some(init) = &d.init {
                                        self.visit_expression(init);
                                    }
                                }
                            }
                            ast::Declaration::FunctionDeclaration(f) => {
                                if let Some(body) = &f.body {
                                    for st in &body.statements {
                                        self.visit_statement(st);
                                    }
                                }
                            }
                            ast::Declaration::ClassDeclaration(c) => {
                                for el in &c.body.body {
                                    match el {
                                        ast::ClassElement::MethodDefinition(m) => {
                                            if let Some(body) = &m.value.body {
                                                for st in &body.statements {
                                                    self.visit_statement(st);
                                                }
                                            }
                                        }
                                        ast::ClassElement::PropertyDefinition(p) => {
                                            if let Some(val) = &p.value {
                                                self.visit_expression(val);
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                _ => {} // Ignore other statements
            }
        }

        fn visit_expression(&mut self, expr: &ast::Expression<'a>) {
            match expr {
                ast::Expression::CallExpression(e) => self.visit_call_expression(e),
                ast::Expression::AssignmentExpression(e) => {
                    self.visit_expression(&e.right);
                }
                ast::Expression::ObjectExpression(e) => {
                    for p in &e.properties {
                        match p {
                            ast::ObjectPropertyKind::ObjectProperty(prop) => {
                                self.visit_expression(&prop.value);
                            }
                            _ => {}
                        }
                    }
                }
                ast::Expression::ArrayExpression(e) => {
                    for el in &e.elements {
                        match el {
                            ast::ArrayExpressionElement::SpreadElement(s) => {
                                self.visit_expression(&s.argument)
                            }
                            target => {
                                if let Some(expr) = target.as_expression() {
                                    self.visit_expression(expr);
                                }
                            }
                        }
                    }
                }
                ast::Expression::SequenceExpression(e) => {
                    for ex in &e.expressions {
                        self.visit_expression(ex);
                    }
                }
                ast::Expression::ParenthesizedExpression(e) => {
                    self.visit_expression(&e.expression);
                }
                ast::Expression::ArrowFunctionExpression(e) => {
                    if let Some(body) = &e.body.statements.first() {
                        // Simple body check
                        // Actually body is FunctionBody which has statements.
                        for s in &e.body.statements {
                            self.visit_statement(s);
                        }
                    }
                }
                ast::Expression::FunctionExpression(e) => {
                    if let Some(body) = &e.body {
                        for s in &body.statements {
                            self.visit_statement(s);
                        }
                    }
                }
                _ => {}
            }
        }

        fn visit_call_expression(&mut self, expr: &ast::CallExpression<'a>) {
            // Check callee
            let callee = &expr.callee;
            let mut name = None;

            if let Expression::Identifier(ident) = callee {
                name = Some(ident.name.as_str());
            } else if let Expression::StaticMemberExpression(member) = callee {
                if let Expression::Identifier(_obj) = &member.object {
                    name = Some(member.property.name.as_str());
                }
            }

            if let Some(n) = name {
                {
                    use std::io::Write;
                    let mut f = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("/tmp/linker.log")
                        .unwrap();
                    writeln!(f, "[DEBUG] Visiting Call: {}", n).unwrap();
                }
                // Handle __decorate calls (JIT/Decorator transformation)
                if n == "__decorate" || n == "_ts_decorate" {
                    if expr.arguments.len() >= 2 {
                        // Arg 0: Decorators array
                        if let Some(decorators_arg) = expr.arguments[0].as_expression() {
                            if let Expression::ArrayExpression(decorators_array) = decorators_arg {
                                // Arg 1: Target (Class)
                                let target_arg = expr.arguments[1].as_expression();
                                let mut target_name = "Unknown";
                                if let Some(Expression::Identifier(ident)) = target_arg {
                                    target_name = ident.name.as_str();
                                }

                                for el in &decorators_array.elements {
                                    if let ast::ArrayExpressionElement::CallExpression(
                                        decorator_call,
                                    ) = el
                                    {
                                        // Check decorator name
                                        let mut dec_name = None;
                                        if let Expression::Identifier(ident) =
                                            &decorator_call.callee
                                        {
                                            dec_name = Some(ident.name.as_str());
                                        }

                                        if let Some(d_name) = dec_name {
                                            // We care about Angular decorators
                                            if self.selector.supports_declaration(d_name)
                                                && (d_name == "Component"
                                                    || d_name == "Directive"
                                                    || d_name == "Pipe"
                                                    || d_name == "Injectable"
                                                    || d_name == "NgModule")
                                            {
                                                if decorator_call.arguments.len() > 0 {
                                                    if let Some(meta_arg) =
                                                        decorator_call.arguments[0].as_expression()
                                                    {
                                                        // Link!
                                                        let arg_expr_a: &'a ast::Expression<'a> = unsafe {
                                                            std::mem::transmute(meta_arg)
                                                        };
                                                        let oxc_node =
                                                            OxcNode::Expression(arg_expr_a);
                                                        let value =
                                                            AstValue::new(oxc_node, &self.host);

                                                        match value.get_object() {
                                                            Ok(obj) => {
                                                                let linker =
                                                                    self.selector.get_linker(
                                                                        d_name, "0.0.0", "0.0.0",
                                                                    );
                                                                let mut constant_pool =
                                                                    ConstantPool::new(false);

                                                                // Link partial declaration (reads templateUrl!)
                                                                let result_expr = linker
                                                                    .link_partial_declaration(
                                                                        &mut constant_pool,
                                                                        &obj,
                                                                        self.source_url,
                                                                        "0.0.0",
                                                                        Some(target_name),
                                                                        Some(&self.alias_imports),
                                                                        None,
                                                                    );

                                                                let js_code = if constant_pool
                                                                    .statements
                                                                    .is_empty()
                                                                {
                                                                    self.emit_expression(
                                                                        &result_expr,
                                                                    )
                                                                } else {
                                                                    let stmts_code = self
                                                                        .emit_statements(
                                                                            constant_pool
                                                                                .statements,
                                                                        );
                                                                    let expr_code = self
                                                                        .emit_expression(
                                                                            &result_expr,
                                                                        );
                                                                    format!(
                                                                        "(function() {{ {} return {}; }})()",
                                                                        stmts_code, expr_code
                                                                    )
                                                                };

                                                                // Field name: ɵcmp, ɵdir, ɵpipe, ɵprov, ɵmod?
                                                                // PartialLinkerTrait doesn't expose field name.
                                                                // But we know standard mappings:
                                                                // Component -> ɵcmp
                                                                // Directive -> ɵdir
                                                                // Pipe -> ɵpipe
                                                                // Injectable -> ɵprov
                                                                // NgModule -> ɵmod

                                                                let field_name = match d_name {
                                                                    "Component" => "ɵcmp",
                                                                    "Directive" => "ɵdir",
                                                                    "Pipe" => "ɵpipe",
                                                                    "Injectable" => "ɵprov",
                                                                    "NgModule" => "ɵmod",
                                                                    _ => "ɵunknown",
                                                                };

                                                                let mut assignment = format!(
                                                                    "; {}.{} = {};",
                                                                    target_name,
                                                                    field_name,
                                                                    js_code
                                                                );

                                                                // Generate ɵfac for Component
                                                                if d_name == "Component" {
                                                                    println!("LOG: Visiting Component decorator");
                                                                    let fac_code = format!("; {}.ɵfac = function(t) {{ return new (t || {})({}); }};", target_name, target_name, "");
                                                                    assignment.push_str(&fac_code);

                                                                    let d_span =
                                                                        decorator_call.span;
                                                                    self.replacements.push((
                                                                        d_span.start,
                                                                        d_span.end,
                                                                        "void 0".to_string(),
                                                                    ));
                                                                }
                                                                println!("[Rust Linker] Linked Decorator {} on '{}' -> {}", d_name, target_name, field_name);

                                                                // Append after __decorate call
                                                                let span = expr.span;
                                                                self.replacements.push((
                                                                    span.end, span.end, assignment,
                                                                ));

                                                                {
                                                                    use std::io::Write;
                                                                    let mut f = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/linker_debug.log").unwrap();
                                                                    writeln!(f, "Linked Decorator {} on '{}' -> {}", d_name, target_name, field_name).unwrap();
                                                                }
                                                            }
                                                            Err(e) => {
                                                                self.errors.push(format!("Failed to parse metadata for {}: {}", d_name, e));
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Handle existing ɵɵngDeclare* calls (Partial Compilation)
                if n.starts_with("ɵɵngDeclare") && self.selector.supports_declaration(n) {
                    // It's a target!

                    // Args
                    if expr.arguments.len() > 0 {
                        // Assuming first arg is metadata object
                        if let Some(arg_expr) = expr.arguments[0].as_expression() {
                            // Create AstValue wrapper using OxcNode
                            // SAFETY: The expression resides in the allocator which lives for 'a.
                            // We are extending the lifetime of the reference from the visitor borrow to 'a.
                            let arg_expr_a: &'a ast::Expression<'a> =
                                unsafe { std::mem::transmute(arg_expr) };
                            let oxc_node = OxcNode::Expression(arg_expr_a);
                            let value = AstValue::new(oxc_node, &self.host);
                            match value.get_object() {
                                Ok(obj) => {
                                    let linker = self.selector.get_linker(n, "0.0.0", "0.0.0");
                                    let mut constant_pool = ConstantPool::new(false);

                                    // Link!
                                    let result_expr = linker.link_partial_declaration(
                                        &mut constant_pool,
                                        &obj,
                                        self.source_url,
                                        "0.0.0", // TODO: version
                                        None,
                                        Some(&self.alias_imports),
                                        // Try to find class metadata
                                        // Try to find class metadata
                                        {
                                            let mut matched_meta = None;
                                            if let Ok(type_val) = obj.get_value("type") {
                                                if let OxcNode::Expression(expr) = type_val.node {
                                                    if let ast::Expression::Identifier(id) = expr {
                                                        let t_name = id.name.as_str();
                                                        matched_meta = self
                                                            .class_metadata
                                                            .get(t_name)
                                                            .copied();
                                                    }
                                                }
                                            }

                                            if let Some(meta_expr) = matched_meta {
                                                let oxc_node = OxcNode::Expression(meta_expr);
                                                let val = AstValue::new(oxc_node, &self.host);
                                                val.get_object().ok()
                                            } else {
                                                None
                                            }
                                        }
                                        .as_ref(),
                                    );

                                    // Emit JS
                                    let js_code = if constant_pool.statements.is_empty() {
                                        self.emit_expression(&result_expr)
                                    } else {
                                        let stmts_code =
                                            self.emit_statements(constant_pool.statements);
                                        let expr_code = self.emit_expression(&result_expr);
                                        format!(
                                            "(function() {{ {} return {}; }})()",
                                            stmts_code, expr_code
                                        )
                                    };
                                    println!("[Rust Linker] Linked Partial Declaration {} -> {:.2000}...", n, js_code);

                                    let span = expr.span;
                                    let span = expr.span;
                                    self.replacements
                                        .push((span.start, span.end, js_code.clone()));
                                }
                                Err(e) => {
                                    self.errors
                                        .push(format!("Failed to parse metadata object: {}", e));
                                }
                            }
                        }
                    }
                }
            }

            // Continue visiting children (arguments)
            for arg in &expr.arguments {
                match arg {
                    ast::Argument::SpreadElement(s) => self.visit_expression(&s.argument),
                    target => {
                        if let Some(e) = target.as_expression() {
                            self.visit_expression(e);
                        }
                    }
                }
            }
        }
    }

    use std::io::Write;
    let mut log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/tmp/linker.log")
        .unwrap();
    writeln!(
        log_file,
        "Linking file: {} (original: {})",
        clean_filename, filename
    )
    .unwrap();
    // writeln!(log_file, "Source prefix: {:.100}", source_code).unwrap();

    // Pass 1: Collect Class Metadata
    let mut meta_visitor = ClassMetadataVisitor::new();
    meta_visitor.visit_program(&program);
    let class_metadata = meta_visitor.metadata;

    let mut visitor = LinkerVisitor::new(
        &source_code,
        imports,
        alias_imports,
        named_imports,
        class_metadata,
        &filename,
    );
    visitor.visit_program(&program);

    if !visitor.errors.is_empty() {
        writeln!(log_file, "Errors: {:?}", visitor.errors).unwrap();
        return Err(Error::new(
            Status::GenericFailure,
            visitor.errors.join("\n"),
        ));
    }

    if !visitor.logs.is_empty() {
        writeln!(log_file, "Logs:").unwrap();
        for log in visitor.logs {
            writeln!(log_file, "  - {}", log).unwrap();
        }
    }

    writeln!(
        log_file,
        "Replacements count: {}",
        visitor.replacements.len()
    )
    .unwrap();

    // Apply replacements
    // Sort replacements by start position descending to avoid index shifting issues
    visitor.replacements.sort_by(|a, b| b.0.cmp(&a.0));

    let mut result_code = source_code.clone();
    let had_replacements = !visitor.replacements.is_empty();

    for (start, end, new_text) in visitor.replacements {
        result_code.replace_range((start as usize)..(end as usize), &new_text);
    }

    if result_code.contains("MatFormField") && !filename.contains(".html") {
        let mut f = std::fs::File::create("/tmp/mat_form_field_linked.js").unwrap();
        std::io::Write::write_all(&mut f, result_code.as_bytes()).unwrap();
    }

    // Extract NgModule and directive metadata from linked code for later use
    // This enables dynamic resolution of NgModule exports during template compilation
    if had_replacements {
        let module_path = clean_filename;
        let (modules, directives) = crate::linker::metadata_extractor::extract_metadata_from_linked(
            module_path,
            &result_code,
        );
        if !modules.is_empty() || !directives.is_empty() {
            writeln!(
                log_file,
                "[Metadata] Extracted {} NgModules, {} directives from {}",
                modules.len(),
                directives.len(),
                module_path
            )
            .ok();
        }
    }

    Ok(result_code)
}
