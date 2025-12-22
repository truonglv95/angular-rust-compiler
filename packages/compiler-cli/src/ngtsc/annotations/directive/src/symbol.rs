// Directive Symbol
//
// Represents an Angular directive for semantic graph tracking.

use std::collections::HashSet;

/// Type parameters for a directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticTypeParameter {
    pub name: String,
    pub constraint: Option<String>,
}

/// Input or output mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputOrOutput {
    /// The class property name.
    pub class_property_name: String,
    /// The binding property name (template name).
    pub binding_property_name: String,
    /// Whether this is a signal-based input/output.
    pub is_signal: bool,
}

/// Input mapping with additional metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputMappingMeta {
    /// Base input/output info.
    pub base: InputOrOutput,
    /// Whether this input is required.
    pub required: bool,
}

/// Template guard metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateGuardMeta {
    pub input_name: String,
    pub guard_type: TemplateGuardType,
}

/// Type of template guard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateGuardType {
    Binding,
    Invocation,
}

/// Type check metadata for a directive.
#[derive(Debug, Clone, Default)]
pub struct DirectiveTypeCheckMeta {
    /// Whether the directive has ngTemplateContextGuard.
    pub has_ng_template_context_guard: bool,
    /// Template guards for inputs.
    pub ng_template_guards: Vec<TemplateGuardMeta>,
    /// Whether the directive is generic.
    pub is_generic: bool,
    /// Coerced input fields.
    pub coerced_input_fields: HashSet<String>,
    /// Restricted input fields.
    pub restricted_input_fields: HashSet<String>,
    /// String literal input fields.
    pub string_literal_input_fields: HashSet<String>,
    /// Undeclared input fields.
    pub undeclared_input_fields: HashSet<String>,
}

/// Represents an Angular directive.
#[derive(Debug, Clone)]
pub struct DirectiveSymbol {
    /// The class name.
    pub name: String,
    /// The directive selector.
    pub selector: Option<String>,
    /// Input mappings.
    pub inputs: Vec<InputMappingMeta>,
    /// Output mappings.
    pub outputs: Vec<InputOrOutput>,
    /// Export as names.
    pub export_as: Option<Vec<String>>,
    /// Type check metadata.
    pub type_check_meta: DirectiveTypeCheckMeta,
    /// Type parameters.
    pub type_parameters: Option<Vec<SemanticTypeParameter>>,
    /// Base class symbol, if any.
    pub base_class: Option<String>,
}

impl DirectiveSymbol {
    pub fn new(name: impl Into<String>, selector: Option<String>) -> Self {
        Self {
            name: name.into(),
            selector,
            inputs: Vec::new(),
            outputs: Vec::new(),
            export_as: None,
            type_check_meta: DirectiveTypeCheckMeta::default(),
            type_parameters: None,
            base_class: None,
        }
    }
    
    pub fn with_inputs(mut self, inputs: Vec<InputMappingMeta>) -> Self {
        self.inputs = inputs;
        self
    }
    
    pub fn with_outputs(mut self, outputs: Vec<InputOrOutput>) -> Self {
        self.outputs = outputs;
        self
    }
    
    pub fn with_export_as(mut self, export_as: Vec<String>) -> Self {
        self.export_as = Some(export_as);
        self
    }
    
    /// Check if the public API is affected compared to a previous symbol.
    pub fn is_public_api_affected(&self, previous: &DirectiveSymbol) -> bool {
        // Public API consists of: selector, input/output binding names, exportAs
        if self.selector != previous.selector {
            return true;
        }
        
        let self_input_names: Vec<_> = self.inputs.iter()
            .map(|i| &i.base.binding_property_name)
            .collect();
        let prev_input_names: Vec<_> = previous.inputs.iter()
            .map(|i| &i.base.binding_property_name)
            .collect();
        if self_input_names != prev_input_names {
            return true;
        }
        
        let self_output_names: Vec<_> = self.outputs.iter()
            .map(|o| &o.binding_property_name)
            .collect();
        let prev_output_names: Vec<_> = previous.outputs.iter()
            .map(|o| &o.binding_property_name)
            .collect();
        if self_output_names != prev_output_names {
            return true;
        }
        
        if self.export_as != previous.export_as {
            return true;
        }
        
        false
    }
    
    /// Check if the type check API is affected compared to a previous symbol.
    pub fn is_type_check_api_affected(&self, previous: &DirectiveSymbol) -> bool {
        if self.is_public_api_affected(previous) {
            return true;
        }
        
        // Check inputs/outputs in detail
        if self.inputs != previous.inputs || self.outputs != previous.outputs {
            return true;
        }
        
        // Check type parameters
        if self.type_parameters != previous.type_parameters {
            return true;
        }
        
        // Check type check metadata
        if !self.is_type_check_meta_equal(&previous.type_check_meta) {
            return true;
        }
        
        // Check base class
        if self.base_class != previous.base_class {
            return true;
        }
        
        false
    }
    
    fn is_type_check_meta_equal(&self, other: &DirectiveTypeCheckMeta) -> bool {
        let meta = &self.type_check_meta;
        
        meta.has_ng_template_context_guard == other.has_ng_template_context_guard
            && meta.is_generic == other.is_generic
            && meta.ng_template_guards == other.ng_template_guards
            && meta.coerced_input_fields == other.coerced_input_fields
            && meta.restricted_input_fields == other.restricted_input_fields
            && meta.string_literal_input_fields == other.string_literal_input_fields
            && meta.undeclared_input_fields == other.undeclared_input_fields
    }
}
