//! ML Parser
//!
//! Corresponds to packages/compiler/src/ml_parser/parser.ts (1008 lines)
//! HTML/XML parser - converts tokens into AST
//!
//! NOTE: This is a skeletal implementation with complete structure.
//! Many methods have TODO markers for detailed implementation.

use crate::parse_util::{ParseError, ParseSourceSpan};
use super::ast::*;
use super::lexer::{tokenize, TokenizeOptions};
use super::tags::{merge_ns_and_name, get_ns_prefix, TagDefinition};
use super::tokens::*;

/// Node containers (can contain child nodes)
#[derive(Debug, Clone)]
pub enum NodeContainer {
    Element(Element),
    Block(Block),
    Component(Component),
}

/// Tree parsing error
#[derive(Debug, Clone)]
pub struct TreeError {
    pub element_name: Option<String>,
    pub span: ParseSourceSpan,
    pub msg: String,
}

impl TreeError {
    pub fn create(element_name: Option<String>, span: ParseSourceSpan, msg: String) -> Self {
        TreeError {
            element_name,
            span,
            msg,
        }
    }
}

/// Parse tree result
#[derive(Debug, Clone)]
pub struct ParseTreeResult {
    pub root_nodes: Vec<Node>,
    pub errors: Vec<ParseError>,
}

impl ParseTreeResult {
    pub fn new(root_nodes: Vec<Node>, errors: Vec<ParseError>) -> Self {
        ParseTreeResult { root_nodes, errors }
    }
}

/// Main parser class
pub struct Parser {
    pub get_tag_definition: fn(&str) -> &'static dyn TagDefinition,
}

/// Parser options
#[derive(Debug, Clone)]
pub struct ParseOptions {
    pub preserve_whitespaces: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        ParseOptions {
            preserve_whitespaces: false, // Match Angular default
        }
    }
}

impl Parser {
    pub fn new(get_tag_definition: fn(&str) -> &'static dyn TagDefinition) -> Self {
        Parser { get_tag_definition }
    }

    pub fn parse(&self, source: &str, url: &str, options: Option<TokenizeOptions>) -> ParseTreeResult {
        self.parse_with_options(source, url, options, ParseOptions::default())
    }

    pub fn parse_with_options(
        &self,
        source: &str,
        url: &str,
        tokenize_options: Option<TokenizeOptions>,
        parse_options: ParseOptions,
    ) -> ParseTreeResult {
        let opts = tokenize_options.unwrap_or_default();
        let tokenize_result = tokenize(source.to_string(), url.to_string(), self.get_tag_definition, opts);

        let tree_builder = TreeBuilder::new(
            tokenize_result.tokens,
            self.get_tag_definition,
            parse_options.preserve_whitespaces,
        );

        let mut all_errors = tokenize_result.errors;
        all_errors.extend(tree_builder.errors.into_iter().map(|e| {
            ParseError::new(e.span, e.msg)
        }));

        ParseTreeResult::new(tree_builder.root_nodes, all_errors)
    }
}

/// Internal tree builder
struct TreeBuilder {
    tokens: Vec<Token>,
    tag_definition_resolver: fn(&str) -> &'static dyn TagDefinition,
    index: isize,
    peek: Option<Token>,
    container_stack: Vec<NodeContainer>,
    root_nodes: Vec<Node>,
    errors: Vec<TreeError>,
    preserve_whitespaces: bool,
}

impl TreeBuilder {
    fn new(
        tokens: Vec<Token>,
        tag_definition_resolver: fn(&str) -> &'static dyn TagDefinition,
        preserve_whitespaces: bool,
    ) -> Self {
        let mut builder = TreeBuilder {
            tokens,
            tag_definition_resolver,
            index: -1,
            peek: None,
            container_stack: Vec::new(),
            root_nodes: Vec::new(),
            errors: Vec::new(),
            preserve_whitespaces,
        };

        builder.advance();
        builder.build();

        // Post-process: remove whitespace if not preserving
        if !preserve_whitespaces {
            builder.remove_whitespace_nodes();
        }

        // Always remove trailing whitespace-only text nodes from root (Angular behavior)
        while let Some(Node::Text(text)) = builder.root_nodes.last() {
            if text.value.trim().is_empty() {
                builder.root_nodes.pop();
            } else {
                break;
            }
        }

        builder
    }

