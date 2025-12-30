# Angular Rust Compiler

High-performance Angular AOT compiler written in Rust, providing full static compilation of Angular components and directives.

## 🎯 Project Status

**Overall Progress**: ~85% Complete  
**Status**: ✅ **Functional** - Can compile Angular components to JavaScript

---

## 🚀 Quick Start

### Prerequisites

- Rust 1.70+
- Cargo

### Build & Run

```bash
# Build the compiler
cargo build -p angular-compiler-cli --release

# Compile an Angular project
cargo run -p angular-compiler-cli --bin ngc -- -p demo-app/tsconfig.json
```

Output files will be generated in `demo-app/rust-output/`.

---

## ✅ What's Working

### Core Compilation Features

| Feature                   | Status | Description                                          |
| ------------------------- | ------ | ---------------------------------------------------- |
| **Component Compilation** | ✅     | `@Component` decorator parsing and Ivy compilation   |
| **Directive Compilation** | ✅     | `@Directive` support with `ɵdir` emission            |
| **Template Parsing**      | ✅     | Full HTML/Angular template parsing                   |
| **Template Pipeline**     | ✅     | IR generation and optimization phases                |
| **Code Generation**       | ✅     | JavaScript emission with `ɵcmp` definitions          |
| **Inline Styles**         | ✅     | Style extraction and scoping (`[_ngcontent-%COMP%]`) |
| **External Templates**    | ✅     | `templateUrl` resolution                             |
| **External Styles**       | ✅     | `styleUrls` loading                                  |

### Angular Template Syntax

| Syntax                  | Status | Example                                   |
| ----------------------- | ------ | ----------------------------------------- |
| **Text Interpolation**  | ✅     | `{{ expression }}`                        |
| **Property Binding**    | ✅     | `[property]="value"`                      |
| **Event Binding**       | ✅     | `(click)="handler()"` with `ɵɵlistener()` |
| **Two-way Binding**     | ✅     | `[(ngModel)]="value"`                     |
| **@for Loops**          | ✅     | `@for (item of items; track item.id)`     |
| **@if Conditionals**    | ✅     | `@if (condition) { ... }`                 |
| **@switch**             | ✅     | `@switch (value) { @case ... }`           |
| **@let Declarations**   | ✅     | `@let name = expression`                  |
| **\*ngFor Directive**   | ✅     | `*ngFor="let item of items; index as i"`  |
| **\*ngIf Directive**    | ✅     | `*ngIf="condition"`                       |
| **ng-content**          | ✅     | Content projection                        |
| **Template References** | ✅     | `#ref`                                    |

### Metadata Extraction

| Property            | Status | Details                                         |
| ------------------- | ------ | ----------------------------------------------- |
| **selector**        | ✅     | Component/Directive selector                    |
| **inputs**          | ✅     | `@Input()` and `input()` signal                 |
| **outputs**         | ✅     | `@Output()` and `output()` signal               |
| **changeDetection** | ✅     | `ChangeDetectionStrategy.OnPush` (emits as `0`) |
| **standalone**      | ✅     | Standalone components                           |
| **imports**         | ✅     | Component imports                               |
| **hostDirectives**  | ⏳     | Pending                                         |

### Signal Support

| Signal Type        | Status |
| ------------------ | ------ |
| `input()`          | ✅     |
| `input.required()` | ✅     |
| `output()`         | ✅     |
| `signal()`         | ✅     |
| `computed()`       | ✅     |

---

## 📁 Project Structure

```
rust-compiler/
├── packages/
│   ├── compiler/                  # Core Angular compiler
│   │   ├── src/
│   │   │   ├── expression_parser/ # Expression parsing
│   │   │   ├── ml_parser/         # HTML/template parsing
│   │   │   ├── template/          # Template pipeline
│   │   │   │   └── pipeline/      # IR & optimization phases
│   │   │   ├── render3/           # Render3 code generation
│   │   │   ├── output/            # AST & JavaScript emission
│   │   │   └── shadow_css/        # CSS scoping
│   │   └── Cargo.toml
│   │
│   └── compiler-cli/              # CLI interface
│       ├── src/
│       │   ├── ngtsc/             # Angular TypeScript Compiler
│       │   │   ├── core/          # Core compilation logic
│       │   │   ├── metadata/      # Metadata extraction
│       │   │   └── annotations/   # Decorator handlers
│       │   └── main.rs            # CLI entry point
│       └── Cargo.toml
│
├── demo-app/                      # Example Angular app
│   ├── src/app/
│   │   ├── app.ts                 # Main component
│   │   └── app.html               # Template
│   ├── rust-output/               # Compiled output
│   └── tsconfig.json
│
└── Cargo.toml                     # Workspace config
```

---

## � Usage Examples

### Compile a Project

```bash
cargo run -p angular-compiler-cli --bin ngc -- -p path/to/tsconfig.json
```

### Example Input

```typescript
// app.ts
@Component({
  selector: "app-root",
  templateUrl: "./app.html",
  styleUrls: ["./app.css"],
  changeDetection: ChangeDetectionStrategy.OnPush,
  standalone: true,
  imports: [CommonModule],
})
export class App {
  title = input<string>("Hello");
  count = signal(0);
  items = signal([{ id: 1, name: "Item 1" }]);

  clicked = output<void>();
}
```

```html
<!-- app.html -->
<h1>{{ title() }}</h1>
@for (item of items(); track item.id; let idx = $index) {
<div>{{ idx + 1 }}. {{ item.name }}</div>
}
```

### Example Output

