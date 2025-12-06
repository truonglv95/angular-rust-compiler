# Compiler Tests

Cấu trúc test tương tự như TypeScript version trong `angular/packages/compiler/test/`

## Cấu trúc

```
test/
├── util_spec.rs              # Tests cho util module
├── expression_parser/        # Tests cho expression parser
│   ├── lexer_spec.rs
│   ├── parser_spec.rs
│   └── ...
├── ml_parser/                # Tests cho ML parser
│   ├── lexer_spec.rs
│   ├── html_parser_spec.rs
│   └── ...
├── i18n/                     # Tests cho i18n
│   ├── digest_spec.rs
│   ├── i18n_parser_spec.rs
│   └── ...
└── ...
```

## Chạy tests

```bash
# Chạy tất cả tests
cargo test

# Chạy test cụ thể
cargo test util_spec

# Chạy test với output chi tiết
cargo test -- --nocapture
```

## Lưu ý

- Integration tests được định nghĩa trong `Cargo.toml` với `[[test]]`
- Mỗi test file tương ứng với một spec file trong TypeScript
- Tests sử dụng `angular_compiler` crate name để import modules