    fn build(&mut self) {
        while let Some(ref token) = self.peek.clone() {
            match token {
                Token::TagOpenStart(_) | Token::IncompleteTagOpen(_) => {
                    let tok = self.advance().unwrap();
                    self.consume_element_start_tag(tok);
                }
                Token::TagClose(_) => {
                    let tok = self.advance().unwrap();
                    self.consume_element_end_tag(tok);
                }
                Token::CdataStart(_) => {
                    self.close_void_element();
                    let tok = self.advance().unwrap();
                    self.consume_cdata(tok);
                }
                Token::CommentStart(_) => {
                    self.close_void_element();
                    let tok = self.advance().unwrap();
                    self.consume_comment(tok);
                }
                Token::Text(_) | Token::Interpolation(_) => {
                    self.close_void_element();
                    let tok = self.advance().unwrap();
                    self.consume_text(tok);
                }
                Token::ExpansionFormStart(_) => {
                    let tok = self.advance().unwrap();
                    self.consume_expansion(tok);
                }
                Token::BlockOpenStart(_) => {
                    self.close_void_element();
                    let tok = self.advance().unwrap();
                    self.consume_block_open(tok);
                }
                Token::BlockClose(_) => {
                    self.close_void_element();
                    let tok = self.advance().unwrap();
                    self.consume_block_close(tok);
                }
                Token::IncompleteBlockOpen(_) => {
                    self.close_void_element();
                    self.advance();
                    // TODO: Handle incomplete block
                }
                Token::LetStart(_) => {
                    self.close_void_element();
                    let tok = self.advance().unwrap();
                    self.consume_let(tok);
                }
                Token::IncompleteLet(_) => {
                    // TODO: Implement _consumeIncompleteLet
                    self.close_void_element();
                    self.advance();
                }
                Token::ComponentOpenStart(_) | Token::IncompleteComponentOpen(_) => {
                    let tok = self.advance().unwrap();
                    self.consume_component_start_tag(tok);
                }
                Token::ComponentClose(_) => {
                    let tok = self.advance().unwrap();
                    self.consume_component_end_tag(tok);
                }
                Token::Eof(_) => break,
                _ => {
                    self.advance();
                }
            }
        }

        // Flush all remaining containers to root_nodes
        // NOTE: Only add unclosed error if it's truly unclosed
        while !self.container_stack.is_empty() {
            let container = self.container_stack.pop().unwrap();

            match container {
                NodeContainer::Element(el) => {
                    // Check if unclosed (missing end tag)
                    if el.end_source_span.is_none() && !el.is_self_closing && !el.is_void {
                        self.errors.push(TreeError::create(
                            Some(el.name.clone()),
                            el.source_span.clone(),
                            format!("Unclosed element \"{}\"", el.name),
                        ));
                    }
                    self.root_nodes.push(Node::Element(el));
                }
                NodeContainer::Block(block) => {
                    // Check if unclosed (missing closing brace)
                    if block.end_source_span.is_none() {
                        self.errors.push(TreeError::create(
                            Some(block.name.clone()),
                            block.source_span.clone(),
                            format!("Unclosed block \"@{}\"", block.name),
                        ));
                    }
                    self.root_nodes.push(Node::Block(block));
                }
                NodeContainer::Component(comp) => {
                    // Check if unclosed (missing closing tag)
                    if comp.end_source_span.is_none() && !comp.is_self_closing {
                        self.errors.push(TreeError::create(
                            Some(comp.full_name.clone()),
                            comp.source_span.clone(),
                            format!("Unclosed component \"{}\"", comp.full_name),
                        ));
                    }
                    self.root_nodes.push(Node::Component(comp));
                }
            }
        }
    }

    fn advance(&mut self) -> Option<Token> {
        let prev = self.peek.clone();
        self.index += 1;
        self.peek = self.tokens.get(self.index as usize).cloned();
        prev
    }

    fn advance_if(&mut self, token_type: TokenType) -> Option<Token> {
        if let Some(ref token) = self.peek {
            if std::mem::discriminant(token) == std::mem::discriminant(&create_token_discriminant(token_type)) {
                return self.advance();
            }
        }
        None
    }

    fn close_void_element(&mut self) {
        if let Some(container) = self.get_container() {
            if let NodeContainer::Element(el) = container {
                let tag_def = self.get_tag_definition(&el.name);
                if tag_def.is_void() {
                    self.container_stack.pop();
                }
            }
        }
    }

    fn consume_cdata(&mut self, _start_token: Token) {
        // CDATA is treated as text content
        if let Some(text_token) = self.advance_if(TokenType::Text) {
            self.consume_text(text_token);
        }
        self.advance_if(TokenType::CdataEnd);
    }

    fn consume_comment(&mut self, token: Token) {
        if let Token::CommentStart(comment_token) = token {
            let text = self.advance_if(TokenType::RawText);
            let end_token = self.advance_if(TokenType::CommentEnd);

            let value = text.and_then(|t| {
                match t {
                    Token::Text(txt) => Some(txt.parts.get(0)?.trim().to_string()),
                    _ => None
                }
            });

            let span = if let Some(Token::CommentEnd(_end)) = end_token {
                // Merge spans
                comment_token.source_span.clone()
            } else {
                comment_token.source_span.clone()
            };

            self.add_to_parent(Node::Comment(Comment::new(value, span)));
        }
    }

