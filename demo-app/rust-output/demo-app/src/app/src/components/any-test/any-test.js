import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import * as i1 from '@angular/common';
import * as i0 from '@angular/core';
function AnyTestComponent_p_10_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, 'User is active');
    i0.ɵɵelementEnd();
  }
}
export class AnyTestComponent {
  user = {
    name: 'Test User',
    age: 25,
  };
  static ɵfac = function AnyTestComponent_Factory(t) {
    return new (t || AnyTestComponent)();
  };
  static ɵcmp = /* @__PURE__ */ i0.ɵɵdefineComponent({
    type: AnyTestComponent,
    selectors: [['app-any-test']],
    decls: 16,
    vars: 3,
    consts: [
      [1, 'test-case'],
      [4, 'ngIf'],
      [3, 'title'],
    ],
    template: function AnyTestComponent_Template(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵelementStart(0, 'h2');
        i0.ɵɵtext(1, '$any Test Cases');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(2, 'div', 0)(3, 'h3');
        i0.ɵɵtext(4, 'Test 1: Simple $any');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(5, 'p');
        i0.ɵɵtext(6);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(7, 'div', 0)(8, 'h3');
        i0.ɵɵtext(9, 'Test 2: $any in condition');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(10, AnyTestComponent_p_10_Template, 2, 0, 'p', 1);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(11, 'div', 0)(12, 'h3');
        i0.ɵɵtext(13, 'Test 3: $any in property binding');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(14, 'div', 2);
        i0.ɵɵtext(15, 'Hover me');
        i0.ɵɵelementEnd()();
      }
      if (rf & 2) {
        i0.ɵɵadvance(6);
        i0.ɵɵtextInterpolate1('Value: ', ctx.user.unknownProperty);
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('ngIf', ctx.user.isActive);
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('title', ctx.user.customTitle);
      }
    },
    standalone: true,
    styles: [],
    encapsulation: 2,
    dependencies: [
      CommonModule,
      i1.NgIf,
      i1.NgForOf,
      i1.NgClass,
      i1.NgStyle,
      i1.NgSwitch,
      i1.NgSwitchCase,
      i1.NgSwitchDefault,
      i1.UpperCasePipe,
      i1.LowerCasePipe,
      i1.JsonPipe,
      i1.DecimalPipe,
      i1.DatePipe,
    ],
  });
}
