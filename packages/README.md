# Angular Rust Compiler Packages

Cấu trúc package được tách thành 2 package riêng biệt, tương tự như Angular TypeScript:

## Cấu trúc

```
packages/
├── compiler/          # Core compiler library (@angular/compiler)
│   ├── Cargo.toml
│   ├── build.rs
│   └── src/           # Core compiler logic
│       ├── lib.rs
│       ├── ml_parser/ # HTML/XML parser
│       ├── expression_parser/ # Expression parser
│       ├── template/  # Template compilation pipeline
│       ├── i18n/      # i18n support
│       └── ...
│
└── compiler-cli/      # CLI tools (@angular/compiler-cli)
    ├── Cargo.toml
    ├── build.rs
    └── src/
        ├── lib.rs
        └── bin/
            ├── ngc.rs      # ngc command
            └── ng_xi18n.rs # ng-xi18n command
```

## Dependencies

- `compiler-cli` phụ thuộc vào `compiler` (tương tự như `@angular/compiler-cli` phụ thuộc vào `@angular/compiler`)

## Build

Từ root của workspace:

```bash
# Build tất cả packages
cargo build

# Build chỉ compiler
cargo build -p angular-compiler

# Build chỉ compiler-cli
cargo build -p angular-compiler-cli

# Build và chạy ngc
cargo run --bin ngc --package angular-compiler-cli
```

