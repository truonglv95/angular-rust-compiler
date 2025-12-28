import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import * as i0 from '@angular/core';
import * as i1 from '@angular/common';
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
  static ɵfac = function AnyTestComponent_Factory(__ngFactoryType__) {
    return new (__ngFactoryType__ || AnyTestComponent)();
  };
  static ɵcmp = /*@__PURE__*/ i0.ɵɵdefineComponent({
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
    dependencies: [CommonModule, i1.NgIf],
    styles: [
      '.test-case[_ngcontent-%COMP%] {\n        border: 1px solid #ccc;\n        padding: 10px;\n        margin: 10px 0;\n      }',
    ],
  });
}
(() => {
  (typeof ngDevMode === 'undefined' || ngDevMode) &&
    i0.ɵsetClassMetadata(
      AnyTestComponent,
      [
        {
          type: Component,
          args: [
            {
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
                '\n      .test-case {\n        border: 1px solid #ccc;\n        padding: 10px;\n        margin: 10px 0;\n      }\n    ',
              ],
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
    i0.ɵsetClassDebugInfo(AnyTestComponent, {
      className: 'AnyTestComponent',
      filePath: 'src/app/src/components/any-test/any-test.ts',
      lineNumber: 44,
    });
})();
