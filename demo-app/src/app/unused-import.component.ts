import { Component } from '@angular/core';
import { CommonModule, NgIf, DecimalPipe } from '@angular/common';

@Component({
  selector: 'app-unused-import',
  standalone: true,
  imports: [CommonModule, NgIf, DecimalPipe],
  template: `<div *ngIf="true">Hello World {{ 123.456 | number }}</div>`,
})
export class UnusedImportComponent {}
