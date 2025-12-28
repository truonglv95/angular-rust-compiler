import { Component } from '@angular/core';
import * as i0 from '@angular/core';
export class HostBindingTest {
  isActive = true;
  color = 'red';
  onClick(event) {
    console.log('Host clicked', event);
    this.isActive = !this.isActive;
  }
  static ɵfac = function HostBindingTest_Factory(__ngFactoryType__) {
    return new (__ngFactoryType__ || HostBindingTest)();
  };
  static ɵcmp = /*@__PURE__*/ i0.ɵɵdefineComponent({
    type: HostBindingTest,
    selectors: [['app-host-binding-test']],
    hostAttrs: ['id', 'host-id'],
    hostVars: 4,
    hostBindings: function HostBindingTest_HostBindings(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵlistener('click', function HostBindingTest_click_HostBindingHandler($event) {
          return ctx.onClick($event);
        });
      }
      if (rf & 2) {
        i0.ɵɵstyleProp('color', ctx.color);
        i0.ɵɵclassProp('active', ctx.isActive);
      }
    },
    decls: 2,
    vars: 0,
    template: function HostBindingTest_Template(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵdomElementStart(0, 'p');
        i0.ɵɵtext(1, 'Host Binding Test');
        i0.ɵɵdomElementEnd();
      }
    },
    encapsulation: 2,
  });
}
(() => {
  (typeof ngDevMode === 'undefined' || ngDevMode) &&
    i0.ɵsetClassMetadata(
      HostBindingTest,
      [
        {
          type: Component,
          args: [
            {
              selector: 'app-host-binding-test',
              standalone: true,
              template: '<p>Host Binding Test</p>',
              host: {
                id: 'host-id',
                '[class.active]': 'isActive',
                '[style.color]': 'color',
                '(click)': 'onClick($event)',
              },
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
    i0.ɵsetClassDebugInfo(HostBindingTest, {
      className: 'HostBindingTest',
      filePath: 'src/app/src/components/host-binding-test/host-binding-test.ts',
      lineNumber: 14,
    });
})();
