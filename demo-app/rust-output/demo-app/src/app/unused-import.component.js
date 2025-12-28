import { Component } from '@angular/core';
import { CommonModule, NgIf, DecimalPipe } from '@angular/common';
import * as i1 from '@angular/common';
import * as i0 from '@angular/core';
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
  static ɵfac = function UnusedImportComponent_Factory(t) {
    return new (t || UnusedImportComponent)();
  };
  static ɵcmp = /* @__PURE__ */ i0.ɵɵdefineComponent({
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
    standalone: true,
    styles: [],
    encapsulation: 2,
    dependencies: [
      CommonModule,
      NgIf,
      i1.NgForOf,
      i1.NgClass,
      i1.NgStyle,
      i1.NgSwitch,
      i1.NgSwitchCase,
      i1.NgSwitchDefault,
      i1.UpperCasePipe,
      i1.LowerCasePipe,
      i1.JsonPipe,
      DecimalPipe,
      i1.DatePipe,
    ],
  });
}
