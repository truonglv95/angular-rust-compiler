import {
  Component,
  ViewChild,
  ViewChildren,
  ContentChild,
  ContentChildren,
  QueryList,
  ElementRef,
  AfterViewInit,
  AfterContentInit,
  Input,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatDividerModule } from '@angular/material/divider';
import { MatIconModule } from '@angular/material/icon';

@Component({
  selector: 'complex-test',
  standalone: true,
  imports: [CommonModule, MatCardModule, MatButtonModule, MatDividerModule, MatIconModule],
  templateUrl: './complex.html',
  styleUrls: ['./complex.css'],
})
export class ComplexTestComponent implements AfterViewInit, AfterContentInit {
  @Input() title = 'Complex Component Test';

  // Data for nested structures
  items = [
    {
      id: 1,
      name: 'Item 1',
      type: 'A',
      details: ['Detail 1.1', 'Detail 1.2'],
      active: true,
    },
    {
      id: 2,
      name: 'Item 2',
      type: 'B',
      details: ['Detail 2.1'],
      active: false,
    },
    {
      id: 3,
      name: 'Item 3',
      type: 'C',
      details: [],
      active: true,
    },
    {
      id: 4,
      name: 'Item 4',
      type: 'A',
      details: ['Detail 4.1', 'Detail 4.2', 'Detail 4.3'],
      active: true,
    },
  ];

  // Queries
  @ViewChild('header') header!: ElementRef<HTMLElement>;
  @ViewChildren('row') rows!: QueryList<ElementRef<HTMLElement>>;
  @ContentChild('projectedHeader') projectedHeader!: ElementRef<HTMLElement>;
  @ContentChildren('projectedItem') projectedItems!: QueryList<ElementRef<HTMLElement>>;

  toggleActive(item: any) {
    item.active = !item.active;
  }

  addItem() {
    this.items = [
      ...this.items,
      {
        id: this.items.length + 1,
        name: 'New Item',
        type: 'A',
        details: [],
        active: true,
      },
    ];
  }

  ngAfterViewInit() {
    console.log('ComplexTest: Header found:', !!this.header);
    console.log('ComplexTest: Rows count:', this.rows.length);
    this.rows.changes.subscribe((r) => console.log('ComplexTest: Rows changed:', r.length));
  }

  ngAfterContentInit() {
    console.log('ComplexTest: Projected header found:', !!this.projectedHeader);
    console.log('ComplexTest: Projected items count:', this.projectedItems.length);
  }
}

@Component({
  selector: 'complex-page',
  standalone: true,
  imports: [CommonModule, ComplexTestComponent, MatCardModule],
  template: `
    <div class="complex-page-container">
      <h1>Complex Page Wrapper</h1>
      <p>This page tests nested control flow and content projection.</p>

      <complex-test [title]="'Nested Directives Stress Test'">
        <!-- Projected Content -->
        <h3 #projectedHeader>Projected Header Content</h3>
        <div #projectedItem *ngFor="let i of [1, 2, 3]">Projected Item {{ i }}</div>
      </complex-test>
    </div>
  `,
  styles: [
    `
      .complex-page-container {
        padding: 20px;
      }
    `,
  ],
})
export class ComplexPageComponent {}