    fn consume_text(&mut self, start_token: Token) {
        let mut text = String::new();
        let mut tokens = vec![];
        let start_span = get_token_source_span(&start_token);

        // Add initial token
        text.push_str(&get_token_text(&start_token));
        tokens.push(start_token);

        // Collect consecutive text/interpolation/entity tokens
        while let Some(peek_token) = &self.peek {
            match peek_token {
                Token::Text(_) | Token::Interpolation(_) | Token::EncodedEntity(_) => {
                    if let Some(token) = self.advance() {
                        text.push_str(&get_token_text(&token));
                        tokens.push(token);
                    }
                }
                _ => break,
            }
        }

        if !text.is_empty() {
            let tokens_converted: Vec<InterpolatedTextToken> = tokens.into_iter()
                .filter_map(|t| match t {
                    Token::Text(txt) => Some(Token::Text(txt)),
                    Token::Interpolation(i) => Some(Token::Interpolation(i)),
                    Token::EncodedEntity(e) => Some(Token::EncodedEntity(e)),
                    _ => None
                })
                .collect();

            self.add_to_parent(Node::Text(Text::new(
                text,
                start_span,
                tokens_converted,
                None,
            )));
        }
    }

    fn consume_expansion(&mut self, token: Token) {
        if let Token::ExpansionFormStart(exp_token) = token {
            // Read switch value and type from a single Text token with 2 parts
            // Lexer creates one Text token with parts=["condition", "type"]
            let (switch_value, exp_type) = if let Some(Token::Text(txt)) = self.advance() {
                let condition = txt.parts.get(0).cloned().unwrap_or_default();
                let typ = txt.parts.get(1).cloned().unwrap_or_default();
                (condition, typ)
            } else {
                (String::new(), String::new())
            };

            let mut cases = Vec::new();

            // Read expansion cases
            while let Some(Token::ExpansionCaseValue(_)) = self.peek {
                if let Some(case) = self.parse_expansion_case() {
                    cases.push(case);
                } else {
                    return; // Error occurred
                }
            }

            // Check for closing }
            if !matches!(self.peek, Some(Token::ExpansionFormEnd(_))) {
                self.add_error("Invalid ICU message. Missing '}'.".to_string(), exp_token.source_span.clone());
                return;
            }

            let _end_span = get_token_source_span(self.peek.as_ref().unwrap());
            let source_span = exp_token.source_span.clone(); // TODO: Merge with _end_span
            let switch_span = exp_token.source_span.clone();

            let expansion = Expansion {
                switch_value,
                expansion_type: exp_type,
                cases,
                source_span,
                switch_value_source_span: switch_span,
                i18n: None,
            };

            self.add_to_parent(Node::Expansion(expansion));
            self.advance();
        }
    }

    fn parse_expansion_case(&mut self) -> Option<ExpansionCase> {
        if let Some(Token::ExpansionCaseValue(value_token)) = self.advance() {
            let value = value_token.parts.get(0).cloned().unwrap_or_default();

            // Check for {
            if !matches!(self.peek, Some(Token::ExpansionCaseExpStart(_))) {
                self.add_error("Invalid ICU message. Missing '{'.".to_string(), value_token.source_span.clone());
                return None;
            }

            let start_token = self.advance().unwrap();

            // Collect tokens until }
            let mut exp_tokens = Vec::new();
            let mut depth = 1;

            while depth > 0 {
                match &self.peek {
                    Some(Token::ExpansionCaseExpStart(_)) => {
                        depth += 1;
                        exp_tokens.push(self.advance().unwrap());
                    }
                    Some(Token::ExpansionCaseExpEnd(_)) => {
                        if depth == 1 {
                            break;
                        }
                        depth -= 1;
                        exp_tokens.push(self.advance().unwrap());
                    }
                    Some(Token::Eof(_)) => {
                        self.add_error("Invalid ICU message. Missing '}'.".to_string(), get_token_source_span(&start_token));
                        return None;
                    }
                    Some(_) => {
                        exp_tokens.push(self.advance().unwrap());
                    }
                    None => break,
                }
            }

            let end_token = self.advance().unwrap();

            // Parse expression tokens recursively
            exp_tokens.push(Token::Eof(EndOfFileToken {
                parts: vec![],
                source_span: get_token_source_span(&end_token),
            }));

            let mut case_parser = TreeBuilder::new(exp_tokens, self.tag_definition_resolver, self.preserve_whitespaces);
            case_parser.build();

            if !case_parser.errors.is_empty() {
                self.errors.extend(case_parser.errors);
                return None;
            }

            let source_span = value_token.source_span.clone();
            let value_span = value_token.source_span.clone();
            let exp_span = get_token_source_span(&start_token);

            Some(ExpansionCase {
                value,
                expression: case_parser.root_nodes,
                source_span,
                value_source_span: value_span,
                exp_source_span: exp_span,
            })
        } else {
            None
        }
    }

