# angular-rust-plugins

High-performance Angular linker and compiler plugins powered by Rust. Supports **Vite**, **esbuild**, **Rolldown**, and more bundlers coming soon.

This package bundles the Angular Rust binding - no additional dependencies needed!

## ✨ Features

- ✅ **Standard Directives**: Full support for `*ngIf` (with `else`, `then`, `as`) and `*ngFor` (with all context variables).
- ✅ **Template Parity**: 100% output parity with NGTSC for template instructions and `consts` array sorting.
- ✅ **Change Detection**: Optimized listener emission for `OnPush` components.
- ✅ **Signal Support**: Full support for modern Angular signals (`input()`, `output()`, `computed()`, etc.).
- ✅ **Blazing Fast**: Uses the Rust-based compiler for ~3-5x faster compilation compared to standard TypeScript-based compilation.

## 🚀 Installation

```bash
npm install angular-rust-plugins
```

## 📋 Quick Start

### 1. Create `index.html`

Create an `index.html` file at the root of your project:

```html
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <title>My Angular App</title>
  <base href="/">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <link rel="icon" type="image/x-icon" href="favicon.ico">
</head>
<body>
  <app-root></app-root>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

### 2. Create `vite.config.mjs`

```js
import { defineConfig } from "vite";
import { angularLinkerRolldownPlugin } from "angular-rust-plugins/linker/rolldown";
import { angularCompilerVitePlugin } from "angular-rust-plugins/compiler/vite";

export default defineConfig({
  plugins: [
    // Linker plugin - processes @angular/* packages in node_modules
    angularLinkerRolldownPlugin(),
    // Compiler plugin - compiles your .ts files with Angular decorators
    angularCompilerVitePlugin(),
  ],
  resolve: {
    extensions: [".ts", ".js", ".json"],
  },
  server: {
    port: 4200,
  },
  optimizeDeps: {
    // Exclude Angular packages from pre-bundling so linker plugin can process them
    exclude: [
      "@angular/core",
      "@angular/common",
      "@angular/platform-browser",
      "@angular/router",
      "@angular/forms",
    ],
    // Still include zone.js and rxjs which don't need linking
    include: ["zone.js", "rxjs", "rxjs/operators"],
  },
});
```

### 3. Run dev server

```bash
npx vite
```

## 📖 Complete Configuration Examples

### Full Angular Setup with Vite (Recommended)

```js
// vite.config.mjs
import { defineConfig } from "vite";
import { angularLinkerRolldownPlugin } from "angular-rust-plugins/linker/rolldown";
import { angularCompilerVitePlugin } from "angular-rust-plugins/compiler/vite";

export default defineConfig({
  plugins: [
    angularLinkerRolldownPlugin({ debug: false }),
    angularCompilerVitePlugin({ debug: false }),
  ],
  resolve: {
    extensions: [".ts", ".js", ".json"],
  },
  server: {
    port: 4200,
    open: true, // Open browser automatically
  },
  build: {
    target: "esnext",
    sourcemap: true,
  },
  optimizeDeps: {
    exclude: [
      "@angular/core",
      "@angular/common",
      "@angular/platform-browser",
      "@angular/router",
      "@angular/forms",
      "@angular/animations",
      "@angular/platform-browser-dynamic",
    ],
    include: ["zone.js", "rxjs", "rxjs/operators"],
  },
});
```

### Linker Only (Vite) - For existing Angular projects

If you only need the linker (for pre-AOT compiled Angular libraries):

```js
import { defineConfig } from "vite";
import { angularLinkerVitePlugin } from "angular-rust-plugins/linker/vite";

export default defineConfig({
  plugins: [angularLinkerVitePlugin()],
  optimizeDeps: {
    exclude: ["@angular/core", "@angular/common", "@angular/platform-browser"],
  },
});
```

### Linker with Rolldown (for Vite 6+)

```js
import { defineConfig } from "vite";
import { angularLinkerRolldownPlugin } from "angular-rust-plugins/linker/rolldown";

export default defineConfig({
  plugins: [angularLinkerRolldownPlugin()],
  optimizeDeps: {
    exclude: ["@angular/core", "@angular/common", "@angular/platform-browser"],
  },
});
```

### Linker with esbuild

```js
import esbuild from "esbuild";
import { angularLinkerEsbuildPlugin } from "angular-rust-plugins/linker/esbuild";