```javascript
// app.js
import * as i0 from "@angular/core";

function App_For_1_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, "div");
    i0.ɵɵtext(1);
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const item_r1 = ctx.$implicit;
    const $index_r2 = ctx.$index;
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate2("", $index_r2 + 1, ". ", item_r1.name, "");
  }
}

export class App {
  // ... class body
  static ɵcmp = i0.ɵɵdefineComponent({
    type: App,
    selectors: [["app-root"]],
    inputs: { title: [1, "title"] },
    outputs: { clicked: "clicked" },
    changeDetection: 0,
    standalone: true,
    // ...
  });
}
```

---

## 📈 Performance

| Metric | Rust Compiler | NGTSC (TypeScript) |
| ------ | ------------- | ------------------ |
| Full Build (123 files) | **~2.88s** | ~3.70s |
| Speedup | **~1.3x faster** | Baseline |
| Memory Usage | **Low (Native)** | High (V8 Heap) |

---

## 🧪 Running Tests

```bash
# All compiler tests
cargo test -p angular-compiler

# All compiler-cli tests
cargo test -p angular-compiler-cli

# Specific test suite
cargo test -p angular-compiler ml_parser
cargo test -p angular-compiler expression_parser
```

---

## 🛠️ Recent Improvements

### December 2024 (Latest)

#### Control Flow - `@for` Loops

| Feature | Status | Notes |
| ------- | ------ | ----- |
| Basic `@for` | ✅ | `@for (item of items; track item.id)` |
| Context Variables | ✅ | `$index`, `$count`, `$first`, `$last`, `$even`, `$odd` |
| Local Aliases | ✅ | `let idx = $index, first = $first` |
| Nested `@for` | ✅ | Multiple levels with correct context |
| `@empty` Block | ✅ | Fallback when collection is empty |
| Track By Function | ✅ | `track item.id` or `track trackFn($index, item)` |
| Event Handlers | ⚠️ | Works but has view hierarchy issue in nested conditionals |

**Known Issue**: `@for` inside `@if/@else` blocks may cause `nextContext()` to return undefined in event handlers. Investigating.

#### Control Flow - `@if` Conditionals

| Feature | Status | Notes |
| ------- | ------ | ----- |
| Basic `@if` | ✅ | `@if (condition) { ... }` |
| `@else` Block | ✅ | `@if (condition) { ... } @else { ... }` |
| `@else if` | ✅ | Chained conditionals |
| Template Chaining | ✅ | `conditionalCreate()` fluent API |

#### NgFor Directive (`*ngFor`)

- ✅ **Full Template Syntax**: `*ngFor="let item of items; index as i; first as isFirst; last as isLast; even as isEven; odd as isOdd; trackBy: trackFn"`
- ✅ **Context Variables**: All loop variables (`$implicit`, `index`, `count`, `first`, `last`, `even`, `odd`) correctly mapped
- ✅ **Source-Span Sorting**: Attributes in `consts` array are sorted by their source position in the template, ensuring parity with NGTSC
- ✅ **Nested Loops**: Full support for nested `*ngFor` with correct view restoration
- ✅ **Event Handlers Inside Loops**: Proper `getCurrentView()`/`restoreView()` emission for event handlers within loop bodies

#### NgIf Directive (`*ngIf`)

- ✅ **Basic Conditionals**: `*ngIf="condition"`
- ✅ **Else Template**: `*ngIf="condition; else elseTemplate"` with `TemplateRef` extraction
- ✅ **Then/Else Templates**: `*ngIf="condition; then thenTpl; else elseTpl"`
- ✅ **As Syntax**: `*ngIf="user$ | async as user"` with local variable binding
- ✅ **Nested NgIf**: Complex nested conditionals with proper context isolation

#### Two-Way Binding (`[(ngModel)]`)

- ✅ **`ɵɵtwoWayListener`**: Proper two-way listener emission
- ✅ **`ɵɵtwoWayProperty`**: Property binding for two-way data flow
- ✅ **`ɵɵtwoWayBindingSet`**: Setter with fallback assignment

#### Optimizations & Parity

- ✅ **RestoreView Optimization**: Single-usage context variables are inlined (`const user = restoreView().$implicit`) while multi-usage retains separate statements
- ✅ **Variable Naming Parity**: Global monotonic variable naming (`_r1`, `_r2`, ...) matches NGTSC exactly
- ✅ **Consts Array Parity**: Attribute ordering and marker values (`3` for bindings, `4` for template directives) match NGTSC
- ✅ **OnPush Optimization**: Root view listeners skip unnecessary `restoreView()`/`resetView()` calls for `OnPush` components
- ✅ **Listener Element Association**: Correctly associates listeners with their parent elements for proper `consts` array ordering

#### Other Improvements

- ✅ **Event Binding Emission**: Full support for `(click)="handler()"` with proper `ɵɵlistener()` emission
- ✅ **Rolldown/Vite Integration**: Angular Linker plugin for Rolldown bundler compatibility
- ✅ **Deterministic Build Output**: `HashMap` → `IndexMap` for consistent ordering of `inputs`, `outputs`
- ✅ **Signal Inputs/Outputs**: Full support for `input()`, `input.required()`, and `output()` signals

---

## 📝 Known Limitations

- **i18n**: Not fully implemented
- **Lazy Loading**: Deferred blocks partially supported
- **Animations**: Basic support only
- **View Encapsulation**: Only Emulated mode
- **Source Maps**: Not yet implemented

---

## 🎯 Roadmap

- [ ] Complete i18n support
- [ ] Full animation support
- [ ] Source map generation
- [ ] Angular CLI integration
- [ ] Incremental compilation
- [ ] Watch mode

---

## 📝 License

MIT - Same as Angular

---

**Built with ❤️ using Rust**