    fn consume_element_start_tag(&mut self, token: Token) {
        if let Token::TagOpenStart(start_token) = token {
            let mut attrs: Vec<Attribute> = Vec::new();
            let mut directives: Vec<Directive> = Vec::new();

            // Consume attributes and directives
            self.consume_attributes_and_directives(&mut attrs, &mut directives);

            // Get element name from token parts
            let full_name = if start_token.parts.len() > 1 {
                merge_ns_and_name(
                    if start_token.parts[0].is_empty() { None } else { Some(&start_token.parts[0]) },
                    &start_token.parts[1]
                )
            } else {
                start_token.parts.get(1).cloned().unwrap_or_default()
            };

            let tag_def = self.get_tag_definition(&full_name);
            let mut self_closing = false;

            // Check for self-closing or void tags
            if let Some(Token::TagOpenEndVoid(_)) = self.peek {
                self.advance();
                self_closing = true;

                // Validate self-closing
                if !tag_def.can_self_close() && get_ns_prefix(Some(&full_name)).is_none() && !tag_def.is_void() {
                    let msg = format!("Only void, custom and foreign elements can be self closed \"{}\"", full_name);
                    self.add_error(msg, start_token.source_span.clone());
                }
            } else if let Some(Token::TagOpenEnd(_)) = self.peek {
                self.advance();
                self_closing = false;
            }

            let span = start_token.source_span.clone();
            let start_span = span.clone();

            // Create Element node
            let element = Element {
                name: full_name.clone(),
                attrs,
                directives,
                children: Vec::new(),
                is_self_closing: self_closing,
                source_span: span.clone(),
                start_source_span: start_span,
                end_source_span: None,
                is_void: tag_def.is_void(),
                i18n: None,
            };

            // Push to container stack
            let is_closed_by_child = if let Some(parent) = self.get_container() {
                match parent {
                    NodeContainer::Element(parent_el) => {
                        self.get_tag_definition(&parent_el.name).is_closed_by_child(&full_name)
                    }
                    _ => false
                }
            } else {
                false
            };

            if is_closed_by_child {
                self.container_stack.pop();
            }

            // Handle self-closing and void elements
            if self_closing || tag_def.is_void() {
                // Self-closing (like <br/>) or void (like <br>) elements are completed immediately
                // Set end_source_span and add to parent directly
                let mut completed_element = element;
                completed_element.end_source_span = Some(span);
                self.add_to_parent(Node::Element(completed_element));
            } else {
                // Non-self-closing: Push to stack to collect children
                // Will be added to parent when end tag is processed
                self.container_stack.push(NodeContainer::Element(element.clone()));
            }
        }
    }

    fn consume_element_end_tag(&mut self, token: Token) {
        if let Token::TagClose(end_token) = token {
            // Get element name
            let full_name = if end_token.parts.len() > 1 {
                merge_ns_and_name(
                    if end_token.parts[0].is_empty() { None } else { Some(&end_token.parts[0]) },
                    &end_token.parts[1]
                )
            } else {
                end_token.parts.get(1).cloned().unwrap_or_default()
            };

            // Check if it's a void element
            let tag_def = self.get_tag_definition(&full_name);
            if tag_def.is_void() {
                let msg = format!("Void elements do not have end tags \"{}\"", full_name);
                self.add_error(msg, end_token.source_span.clone());
                return;
            }

            // Find and pop matching element from stack
            let mut found = false;
            for i in (0..self.container_stack.len()).rev() {
                if let NodeContainer::Element(el) = &self.container_stack[i] {
                    if el.name == full_name {
                        // Found matching element - pop it
                        let removed = self.container_stack.remove(i);

                        // Set end span and add to parent
                        if let NodeContainer::Element(mut el) = removed {
                            el.end_source_span = Some(end_token.source_span.clone());

                            // Add completed element to parent or root
                            if i > 0 {
                                // Has parent - add to parent's children
                                if let Some(parent) = self.container_stack.get_mut(i - 1) {
                                    match parent {
                                        NodeContainer::Element(parent_el) => parent_el.children.push(Node::Element(el)),
                                        NodeContainer::Block(parent_block) => parent_block.children.push(Node::Element(el)),
                                        NodeContainer::Component(parent_comp) => parent_comp.children.push(Node::Element(el)),
                                    }
                                }
                            } else {
                                // No parent - add to root
                                self.root_nodes.push(Node::Element(el));
                            }
                        }

                        found = true;
                        break;
                    }
                }
            }

            if !found {
                let msg = format!(
                    "Unexpected closing tag \"{}\". It may happen when the tag has already been closed by another tag.",
                    full_name
                );
                self.add_error(msg, end_token.source_span);
            }
        }
    }

