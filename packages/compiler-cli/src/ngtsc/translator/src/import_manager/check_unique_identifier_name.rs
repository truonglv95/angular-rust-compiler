use std::collections::HashSet;

/// Trait to allow checking if an identifier is used in a file.
/// This abstracts over the actual AST/SourceFile implementation.
pub trait IdentifierScope {
    fn is_identifier_used(&self, name: &str) -> bool;
    fn file_name(&self) -> &str;
}

pub struct UniqueIdentifierGenerator {
    generated_identifiers: HashSet<String>,
}

impl UniqueIdentifierGenerator {
    pub fn new() -> Self {
        Self {
            generated_identifiers: HashSet::new(),
        }
    }

    pub fn generate_unique_identifier<F: IdentifierScope + ?Sized>(
        &mut self,
        file: &F,
        symbol_name: &str,
    ) -> Option<String> {
        let is_generated = |name: &str| {
            self.generated_identifiers.contains(&format!("{}@@{}", file.file_name(), name))
        };

        // If the name is free in the file and hasn't been generated yet, just return None (use original)
        if !file.is_identifier_used(symbol_name) && !is_generated(symbol_name) {
            self.mark_as_generated(file, symbol_name);
            return None;
        }

        let mut counter = 1;
        let mut name = format!("{}_{}", symbol_name, counter);
        while file.is_identifier_used(&name) || is_generated(&name) {
            counter += 1;
            name = format!("{}_{}", symbol_name, counter);
        }

        self.mark_as_generated(file, &name);
        Some(name)
    }

    fn mark_as_generated<F: IdentifierScope + ?Sized>(&mut self, file: &F, name: &str) {
        self.generated_identifiers.insert(format!("{}@@{}", file.file_name(), name));
    }
}