esbuild.build({
  entryPoints: ["src/main.ts"],
  bundle: true,
  outfile: "dist/bundle.js",
  plugins: [angularLinkerEsbuildPlugin()],
});
```

## 📦 Package Exports

| Export Path                            | Description                        |
| -------------------------------------- | ---------------------------------- |
| `angular-rust-plugins`                 | All plugins                        |
| **Linker Plugins**                     |                                    |
| `angular-rust-plugins/linker`          | All linker plugins                 |
| `angular-rust-plugins/linker/vite`     | Vite linker plugin (esbuild-based) |
| `angular-rust-plugins/linker/esbuild`  | esbuild linker plugin              |
| `angular-rust-plugins/linker/rolldown` | Rolldown linker plugin             |
| **Compiler Plugins**                   |                                    |
| `angular-rust-plugins/compiler`        | All compiler plugins               |
| `angular-rust-plugins/compiler/vite`   | Vite compiler plugin               |

## ⚙️ Options

### Linker Options

```ts
interface LinkerOptions {
  debug?: boolean; // Enable debug logging (default: false)
  bindingPath?: string; // Custom path to binding (optional)
}
```

### Compiler Options

```ts
interface CompilerOptions {
  debug?: boolean; // Enable debug logging (default: false)
  bindingPath?: string; // Custom path to binding (optional)
}
```

## 📁 Project Structure

Your Angular project should have this structure:

```
my-angular-app/
├── index.html           # Entry HTML file
├── vite.config.mjs      # Vite configuration
├── package.json
├── tsconfig.json
├── src/
│   ├── main.ts          # Bootstrap entry
│   ├── styles.css       # Global styles
│   └── app/
│       ├── app.ts       # Root component
│       ├── app.html     # Root template
│       └── app.config.ts # App configuration
└── public/
    └── favicon.ico
```

### Example `src/main.ts`

```ts
import { bootstrapApplication } from "@angular/platform-browser";
import { appConfig } from "./app/app.config";
import { App } from "./app/app";

bootstrapApplication(App, appConfig).catch((err) => console.error(err));
```

### Example `src/app/app.ts`

```ts
import { Component } from "@angular/core";

@Component({
  selector: "app-root",
  standalone: true,
  templateUrl: "./app.html",
  styleUrl: "./app.css",
})
export class App {
  title = "My Angular App";
}
```

## 🔧 Troubleshooting

### Angular packages not being linked

Make sure Angular packages are excluded from `optimizeDeps`:

```js
optimizeDeps: {
  exclude: [
    "@angular/core",
    "@angular/common",
    "@angular/platform-browser",
    // Add other @angular/* packages you use
  ],
}
```

### TypeScript compilation errors

Ensure your `tsconfig.json` includes:

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ES2022",
    "moduleResolution": "bundler",
    "experimentalDecorators": true,
    "emitDecoratorMetadata": true
  }
}
```

### HMR (Hot Module Replacement)

The compiler plugin includes HMR support for `.html` template files. When you modify a template, the corresponding component will be recompiled automatically.

## 💡 Advanced Directive Support

The compiler now achieves full feature parity with the standard Angular compiler for core structural directives:

### `*ngFor` Support
- **Full Context Access**: Support for `index`, `count`, `first`, `last`, `even`, `odd`.
- **Performance**: Automatic optimization of context access in event listeners.
- **Parity**: Identical `consts` array ordering through source-span sorting.

### `*ngIf` Support
- **Complex Templates**: Support for `else` and `then` templates with `TemplateRef` extraction.
- **As Syntax**: Support for the `as` local variable binding (e.g., `*ngIf="obs$ | async as val"`).
- **Context Isolation**: Correct variable scoping and restoration for nested conditionals.

## ⚡ Performance

**~1.3x faster** than TypeScript-based Angular compiler in full compilation tests.

| Metric               | Rust Compiler (Native) | NGTSC (Node.js) |
| -------------------- | ---------------------- | --------------- |
| Full Compile (Demo)  | **~2.88s**             | ~3.70s          |
| Speedup              | **~28%**               | Baseline        |
| Memory Overhead      | **Minimal**            | High            |

## 🌍 Platform & Framework Support

### Angular Compatibility

Fully compatible with **Angular 16.0.0+**. Older versions (View Engine) are not supported.

### Bundler Compatibility

| Bundler | Status | Notes |
|---------|--------|-------|
| **Vite** | ✅ Stable | Recommended for development |
| **Rolldown** | 🚀 Beta | Extremely fast, future default |
| **Esbuild** | ✅ Stable | Solid for production builds |
| **Webpack** | ⏳ Planned | Coming in v1.0 |

### Operating Systems

Pre-compiled binaries are provided for:
- **macOS**: x64, arm64
- **Windows**: x64, arm64
- **Linux**: x64, arm64 (glibc & musl/Alpine)

## 🔧 Development

```bash
# Build with current binding
npm run build

# Rebuild binding and plugin
npm run build:full
```

## 📝 License

MIT

---

**Built with ❤️ using Rust**
