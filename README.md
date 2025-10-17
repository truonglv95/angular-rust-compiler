# Angular Rust Compiler

High-performance Angular compiler written in Rust with Node.js bindings via NAPI-RS.

## 🎯 Project Status

**Overall Progress**: ~62% Complete

### ✅ Completed Modules (100%)

#### 1. expression_parser (100%)
- Lexer: 621 lines - Tokenize JavaScript expressions
- Parser: 869 lines - Parse to AST
- AST: Complete definitions
- Serializer: AST → string conversion

**Can parse**: All JavaScript expressions, ternary operators, pipes, property access, function calls

#### 2. schema (100%)
- DOM Element Schema: 188 HTML elements
- Security Schema: XSS protection
- Trusted Types: Sink validation

**Provides**: HTML validation, security context checking

#### 3. ml_parser (100%)
- **lexer.rs**: 1,172 lines - HTML/XML tokenization
- **parser.rs**: 948 lines - Build AST from tokens
- **entities.rs**: 2,178 lines - 2,125 HTML entities
- **tokens.rs**: 358 lines - Token definitions
- **tags.rs**, **ast.rs**, **defaults.rs**: Complete

**Can parse**: Any HTML document, any Angular template, all modern Angular syntax

### ⏳ Pending Modules

- template/pipeline: IR generation & optimization
- render3: Code generation
- output: Output formatting

## 🚀 Quick Start

### Build Compiler (without Node.js)

```bash
cd rust-compiler
cargo build --no-default-features --release
```

### Run Tests

```bash
# All unit tests
cargo test --lib --no-default-features

# Simple smoke test
cargo run --example simple_test --no-default-features

# Parse a template file
cargo run --example parse_template --no-default-features examples/test.html
```

### Build with NAPI (for Node.js)

```bash
cargo build --release
npm run build
```

## 📊 Features

### ✅ Working Now
- ✅ Tokenize HTML/Angular templates
- ✅ Parse to AST
- ✅ Expression parsing
- ✅ All Angular syntax (@if, @for, @switch, @let)
- ✅ Component syntax
- ✅ ICU messages (i18n)
- ✅ Entity decoding (2,125 entities)
- ✅ Error reporting

### ⏳ Coming Soon
- Code generation (render3)
- Optimization pipeline
- Source maps

## 📁 Project Structure

```
rust-compiler/
├── src/
│   ├── expression_parser/     # JavaScript expression parsing ✅
│   ├── ml_parser/              # HTML/Angular template parsing ✅
│   ├── schema/                 # HTML schema & validation ✅
│   ├── template/               # IR & optimization (pending)
│   ├── chars.rs                # Character constants ✅
│   ├── parse_util.rs           # Parsing utilities ✅
│   └── lib.rs                  # Main entry point
│
├── examples/
│   ├── simple_test.rs          # Quick smoke tests
│   ├── parse_template.rs       # Parse from file
│   └── test.html               # Sample Angular template
│
├── tests/
│   └── integration_test.rs     # Integration tests
│
└── Cargo.toml                  # Dependencies
```

## 🔬 Example Usage

```rust
use angular_rust_compiler::ml_parser::lexer::{tokenize, TokenizeOptions};
use angular_rust_compiler::ml_parser::parser::Parser;
use angular_rust_compiler::ml_parser::html_tags::get_html_tag_definition;
use angular_rust_compiler::ml_parser::tags::TagDefinition;

fn tag_def(name: &str) -> &'static dyn TagDefinition {
    get_html_tag_definition(name)
}

// Tokenize
let source = r#"<div class="app">{{ title }}</div>"#.to_string();
let result = tokenize(source, "test.html".to_string(), tag_def, TokenizeOptions::default());
println!("Tokens: {}", result.tokens.len());

// Parse to AST
let parser = Parser::new(tag_def);
let parse_result = parser.parse(&source, "test.html", None);
println!("AST Nodes: {}", parse_result.root_nodes.len());
```

## 📈 Performance

Expected improvements over TypeScript compiler:
- **Parsing**: 2-5x faster (zero-copy string processing)
- **Memory**: 30-50% less (stack allocation, no GC)
- **Consistent**: No GC pauses

## 🛠️ Development

### Prerequisites
- Rust 1.70+
- Node.js 16+ (for NAPI bindings)
- pnpm (for Angular build)

### Commands

```bash
# Test Rust code
cargo test --lib --no-default-features

# Build release
cargo build --no-default-features --release

# Run examples
cargo run --example simple_test --no-default-features

# With NAPI for Node.js
cargo build --release
npm run build
```

## 📝 License

MIT - Same as Angular

## 🎯 Next Steps

1. Implement template/pipeline module
2. Implement render3 code generation
3. Complete output module
4. Performance benchmarking
5. Integration with Angular CLI

---

**Current Status**: Parsing works perfectly! Code generation pending.

# angular-rust-compiler
