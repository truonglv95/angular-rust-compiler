import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';

interface User {
  name: string;
  age: number;
}

@Component({
  selector: 'app-any-test',
  standalone: true,
  imports: [CommonModule],
  template: `
    <h2>$any Test Cases</h2>

    <!-- Test 1: Simple $any usage -->
    <div class="test-case">
      <h3>Test 1: Simple $any</h3>
      <p>Value: {{ $any(user).unknownProperty }}</p>
    </div>

    <!-- Test 2: $any in expression -->
    <div class="test-case">
      <h3>Test 2: $any in condition</h3>
      <p *ngIf="$any(user).isActive">User is active</p>
    </div>

    <!-- Test 3: $any with property binding -->
    <div class="test-case">
      <h3>Test 3: $any in property binding</h3>
      <div [title]="$any(user).customTitle">Hover me</div>
    </div>
  `,
  styles: [
    `
      .test-case {
        border: 1px solid #ccc;
        padding: 10px;
        margin: 10px 0;
      }
    `,
  ],
})
export class AnyTestComponent {
  user: User = {
    name: 'Test User',
    age: 25,
  };
}
