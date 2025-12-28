import { Component } from '@angular/core';
import * as i0 from '@angular/core';
export class HostBindingTest {
  isActive = true;
  color = 'red';
  onClick(event) {
    console.log('Host clicked', event);
    this.isActive = !this.isActive;
  }
  static ɵfac = function HostBindingTest_Factory(t) {
    return new (t || HostBindingTest)();
  };
  static ɵcmp = /* @__PURE__ */ i0.ɵɵdefineComponent({
    type: HostBindingTest,
    selectors: [['app-host-binding-test']],
    decls: 2,
    vars: 0,
    hostBindings: function HostBindingTest_Template_1(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵlistener('click', function HostBindingTest_Templateclick_HostBindingHandler($event) {
          return ctx.onClick($event);
        });
      }
      if (rf & 2) {
        i0.ɵɵclassProp('active', ctx.isActive);
        i0.ɵɵstyleProp('color', ctx.color);
      }
    },
    hostVars: 4,
    hostAttrs: ['id', 'host-id'],
    consts: [],
    template: function HostBindingTest_Template(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵdomElementStart(0, 'p');
        i0.ɵɵtext(1, 'Host Binding Test');
        i0.ɵɵdomElementEnd();
      }
    },
    standalone: true,
    styles: [],
    encapsulation: 2,
  });
}
