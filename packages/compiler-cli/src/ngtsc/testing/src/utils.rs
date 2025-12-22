// Test Utilities
//
// General testing utilities.

use std::collections::HashMap;

/// Options for test setup.
#[derive(Debug, Clone, Default)]
pub struct TestOptions {
    /// Enable strict mode.
    pub strict: bool,
    /// Enable Ivy.
    pub ivy: bool,
    /// Additional compiler options.
    pub options: HashMap<String, String>,
}

/// Test environment setup.
pub struct TestEnvironment {
    /// Test options.
    options: TestOptions,
    /// Files to compile.
    files: HashMap<String, String>,
}

impl TestEnvironment {
    pub fn new(options: TestOptions) -> Self {
        Self {
            options,
            files: HashMap::new(),
        }
    }
    
    /// Add a source file.
    pub fn add_file(&mut self, path: &str, content: &str) {
        self.files.insert(path.to_string(), content.to_string());
    }
    
    /// Add a component file.
    pub fn add_component(
        &mut self,
        name: &str,
        template: &str,
        styles: &str,
    ) {
        let content = format!(
            r#"import {{ Component }} from '@angular/core';

@Component({{
  selector: '{}',
  template: `{}`,
  styles: [`{}`]
}})
export class {}Component {{}}"#,
            to_kebab_case(name),
            template,
            styles,
            name
        );
        self.add_file(&format!("{}.component.ts", to_kebab_case(name)), &content);
    }
    
    /// Get files.
    pub fn get_files(&self) -> &HashMap<String, String> {
        &self.files
    }
}

impl Default for TestEnvironment {
    fn default() -> Self {
        Self::new(TestOptions::default())
    }
}

/// Convert to kebab-case.
fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
        }
        result.push(c.to_ascii_lowercase());
    }
    result
}

/// Create a simple test assertion.
pub fn expect_diagnostics(diagnostics: &[String], expected: &[&str]) -> bool {
    if diagnostics.len() != expected.len() {
        return false;
    }
    
    for (diag, exp) in diagnostics.iter().zip(expected.iter()) {
        if !diag.contains(exp) {
            return false;
        }
    }
    
    true
}

/// Create test source file content.
pub fn make_component_source(name: &str, template: &str) -> String {
    format!(
        r#"import {{ Component }} from '@angular/core';
@Component({{ selector: '{}', template: `{}` }})
export class {} {{}}"#,
        to_kebab_case(name),
        template,
        name
    )
}

/// Create test module source file content.
pub fn make_ng_module_source(name: &str, declarations: &[&str]) -> String {
    let decls = declarations.join(", ");
    format!(
        r#"import {{ NgModule }} from '@angular/core';
@NgModule({{ declarations: [{}] }})
export class {} {{}}"#,
        decls,
        name
    )
}
