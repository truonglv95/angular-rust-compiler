import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-template-outlet-test',
  standalone: true,
  imports: [CommonModule],
  template: `
    <h2>Template Outlet Test</h2>

    <ng-template #tpl>
      <div style="background: #eee; padding: 10px; margin: 5px;">Content from template</div>
    </ng-template>

    <h3>1. Using ng-container *ngTemplateOutlet</h3>
    <ng-container *ngTemplateOutlet="tpl"></ng-container>

    <h3>2. Using ng-template [ngTemplateOutlet] (Pattern from PrimeNG)</h3>
    <!-- This mimics the pattern found in PrimeNG Button -->
    <ng-template [ngTemplateOutlet]="tpl"></ng-template>

    <h3>3. Using ng-template *ngTemplateOutlet (Structural on Template)</h3>
    <ng-template *ngTemplateOutlet="tpl"></ng-template>

    <h3>4. Using ng-template *ngIf (Structural on Template)</h3>
    <ng-template *ngIf="true">
      <div>Content from ngIf on ng-template</div>
    </ng-template>
  `,
})
export class TemplateOutletTestComponent {}
