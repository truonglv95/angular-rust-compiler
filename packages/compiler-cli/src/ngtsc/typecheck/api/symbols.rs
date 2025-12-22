// TypeCheck Symbols API
//
// Template symbol information.

/// A symbol in an Angular template.
#[derive(Debug, Clone)]
pub enum TemplateSymbol {
    /// A reference to a directive.
    Directive(DirectiveSymbolInfo),
    /// A reference to a pipe.
    Pipe(PipeSymbolInfo),
    /// A reference to a variable.
    Variable(VariableSymbolInfo),
    /// A DOM element.
    Element(ElementSymbolInfo),
    /// A reference.
    Reference(ReferenceSymbolInfo),
    /// An expression.
    Expression(ExpressionSymbolInfo),
}

/// Directive symbol information.
#[derive(Debug, Clone)]
pub struct DirectiveSymbolInfo {
    /// Directive name.
    pub name: String,
    /// Selector.
    pub selector: String,
    /// Whether this is a component.
    pub is_component: bool,
    /// Input bindings.
    pub inputs: Vec<InputBinding>,
    /// Output bindings.
    pub outputs: Vec<OutputBinding>,
}

/// Pipe symbol information.
#[derive(Debug, Clone)]
pub struct PipeSymbolInfo {
    /// Pipe name (as used in templates).
    pub name: String,
    /// Class name.
    pub class_name: String,
}

/// Variable symbol information.
#[derive(Debug, Clone)]
pub struct VariableSymbolInfo {
    /// Variable name.
    pub name: String,
    /// Variable kind.
    pub kind: VariableKind,
}

/// Kind of template variable.
#[derive(Debug, Clone, Copy)]
pub enum VariableKind {
    /// Loop variable (e.g., from *ngFor).
    Loop,
    /// Let declaration.
    Let,
    /// Context variable (e.g., $implicit).
    Context,
}

/// DOM element symbol.
#[derive(Debug, Clone)]
pub struct ElementSymbolInfo {
    /// Tag name.
    pub tag_name: String,
    /// Applied directives.
    pub directives: Vec<String>,
}

/// Template reference symbol.
#[derive(Debug, Clone)]
pub struct ReferenceSymbolInfo {
    /// Reference name (after #).
    pub name: String,
    /// Target type.
    pub target_type: String,
}

/// Expression symbol.
#[derive(Debug, Clone)]
pub struct ExpressionSymbolInfo {
    /// Expression text.
    pub expression: String,
    /// Inferred type.
    pub inferred_type: String,
}

/// Input binding info.
#[derive(Debug, Clone)]
pub struct InputBinding {
    /// Binding name.
    pub name: String,
    /// Input type.
    pub type_str: String,
    /// Whether required.
    pub required: bool,
}

/// Output binding info.
#[derive(Debug, Clone)]
pub struct OutputBinding {
    /// Event name.
    pub name: String,
    /// Event type.
    pub type_str: String,
}