    fn consume_attributes_and_directives(&mut self, attrs: &mut Vec<Attribute>, directives: &mut Vec<Directive>) {
        // Collect all attributes and directives
        while let Some(token) = &self.peek {
            match token {
                Token::AttrName(_) => {
                    if let Some(Token::AttrName(attr_token)) = self.advance() {
                        let attr = self.consume_attr(attr_token);
                        attrs.push(attr);
                    }
                }
                Token::DirectiveName(_) => {
                    if let Some(Token::DirectiveName(dir_token)) = self.advance() {
                        let directive = self.consume_directive(dir_token);
                        directives.push(directive);
                    }
                }
                _ => break,
            }
        }
    }

    fn consume_attr(&mut self, attr_name: AttributeNameToken) -> Attribute {
        let full_name = merge_ns_and_name(
            if attr_name.parts[0].is_empty() { None } else { Some(&attr_name.parts[0]) },
            &attr_name.parts.get(1).map(|s| s.as_str()).unwrap_or("")
        );
        let mut _attr_end = attr_name.source_span.clone();

        // Consume opening quote
        if let Some(Token::AttrQuote(_)) = self.peek {
            self.advance();
        }

        // Consume attribute value
        let mut value = String::new();
        let mut value_tokens = Vec::new();
        let mut value_span: Option<ParseSourceSpan> = None;

        while let Some(token) = &self.peek {
            match token {
                Token::AttrValueText(_) | Token::AttrValueInterpolation(_) | Token::EncodedEntity(_) => {
                    if let Some(val_token) = self.advance() {
                        let text = get_token_text(&val_token);
                        value.push_str(&text);

                        if value_span.is_none() {
                            value_span = Some(get_token_source_span(&val_token));
                        }
                        _attr_end = get_token_source_span(&val_token);

                        // Store token for interpolation tracking
                        value_tokens.push(val_token);
                    }
                }
                _ => break,
            }
        }

        // Consume closing quote
        if let Some(Token::AttrQuote(quote)) = self.advance_if(TokenType::AttrQuote) {
            _attr_end = quote.source_span;
        }

        Attribute {
            name: full_name,
            value,
            source_span: attr_name.source_span.clone(),
            key_span: Some(attr_name.source_span),
            value_span,
            value_tokens: if value_tokens.is_empty() { None } else { Some(value_tokens) },
            i18n: None,
        }
    }

    fn consume_directive(&mut self, name_token: DirectiveNameToken) -> Directive {
        let mut attributes = Vec::new();
        let mut end_source_span: Option<ParseSourceSpan> = None;

        // Check for DIRECTIVE_OPEN
        if let Some(Token::DirectiveOpen(_)) = self.peek {
            self.advance();

            // Collect attributes
            while let Some(Token::AttrName(attr_token)) = self.advance_if(TokenType::AttrName) {
                attributes.push(self.consume_attr(attr_token));
            }

            // Check for DIRECTIVE_CLOSE
            if let Some(Token::DirectiveClose(close)) = self.advance_if(TokenType::DirectiveClose) {
                end_source_span = Some(close.source_span);
            } else {
                self.add_error("Unterminated directive definition".to_string(), name_token.source_span.clone());
            }
        }

        let start_span = name_token.source_span.clone();
        let source_span = if let Some(ref _end) = end_source_span {
            start_span.clone() // TODO: Merge with end span
        } else {
            start_span.clone()
        };

        Directive {
            name: name_token.parts.get(0).cloned().unwrap_or_default(),
            attrs: attributes,
            source_span,
            start_source_span: start_span,
            end_source_span,
        }
    }

    fn consume_block_open(&mut self, token: Token) {
        if let Token::BlockOpenStart(block_token) = token {
            let mut parameters = Vec::new();

            // Collect block parameters
            while let Some(Token::BlockParameter(param)) = self.advance_if(TokenType::BlockParameter) {
                parameters.push(BlockParameter::new(
                    param.parts.get(0).cloned().unwrap_or_default(),
                    param.source_span,
                ));
            }

            // Check for BLOCK_OPEN_END
            if let Some(Token::BlockOpenEnd(_)) = self.peek {
                self.advance();
            }

            let span = block_token.source_span.clone();
            let name_span = span.clone();
            let start_span = span.clone();

            let block = Block {
                name: block_token.parts.get(0).cloned().unwrap_or_default(),
                parameters,
                children: Vec::new(),
                source_span: span,
                name_span,
                start_source_span: start_span,
                end_source_span: None,
                i18n: None,
            };

            // Don't add to parent yet - will add when block is closed
            // Just push to stack to collect children
            self.container_stack.push(NodeContainer::Block(block));
        }
    }

