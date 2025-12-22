// Component Symbol
//
// Symbol representation for component semantic graph tracking.

use crate::ngtsc::annotations::directive::src::symbol::DirectiveSymbol;

/// Component symbol for incremental compilation tracking.
#[derive(Debug, Clone)]
pub struct ComponentSymbol {
    /// Base directive symbol.
    pub directive: DirectiveSymbol,
    /// Used directives in this component's template.
    pub used_directives: Vec<SemanticReference>,
    /// Used pipes in this component's template.
    pub used_pipes: Vec<SemanticReference>,
    /// Whether this component is remotely scoped by an NgModule.
    pub is_remotely_scoped: bool,
}

/// A reference for semantic graph tracking.
#[derive(Debug, Clone)]
pub struct SemanticReference {
    /// Symbol name.
    pub symbol: String,
}

impl ComponentSymbol {
    pub fn new(name: impl Into<String>, selector: Option<String>) -> Self {
        Self {
            directive: DirectiveSymbol::new(name, selector),
            used_directives: Vec::new(),
            used_pipes: Vec::new(),
            is_remotely_scoped: false,
        }
    }
    
    pub fn with_directive(directive: DirectiveSymbol) -> Self {
        Self {
            directive,
            used_directives: Vec::new(),
            used_pipes: Vec::new(),
            is_remotely_scoped: false,
        }
    }
    
    /// Check if emit is affected by changes.
    pub fn is_emit_affected(
        &self,
        previous: &ComponentSymbol,
        public_api_affected: &std::collections::HashSet<String>,
    ) -> bool {
        // Remote scope status changed
        if self.is_remotely_scoped != previous.is_remotely_scoped {
            return true;
        }
        
        // Directive list changed
        if self.used_directives.len() != previous.used_directives.len() {
            return true;
        }
        
        // Pipe list changed
        if self.used_pipes.len() != previous.used_pipes.len() {
            return true;
        }
        
        // Check if any used directive has public API affected
        for dir in &self.used_directives {
            if public_api_affected.contains(&dir.symbol) {
                return true;
            }
        }
        
        for pipe in &self.used_pipes {
            if public_api_affected.contains(&pipe.symbol) {
                return true;
            }
        }
        
        false
    }
    
    /// Check if type check block is affected.
    pub fn is_type_check_block_affected(
        &self,
        previous: &ComponentSymbol,
        type_check_api_affected: &std::collections::HashSet<String>,
    ) -> bool {
        // Directive list changed
        if self.used_directives.len() != previous.used_directives.len() {
            return true;
        }
        
        // Pipe list changed
        if self.used_pipes.len() != previous.used_pipes.len() {
            return true;
        }
        
        // Check if any used directive has type check API affected
        for dir in &self.used_directives {
            if type_check_api_affected.contains(&dir.symbol) {
                return true;
            }
        }
        
        for pipe in &self.used_pipes {
            if type_check_api_affected.contains(&pipe.symbol) {
                return true;
            }
        }
        
        false
    }
}
