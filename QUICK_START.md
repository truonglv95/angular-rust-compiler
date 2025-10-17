# ⚡ QUICK START: Test Rust Compiler Trong Angular App

## 🎯 Mục Tiêu

Test Rust compiler trong 1 Angular app thực trong **30 phút**!

---

## 📋 Prerequisites

- ✅ Rust compiler đã hoàn thành (examples/test.html parse OK)
- ✅ Node.js v18+
- ✅ Angular CLI: `npm install -g @angular/cli`
- ✅ Cargo và Rust toolchain

---

## 🚀 5 BƯỚC ĐƠN GIẢN

### BƯỚC 1: Hoàn Thiện NAPI Bindings (5 phút)

```bash
cd rust-compiler

# Check NAPI bindings hiện tại
cat src/lib.rs | grep "parse_template"

# Build
cargo build --release --features napi-bindings

# Check output
ls -lh target/release/*.node target/release/*.dylib 2>/dev/null | head -5
```

### BƯỚC 2: Tạo Node.js Entry Point (2 phút)

Tạo `index.js`:
```javascript
// index.js
try {
  // Try load native module
  const native = require('./target/release/libangular_rust_compiler.node') ||
                 require('./angular-rust-compiler.darwin-arm64.node') ||
                 require('./angular-rust-compiler.node');
  
  module.exports = native;
} catch (e) {
  console.error('Failed to load Rust compiler:', e.message);
  throw e;
}
```

Tạo `index.d.ts`:
```typescript
// index.d.ts
export interface ParsedTemplate {
  rootNodes: any[];
  errors: Array<{ msg: string }>;
}

export interface ParseOptions {
  tokenizeExpansionForms?: boolean;
  tokenizeBlocks?: boolean;
  tokenizeLet?: boolean;
}

export function parseTemplate(
  source: string,
  url: string,
  options?: ParseOptions
): ParsedTemplate;
```

### BƯỚC 3: Tạo Simple Angular Test App (5 phút)

```bash
# Về thư mục cha
cd ..

# Tạo simple Angular app
ng new rust-test-app --minimal --skip-git --routing=false
cd rust-test-app

# Tạo test component
cat > src/app/test.component.ts << 'EOF'
import { Component } from '@angular/core';

@Component({
  selector: 'app-test',
  standalone: true,
  template: `
    <div class="container">
      <h1>{{ title }}</h1>
      
      @if (showContent) {
        <p>Content is visible!</p>
      } @else {
        <p>Content is hidden!</p>
      }
      
      @for (item of items; track item.id) {
        <div class="item">
          <h3>{{ item.name }}</h3>
          <p>{{ item.description }}</p>
        </div>
      }
      
      <div class="messages">
        {messageCount, plural,
          =0 {No messages}
          =1 {One message}
          other {{{ messageCount }} messages}
        }
      </div>
    </div>
  `
})
export class TestComponent {
  title = 'Rust Compiler Test';
  showContent = true;
  messageCount = 5;
  items = [
    { id: 1, name: 'Item 1', description: 'First item' },
    { id: 2, name: 'Item 2', description: 'Second item' }
  ];
}
EOF
```

### BƯỚC 4: Link Rust Compiler (3 phút)

```bash
# Link Rust compiler package
npm link ../rust-compiler

# Verify link
ls -la node_modules/@angular-rust/compiler 2>/dev/null || \
  ls -la node_modules/angular-rust-compiler 2>/dev/null

# Test import
node -e "const rust = require('../rust-compiler'); console.log('Rust loaded:', typeof rust.parseTemplate);"
```

### BƯỚC 5: Test Build (5 phút)

```bash
# Build app
ng build

# Check if it compiled successfully
ls -lh dist/

# Serve app
ng serve &

# Open browser
open http://localhost:4200

# Stop server
kill %1
```

---

## 🧪 ADVANCED: Monkey-Patch Angular Compiler

### Create Patch File