    fn consume_block_close(&mut self, token: Token) {
        if let Token::BlockClose(close_token) = token {
            // Pop block from stack
            let mut found = false;
            for i in (0..self.container_stack.len()).rev() {
                if let NodeContainer::Block(_) = &self.container_stack[i] {
                    // Found matching block - pop it
                    let removed = self.container_stack.remove(i);

                    // Set end span and add to parent
                    if let NodeContainer::Block(mut block) = removed {
                        block.end_source_span = Some(close_token.source_span.clone());

                        // Add completed block to parent or root
                        if i > 0 {
                            // Has parent - add to parent's children
                            if let Some(parent) = self.container_stack.get_mut(i - 1) {
                                match parent {
                                    NodeContainer::Element(parent_el) => parent_el.children.push(Node::Block(block)),
                                    NodeContainer::Block(parent_block) => parent_block.children.push(Node::Block(block)),
                                    NodeContainer::Component(parent_comp) => parent_comp.children.push(Node::Block(block)),
                                }
                            }
                        } else {
                            // No parent - add to root
                            self.root_nodes.push(Node::Block(block));
                        }
                    }

                    found = true;
                    break;
                }
            }

            if !found {
                let msg = "Unexpected closing block. The block may have been closed earlier. If you meant to write the } character, you should use the \"&#125;\" HTML entity instead.".to_string();
                self.add_error(msg, close_token.source_span);
            }
        }
    }

    fn consume_let(&mut self, token: Token) {
        if let Token::LetStart(let_token) = token {
            let name = let_token.parts.get(0).cloned().unwrap_or_default();

            // Consume LET_VALUE
            let value = if let Some(Token::LetValue(val)) = self.advance_if(TokenType::LetValue) {
                val.parts.get(0).cloned().unwrap_or_default()
            } else {
                String::new()
            };

            // Consume LET_END
            let end_span = if let Some(Token::LetEnd(end)) = self.advance_if(TokenType::LetEnd) {
                end.source_span
            } else {
                let_token.source_span.clone()
            };

            let decl = LetDeclaration {
                name,
                value,
                source_span: let_token.source_span.clone(),
                name_span: let_token.source_span.clone(),
                value_span: end_span,
            };

            self.add_to_parent(Node::LetDeclaration(decl));
        }
    }

    fn consume_component_start_tag(&mut self, token: Token) {
        if let Token::ComponentOpenStart(start_token) = token {
            let component_name = start_token.parts.get(0).cloned().unwrap_or_default();
            let mut attrs: Vec<Attribute> = Vec::new();
            let mut directives: Vec<Directive> = Vec::new();

            // Consume attributes and directives
            self.consume_attributes_and_directives(&mut attrs, &mut directives);

            // Determine tag name and full name
            let tag_name = start_token.parts.get(1).cloned();
            let full_name = component_name.clone();

            // Check for self-closing
            let self_closing = matches!(self.peek, Some(Token::ComponentOpenEndVoid(_)));
            if self_closing {
                self.advance();
            } else if matches!(self.peek, Some(Token::ComponentOpenEnd(_))) {
                self.advance();
            }

            let span = start_token.source_span.clone();
            let start_span = span.clone();

            let component = Component {
                component_name,
                tag_name,
                full_name: full_name.clone(),
                attrs,
                directives,
                children: Vec::new(),
                is_self_closing: self_closing,
                source_span: span.clone(),
                start_source_span: start_span,
                end_source_span: None,
                i18n: None,
            };

            if self_closing {
                // Self-closing - add directly to parent
                self.add_to_parent(Node::Component(component));
            } else {
                // Not self-closing - push to stack to collect children
                self.container_stack.push(NodeContainer::Component(component));
            }
        }
    }

