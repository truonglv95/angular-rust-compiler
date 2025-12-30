const fs = require('fs');
const path = require('path');

const TARGET_DIR = path.resolve(__dirname, '../demo-app/src/app/benchmark');
const NUM_COMPONENTS = 5000;

console.log(`Generating ${NUM_COMPONENTS} complex components in ${TARGET_DIR}...`);

if (fs.existsSync(TARGET_DIR)) {
  fs.rmSync(TARGET_DIR, { recursive: true, force: true });
}
fs.mkdirSync(TARGET_DIR, { recursive: true });

let indexContent = '';

for (let i = 0; i < NUM_COMPONENTS; i++) {
  const componentName = `Benchmark${i}Component`;
  const fileName = `benchmark-${i}.component.ts`;
  const content = `
import { Component, Input, Output, EventEmitter } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';

@Component({
  selector: 'app-benchmark-${i}',
  standalone: true,
  imports: [CommonModule, FormsModule],
  template: \`
    <div class="benchmark-container">
      <h3>Benchmark ${i} Works!</h3>
      
      <!-- Interpolation & Input -->
      <p>Input Value: {{ data }}</p>
      
      <!-- Structural Directives (*ngIf, *ngFor) -->
      <div *ngIf="isVisible">
        <ul>
          <li *ngFor="let item of items; let idx = index">
            Item {{ idx }}: {{ item }}
          </li>
        </ul>
      </div>

      <!-- Two-way Binding -->
      <input [(ngModel)]="localState" placeholder="Type something...">
      <p>Current input: {{ localState }}</p>

      <!-- Event Binding -->
      <button (click)="toggleVisibility()">Toggle Visibility</button>
      <button (click)="emitEvent()">Emit Event</button>
    </div>
  \`
})
export class ${componentName} {
  @Input() data: string = 'Default Data';
  @Output() action = new EventEmitter<string>();

  isVisible = true;
  items = ['Item A', 'Item B', 'Item C', 'Item D', 'Item E'];
  localState = 'Initial State';

  toggleVisibility() {
    this.isVisible = !this.isVisible;
  }

  emitEvent() {
    this.action.emit(this.localState);
  }
}
`;
  fs.writeFileSync(path.join(TARGET_DIR, fileName), content);
  indexContent += `export * from './benchmark-${i}.component';\n`;
}

fs.writeFileSync(path.join(TARGET_DIR, 'index.ts'), indexContent);

console.log(`Done. Generated ${NUM_COMPONENTS} complex components.`);
