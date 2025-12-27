import { Component } from '@angular/core';
import { CommonModule, NgIf, DecimalPipe } from '@angular/common';
import * as i0 from '@angular/core';
import * as i1 from '@angular/common';
function UnusedImportComponent_div_0_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div');
    i0.ɵɵtext(1);
    i0.ɵɵpipe(2, 'number');
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate1('Hello World ', i0.ɵɵpipeBind1(2, 1, 123.456));
  }
}
export class UnusedImportComponent {
  static ɵfac = function UnusedImportComponent_Factory(__ngFactoryType__) {
    return new (__ngFactoryType__ || UnusedImportComponent)();
  };
  static ɵcmp = /*@__PURE__*/ i0.ɵɵdefineComponent({
    type: UnusedImportComponent,
    selectors: [['app-unused-import']],
    decls: 1,
    vars: 1,
    consts: [[4, 'ngIf']],
    template: function UnusedImportComponent_Template(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵtemplate(0, UnusedImportComponent_div_0_Template, 3, 3, 'div', 0);
      }
      if (rf & 2) {
        i0.ɵɵproperty('ngIf', true);
      }
    },
    dependencies: [CommonModule, i1.NgIf, i1.DecimalPipe],
    encapsulation: 2,
  });
}
(() => {
  (typeof ngDevMode === 'undefined' || ngDevMode) &&
    i0.ɵsetClassMetadata(
      UnusedImportComponent,
      [
        {
          type: Component,
          args: [
            {
              selector: 'app-unused-import',
              standalone: true,
              imports: [CommonModule, NgIf, DecimalPipe],
              template: `<div *ngIf="true">Hello World {{ 123.456 | number }}</div>`,
            },
          ],
        },
      ],
      null,
      null,
    );
})();
(() => {
  (typeof ngDevMode === 'undefined' || ngDevMode) &&
    i0.ɵsetClassDebugInfo(UnusedImportComponent, {
      className: 'UnusedImportComponent',
      filePath: 'src/app/unused-import.component.ts',
      lineNumber: 10,
    });
})();