    fn consume_component_end_tag(&mut self, token: Token) {
        if let Token::ComponentClose(end_token) = token {
            let full_name = end_token.parts.get(0).cloned().unwrap_or_default();

            // Find and pop matching component from stack
            let mut found = false;
            for i in (0..self.container_stack.len()).rev() {
                if let NodeContainer::Component(comp) = &self.container_stack[i] {
                    if comp.full_name == full_name {
                        // Found matching component - pop it
                        let removed = self.container_stack.remove(i);

                        // Set end span and add to parent
                        if let NodeContainer::Component(mut comp) = removed {
                            comp.end_source_span = Some(end_token.source_span.clone());

                            // Add completed component to parent or root
                            if i > 0 {
                                // Has parent - add to parent's children
                                if let Some(parent) = self.container_stack.get_mut(i - 1) {
                                    match parent {
                                        NodeContainer::Element(parent_el) => parent_el.children.push(Node::Component(comp)),
                                        NodeContainer::Block(parent_block) => parent_block.children.push(Node::Component(comp)),
                                        NodeContainer::Component(parent_comp) => parent_comp.children.push(Node::Component(comp)),
                                    }
                                }
                            } else {
                                // No parent - add to root
                                self.root_nodes.push(Node::Component(comp));
                            }
                        }

                        found = true;
                        break;
                    }
                }
            }

            if !found {
                let msg = format!("Unexpected closing component tag \"{}\"", full_name);
                self.add_error(msg, end_token.source_span);
            }
        }
    }

    fn add_error(&mut self, msg: String, span: ParseSourceSpan) {
        self.errors.push(TreeError::create(None, span, msg));
    }

    fn get_tag_definition(&self, tag_name: &str) -> &'static dyn TagDefinition {
        (self.tag_definition_resolver)(tag_name)
    }

    fn get_container(&self) -> Option<&NodeContainer> {
        self.container_stack.last()
    }

    fn add_to_parent(&mut self, node: Node) {
        if let Some(container) = self.container_stack.last_mut() {
            match container {
                NodeContainer::Element(el) => el.children.push(node),
                NodeContainer::Block(block) => block.children.push(node),
                NodeContainer::Component(comp) => comp.children.push(node),
            }
        } else {
            self.root_nodes.push(node);
        }
    }

    /// Remove whitespace-only text nodes (Angular default behavior)
    fn remove_whitespace_nodes(&mut self) {
        // Process root nodes
        let nodes = std::mem::take(&mut self.root_nodes);
        self.root_nodes = Self::remove_whitespace_from_list_static(nodes, 0);
    }

    fn remove_whitespace_from_list_static(nodes: Vec<Node>, depth: usize) -> Vec<Node> {
        if depth > 100 {
            return nodes; // Safety limit
        }

        nodes.into_iter().filter_map(|node| {
            match node {
                Node::Element(mut el) => {
                    // Recursively process children
                    el.children = Self::remove_whitespace_from_list_static(el.children, depth + 1);
                    Some(Node::Element(el))
                }
                Node::Block(mut block) => {
                    // Recursively process children
                    block.children = Self::remove_whitespace_from_list_static(block.children, depth + 1);
                    Some(Node::Block(block))
                }
                Node::Component(mut comp) => {
                    // Recursively process children
                    comp.children = Self::remove_whitespace_from_list_static(comp.children, depth + 1);
                    Some(Node::Component(comp))
                }
                Node::Text(text) => {
                    // Remove if whitespace-only
                    if text.value.trim().is_empty() {
                        None // Filter out
                    } else {
                        Some(Node::Text(text))
                    }
                }
                // Keep other node types as-is
                _ => Some(node),
            }
        }).collect()
    }

    fn push_container(&mut self, container: NodeContainer) {
        self.container_stack.push(container);
    }

    fn pop_container(&mut self) -> Option<NodeContainer> {
        self.container_stack.pop()
    }
}

// Helper functions

fn get_token_source_span(token: &Token) -> ParseSourceSpan {
    match token {
        Token::Text(t) => t.source_span.clone(),
        Token::Interpolation(t) => t.source_span.clone(),
        Token::TagOpenStart(t) => t.source_span.clone(),
        Token::TagClose(t) => t.source_span.clone(),
        Token::CommentStart(t) => t.source_span.clone(),
        Token::EncodedEntity(t) => t.source_span.clone(),
        _ => ParseSourceSpan::new(
            crate::parse_util::ParseLocation::new(
                crate::parse_util::ParseSourceFile::new(String::new(), String::new()),
                0, 0, 0
            ),
            crate::parse_util::ParseLocation::new(
                crate::parse_util::ParseSourceFile::new(String::new(), String::new()),
                0, 0, 0
            ),
        ),
    }
}

fn get_token_text(token: &Token) -> String {
    match token {
        Token::Text(t) => t.parts.join(""),
        Token::Interpolation(t) => t.parts.join(""),
        Token::EncodedEntity(t) => t.parts.get(0).cloned().unwrap_or_default(),
        Token::AttrValueText(t) => t.parts.join(""),
        Token::AttrValueInterpolation(t) => t.parts.join(""),
        _ => String::new(),
    }
}

