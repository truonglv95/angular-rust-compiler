# angular-rust-plugins

High-performance Angular linker and compiler plugins powered by Rust. Supports **Vite**, **esbuild**, **Rolldown**, and more bundlers coming soon.

This package bundles the Angular Rust binding - no additional dependencies needed!

## 🚀 Installation

```bash
npm install angular-rust-plugins
```

## 📖 Usage

### Full Angular Setup with Vite

```js
// vite.config.js
import { defineConfig } from "vite";
import { angularLinkerVitePlugin } from "angular-rust-plugins/linker/vite";
import { angularCompilerVitePlugin } from "angular-rust-plugins/compiler/vite";

export default defineConfig({
  plugins: [
    angularLinkerVitePlugin(), // Links @angular/* packages
    angularCompilerVitePlugin(), // Compiles your .ts files
  ],
});
```

### Linker Only (Vite)

```js
import { angularLinkerVitePlugin } from "angular-rust-plugins/linker/vite";

export default defineConfig({
  plugins: [angularLinkerVitePlugin()],
});
```

### Linker with Rolldown

```js
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
import { angularLinkerEsbuildPlugin } from "angular-rust-plugins/linker/esbuild";

esbuild.build({
  plugins: [angularLinkerEsbuildPlugin()],
});
```

## 📦 Package Exports

| Export Path                            | Description            |
| -------------------------------------- | ---------------------- |
| `angular-rust-plugins`                 | All plugins            |
| **Linker Plugins**                     |                        |
| `angular-rust-plugins/linker`          | All linker plugins     |
| `angular-rust-plugins/linker/vite`     | Vite linker plugin     |
| `angular-rust-plugins/linker/esbuild`  | esbuild linker plugin  |
| `angular-rust-plugins/linker/rolldown` | Rolldown linker plugin |
| **Compiler Plugins**                   |                        |
| `angular-rust-plugins/compiler`        | All compiler plugins   |
| `angular-rust-plugins/compiler/vite`   | Vite compiler plugin   |

## ⚙️ Options

### Linker Options

```ts
interface LinkerOptions {
  debug?: boolean; // Enable debug logging
  bindingPath?: string; // Custom path to binding
}
```

### Compiler Options

```ts
interface CompilerOptions {
  debug?: boolean; // Enable debug logging
  bindingPath?: string; // Custom path to binding
}
```

## 🔧 Development

```bash
# Build with current binding
npm run build

# Rebuild binding and plugin
npm run build:full
```

## ⚡ Performance

**2-5x faster** than TypeScript-based Angular compiler with lower memory usage.

## 📝 License

MIT

---

**Built with ❤️ using Rust**