```bash
cd rust-test-app

cat > src/enable-rust-parser.ts << 'EOF'
/**
 * 🦀 Rust Parser Monkey-Patch
 * 
 * Replaces Angular's TypeScript HtmlParser with Rust parser
 */

import { HtmlParser } from '@angular/compiler';

// Try to load Rust compiler
let rustCompiler: any;
try {
  rustCompiler = require('angular-rust-compiler');
  console.log('🦀 Rust compiler loaded successfully!');
} catch (e) {
  console.warn('⚠️ Rust compiler not available, using TypeScript parser');
}

if (rustCompiler && rustCompiler.parseTemplate) {
  const originalParse = HtmlParser.prototype.parse;
  
  HtmlParser.prototype.parse = function(source: string, url: string, options?: any) {
    try {
      // Use Rust parser
      const startTime = performance.now();
      const result = rustCompiler.parseTemplate(source, url, {
        tokenizeExpansionForms: options?.tokenizeExpansionForms ?? true,
        tokenizeBlocks: options?.tokenizeBlocks ?? true,
        tokenizeLet: options?.tokenizeLet ?? true,
      });
      const duration = performance.now() - startTime;
      
      console.log(`🦀 Rust parsed ${url} in ${duration.toFixed(2)}ms`);
      
      return {
        rootNodes: result.rootNodes || [],
        errors: result.errors || []
      };
    } catch (error) {
      // Fallback to TypeScript parser
      console.warn(`⚠️ Rust parser failed for ${url}, using TypeScript:`, error);
      return originalParse.call(this, source, url, options);
    }
  };
  
  console.log('✅ Angular HtmlParser replaced with Rust implementation');
}
EOF

# Import in main.ts (MUST be first import!)
cat > src/main.ts.new << 'EOF'
import './enable-rust-parser';  // ← Must be FIRST!

import { bootstrapApplication } from '@angular/platform-browser';
import { appConfig } from './app/app.config';
import { AppComponent } from './app/app.component';

bootstrapApplication(AppComponent, appConfig)
  .catch((err) => console.error(err));
EOF

mv src/main.ts.new src/main.ts
```

### Test The Patch

```bash
# Development build with console logs
ng serve

# Check browser console - should see:
# 🦀 Rust compiler loaded successfully!
# 🦀 Rust parsed app.component.html in 0.23ms
# ✅ Angular HtmlParser replaced with Rust implementation
```

---

## 📊 BENCHMARK COMPARISON

### Create Benchmark Script

```bash
cat > benchmark-parsers.mjs << 'EOF'
import { HtmlParser } from '@angular/compiler';
import * as fs from 'fs';

const rustCompiler = await import('./rust-compiler/index.js');

const templates = [
  { name: 'Simple', file: 'rust-compiler/examples/test.html' },
  // Add more templates
];

console.log('🏁 Parser Benchmark\n');

for (const test of templates) {
  const source = fs.readFileSync(test.file, 'utf-8');
  
  // TypeScript benchmark
  const tsParser = new HtmlParser();
  const tsStart = performance.now();
  for (let i = 0; i < 100; i++) {
    tsParser.parse(source, test.file, { tokenizeBlocks: true });
  }
  const tsTime = performance.now() - tsStart;
  
  // Rust benchmark
  const rustStart = performance.now();
  for (let i = 0; i < 100; i++) {
    rustCompiler.parseTemplate(source, test.file, { tokenizeBlocks: true });
  }
  const rustTime = performance.now() - rustStart;
  
  const speedup = ((tsTime - rustTime) / tsTime * 100).toFixed(1);
  
  console.log(`${test.name}:`);
  console.log(`  TypeScript: ${tsTime.toFixed(2)}ms`);
  console.log(`  Rust:       ${rustTime.toFixed(2)}ms`);
  console.log(`  Speedup:    ${speedup}% faster ⚡\n`);
}
EOF

node benchmark-parsers.mjs
```

---

## ✅ SUCCESS CHECKLIST

- [ ] NAPI module builds successfully
- [ ] Can import from Node.js: `require('./rust-compiler')`
- [ ] TypeScript types available: `index.d.ts`
- [ ] Angular app builds with Rust parser
- [ ] App runs in browser without errors
- [ ] Console shows "🦀 Rust parser" messages
- [ ] Performance improvement measured

---

## 🐛 TROUBLESHOOTING

### Issue: "Cannot find module"

```bash
# Check module exists
ls -la target/release/*.node

# Rebuild
cargo clean
cargo build --release --features napi-bindings

# Check index.js path
cat index.js | grep "require"
```

### Issue: "Undefined function parseTemplate"

```bash
# Check exports in lib.rs
grep "#\[napi\]" src/lib.rs

# Verify build
cargo build --release --features napi-bindings 2>&1 | grep "parseTemplate"
```

### Issue: Angular build fails

```bash
# Disable Rust parser
rm src/enable-rust-parser.ts

# Update main.ts
git checkout src/main.ts

# Build normally
ng build
```

---

## 📞 NEXT STEPS

Sau khi test thành công:

1. **Measure Performance**: Benchmark với nhiều templates
2. **Test Edge Cases**: Error handling, malformed HTML
3. **Optimize**: Profile và optimize bottlenecks
4. **Package**: Publish lên npm
5. **Document**: API docs và usage guide

---

🎉 **Bắt đầu ngay?** Run BƯỚC 1 và tôi sẽ giúp implement từng bước!