// Helper to create discriminant for token type matching
fn create_token_discriminant(token_type: TokenType) -> Token {
    let dummy_span = ParseSourceSpan::new(
        crate::parse_util::ParseLocation::new(
            crate::parse_util::ParseSourceFile::new(String::new(), String::new()),
            0, 0, 0
        ),
        crate::parse_util::ParseLocation::new(
            crate::parse_util::ParseSourceFile::new(String::new(), String::new()),
            0, 0, 0
        ),
    );

    match token_type {
        TokenType::TagOpenStart => Token::TagOpenStart(TagOpenStartToken { parts: vec![], source_span: dummy_span }),
        TokenType::TagOpenEnd => Token::TagOpenEnd(TagOpenEndToken { parts: vec![], source_span: dummy_span }),
        TokenType::TagOpenEndVoid => Token::TagOpenEndVoid(TagOpenEndVoidToken { parts: vec![], source_span: dummy_span }),
        TokenType::TagClose => Token::TagClose(TagCloseToken { parts: vec![], source_span: dummy_span }),
        TokenType::AttrName => Token::AttrName(AttributeNameToken { parts: vec![], source_span: dummy_span }),
        TokenType::AttrQuote => Token::AttrQuote(AttributeQuoteToken { parts: vec![], source_span: dummy_span }),
        TokenType::AttrValueText => Token::AttrValueText(AttributeValueTextToken { parts: vec![], source_span: dummy_span }),
        TokenType::Text => Token::Text(TextToken { parts: vec![], source_span: dummy_span }),
        TokenType::CommentStart => Token::CommentStart(CommentStartToken { parts: vec![], source_span: dummy_span }),
        TokenType::CommentEnd => Token::CommentEnd(CommentEndToken { parts: vec![], source_span: dummy_span }),
        TokenType::CdataStart => Token::CdataStart(CdataStartToken { parts: vec![], source_span: dummy_span }),
        TokenType::CdataEnd => Token::CdataEnd(CdataEndToken { parts: vec![], source_span: dummy_span }),
        TokenType::BlockOpenStart => Token::BlockOpenStart(BlockOpenStartToken { parts: vec![], source_span: dummy_span }),
        TokenType::BlockOpenEnd => Token::BlockOpenEnd(BlockOpenEndToken { parts: vec![], source_span: dummy_span }),
        TokenType::BlockClose => Token::BlockClose(BlockCloseToken { parts: vec![], source_span: dummy_span }),
        TokenType::BlockParameter => Token::BlockParameter(BlockParameterToken { parts: vec![], source_span: dummy_span }),
        TokenType::LetStart => Token::LetStart(LetStartToken { parts: vec![], source_span: dummy_span }),
        TokenType::LetValue => Token::LetValue(LetValueToken { parts: vec![], source_span: dummy_span }),
        TokenType::LetEnd => Token::LetEnd(LetEndToken { parts: vec![], source_span: dummy_span }),
        TokenType::ExpansionFormStart => Token::ExpansionFormStart(ExpansionFormStartToken { parts: vec![], source_span: dummy_span }),
        TokenType::ExpansionFormEnd => Token::ExpansionFormEnd(ExpansionFormEndToken { parts: vec![], source_span: dummy_span }),
        TokenType::ExpansionCaseValue => Token::ExpansionCaseValue(ExpansionCaseValueToken { parts: vec![], source_span: dummy_span }),
        TokenType::ExpansionCaseExpStart => Token::ExpansionCaseExpStart(ExpansionCaseExpressionStartToken { parts: vec![], source_span: dummy_span }),
        TokenType::ExpansionCaseExpEnd => Token::ExpansionCaseExpEnd(ExpansionCaseExpressionEndToken { parts: vec![], source_span: dummy_span }),
        TokenType::Eof => Token::Eof(EndOfFileToken { parts: vec![], source_span: dummy_span }),
        _ => Token::Eof(EndOfFileToken { parts: vec![], source_span: dummy_span }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_error_creation() {
        let span = ParseSourceSpan::new(
            crate::parse_util::ParseLocation::new(
                crate::parse_util::ParseSourceFile::new("test".to_string(), "test.html".to_string()),
                0, 0, 0
            ),
            crate::parse_util::ParseLocation::new(
                crate::parse_util::ParseSourceFile::new("test".to_string(), "test.html".to_string()),
                0, 0, 0
            ),
        );

        let error = TreeError::create(Some("div".to_string()), span, "Test error".to_string());
        assert_eq!(error.element_name, Some("div".to_string()));
        assert_eq!(error.msg, "Test error");
    }

    #[test]
    fn test_parse_tree_result_creation() {
        let result = ParseTreeResult::new(vec![], vec![]);
        assert_eq!(result.root_nodes.len(), 0);
        assert_eq!(result.errors.len(), 0);
    }
}

