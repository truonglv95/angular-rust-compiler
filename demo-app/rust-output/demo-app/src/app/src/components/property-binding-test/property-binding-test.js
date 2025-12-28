import { NgIf, NgFor, NgClass, NgStyle } from '@angular/common';
import { Component } from '@angular/core';
import * as i0 from '@angular/core';
export class PropertyBindingTest {
  // Text bindings
  title = 'Property Binding Demo';
  description = 'Testing various property bindings';
  // Attribute bindings
  imageSrc = 'https://via.placeholder.com/150';
  imageAlt = 'Placeholder Image';
  linkHref = 'https://angular.io';
  // Boolean properties
  isDisabled = false;
  isReadonly = false;
  isHidden = false;
  // Style bindings
  textColor = 'blue';
  fontSize = 16;
  backgroundColor = '#f0f0f0';
  borderRadius = 8;
  // Class bindings
  isActive = false;
  isHighlighted = false;
  isPrimary = true;
  // Dynamic class object
  get dynamicClasses() {
    return {
      active: this.isActive,
      highlighted: this.isHighlighted,
      primary: this.isPrimary,
    };
  }
  // Dynamic style object
  get dynamicStyles() {
    return {
      color: this.textColor,
      'font-size': `${this.fontSize}px`,
      'background-color': this.backgroundColor,
      'border-radius': `${this.borderRadius}px`,
      padding: '10px',
    };
  }
  // Width/Height bindings
  boxWidth = 200;
  boxHeight = 100;
  // ARIA bindings
  ariaLabel = 'Interactive button';
  ariaExpanded = false;
  ariaDisabled = false;
  // Data attributes
  dataId = '12345';
  dataType = 'example';
  // Methods
  toggleDisabled() {
    this.isDisabled = !this.isDisabled;
  }
  toggleReadonly() {
    this.isReadonly = !this.isReadonly;
  }
  toggleHidden() {
    this.isHidden = !this.isHidden;
  }
  toggleActive() {
    this.isActive = !this.isActive;
  }
  toggleHighlighted() {
    this.isHighlighted = !this.isHighlighted;
  }
  togglePrimary() {
    this.isPrimary = !this.isPrimary;
  }
  setColor(color) {
    this.textColor = color;
  }
  increaseFontSize() {
    this.fontSize += 2;
  }
  decreaseFontSize() {
    if (this.fontSize > 8) {
      this.fontSize -= 2;
    }
  }
  toggleAriaExpanded() {
    this.ariaExpanded = !this.ariaExpanded;
  }
  static ɵfac = function PropertyBindingTest_Factory(t) {
    return new (t || PropertyBindingTest)();
  };
  static ɵcmp = /* @__PURE__ */ i0.ɵɵdefineComponent({
    type: PropertyBindingTest,
    selectors: [['app-property-binding-test']],
    decls: 83,
    vars: 42,
    consts: [
      [1, 'property-binding-test'],
      ['width', '100', 3, 'src', 'alt'],
      ['target', '_blank', 3, 'href'],
      [3, 'click'],
      ['type', 'text', 'placeholder', 'Disabled test', 'value', 'Test', 3, 'disabled'],
      ['type', 'text', 'placeholder', 'Readonly test', 'value', 'Readonly', 3, 'readonly'],
      [3, 'hidden'],
      [1, 'styled-box'],
      [3, 'ngStyle'],
      [1, 'box'],
      [1, 'box', 3, 'ngClass'],
    ],
    template: function PropertyBindingTest_Template(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵelementStart(0, 'div', 0)(1, 'h2');
        i0.ɵɵtext(2);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(3, 'p');
        i0.ɵɵtext(4);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(5, 'section')(6, 'h3');
        i0.ɵɵtext(7, '1. Basic Property Binding [property]');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(8, 'img', 1);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(9, 'a', 2);
        i0.ɵɵtext(10, 'Angular Website');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(11, 'section')(12, 'h3');
        i0.ɵɵtext(13, '2. Boolean Property Bindings');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(14, 'div')(15, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_15_listener() {
          return ctx.toggleDisabled();
        });
        i0.ɵɵtext(16, 'Toggle Disabled');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(17, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_17_listener() {
          return ctx.toggleReadonly();
        });
        i0.ɵɵtext(18, 'Toggle Readonly');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(19, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_19_listener() {
          return ctx.toggleHidden();
        });
        i0.ɵɵtext(20, 'Toggle Hidden');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(21, 'input', 4);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(22, 'input', 5);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(23, 'p', 6);
        i0.ɵɵtext(24, 'This paragraph can be hidden');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(25, 'p');
        i0.ɵɵtext(26);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(27, 'section')(28, 'h3');
        i0.ɵɵtext(29, '3. Style Binding [style.property]');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(30, 'div')(31, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_31_listener() {
          return ctx.setColor('red');
        });
        i0.ɵɵtext(32, 'Red');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(33, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_33_listener() {
          return ctx.setColor('green');
        });
        i0.ɵɵtext(34, 'Green');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(35, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_35_listener() {
          return ctx.setColor('blue');
        });
        i0.ɵɵtext(36, 'Blue');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(37, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_37_listener() {
          return ctx.increaseFontSize();
        });
        i0.ɵɵtext(38, 'Font +');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(39, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_39_listener() {
          return ctx.decreaseFontSize();
        });
        i0.ɵɵtext(40, 'Font -');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(41, 'p');
        i0.ɵɵtext(42, ' This text has dynamic color and font size ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(43, 'div', 7);
        i0.ɵɵtext(44, ' Dynamic box ');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(45, 'section')(46, 'h3');
        i0.ɵɵtext(47, '4. [ngStyle] Object Binding');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(48, 'p', 8);
        i0.ɵɵtext(49, 'This text uses ngStyle with an object');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(50, 'section')(51, 'h3');
        i0.ɵɵtext(52, '5. Class Binding [class.name]');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(53, 'div')(54, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_54_listener() {
          return ctx.toggleActive();
        });
        i0.ɵɵtext(55, 'Toggle Active');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(56, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_56_listener() {
          return ctx.toggleHighlighted();
        });
        i0.ɵɵtext(57, 'Toggle Highlighted');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(58, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_58_listener() {
          return ctx.togglePrimary();
        });
        i0.ɵɵtext(59, 'Toggle Primary');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(60, 'div', 9);
        i0.ɵɵtext(61, ' Class-bound box ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(62, 'p');
        i0.ɵɵtext(63);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(64, 'section')(65, 'h3');
        i0.ɵɵtext(66, '6. [ngClass] Object Binding');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(67, 'div', 10);
        i0.ɵɵtext(68, 'ngClass bound box');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(69, 'section')(70, 'h3');
        i0.ɵɵtext(71, '7. Attribute Binding [attr.name]');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(72, 'button', 3);
        i0.ɵɵlistener('click', function PropertyBindingTest_Template_button_click_72_listener() {
          return ctx.toggleAriaExpanded();
        });
        i0.ɵɵtext(73, ' ARIA Bound Button ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(74, 'p');
        i0.ɵɵtext(75);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(76, 'div');
        i0.ɵɵtext(77, 'Element with data attribute');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(78, 'section')(79, 'h3');
        i0.ɵɵtext(80, '8. Conditional Attribute (null removes)');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(81, 'button');
        i0.ɵɵtext(82, 'Conditionally Disabled');
        i0.ɵɵelementEnd()()();
      }
      if (rf & 2) {
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate(ctx.title);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate(ctx.description);
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('src', ctx.imageSrc, i0.ɵɵsanitizeUrl)('alt', ctx.imageAlt);
        i0.ɵɵadvance();
        i0.ɵɵproperty('href', ctx.linkHref, i0.ɵɵsanitizeUrl);
        i0.ɵɵadvance(12);
        i0.ɵɵproperty('disabled', ctx.isDisabled);
        i0.ɵɵadvance();
        i0.ɵɵproperty('readonly', ctx.isReadonly);
        i0.ɵɵadvance();
        i0.ɵɵproperty('hidden', ctx.isHidden);
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate3(
          'Disabled: ',
          ctx.isDisabled,
          ', Readonly: ',
          ctx.isReadonly,
          ', Hidden: ',
          ctx.isHidden,
        );
        i0.ɵɵadvance(15);
        i0.ɵɵstyleProp('color', ctx.textColor)('font-size', ctx.fontSize, 'px');
        i0.ɵɵadvance(2);
        i0.ɵɵstyleProp('width', ctx.boxWidth, 'px')('height', ctx.boxHeight, 'px')(
          'background-color',
          ctx.backgroundColor,
        )('border-radius', ctx.borderRadius, 'px');
        i0.ɵɵadvance(5);
        i0.ɵɵproperty('ngStyle', ctx.dynamicStyles);
        i0.ɵɵadvance(12);
        i0.ɵɵclassProp('active', ctx.isActive)('highlighted', ctx.isHighlighted)(
          'primary',
          ctx.isPrimary,
        );
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate3(
          'Active: ',
          ctx.isActive,
          ', Highlighted: ',
          ctx.isHighlighted,
          ', Primary: ',
          ctx.isPrimary,
        );
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('ngClass', ctx.dynamicClasses);
        i0.ɵɵadvance(5);
        i0.ɵɵattribute('aria-label', ctx.ariaLabel)('aria-expanded', ctx.ariaExpanded)(
          'aria-disabled',
          ctx.ariaDisabled,
        )('data-id', ctx.dataId)('data-type', ctx.dataType);
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate1('aria-expanded: ', ctx.ariaExpanded);
        i0.ɵɵadvance();
        i0.ɵɵattribute('data-custom', 'custom-value-' + ctx.dataId);
        i0.ɵɵadvance(5);
        i0.ɵɵattribute('disabled', ctx.isDisabled ? '' : null);
      }
    },
    standalone: true,
    styles: [
      '.property-binding-test[_ngcontent-%COMP%] {\n  padding: 20px;\n  font-family: Arial, sans-serif;\n}\n\nsection[_ngcontent-%COMP%] {\n  margin-bottom: 24px;\n  padding: 16px;\n  border: 1px solid #ddd;\n  border-radius: 8px;\n}\n\nh3[_ngcontent-%COMP%] {\n  margin-top: 0;\n  color: #333;\n}\n\nbutton[_ngcontent-%COMP%] {\n  margin-right: 8px;\n  margin-bottom: 8px;\n  padding: 8px 16px;\n  cursor: pointer;\n}\n\ninput[_ngcontent-%COMP%] {\n  display: block;\n  margin: 8px 0;\n  padding: 8px;\n}\n\n.styled-box[_ngcontent-%COMP%] {\n  display: flex;\n  align-items: center;\n  justify-content: center;\n  margin-top: 16px;\n  color: #333;\n}\n\n.box[_ngcontent-%COMP%] {\n  padding: 16px;\n  margin: 8px 0;\n  border: 2px solid #ccc;\n  transition: all 0.3s ease;\n}\n\n.box.active[_ngcontent-%COMP%] {\n  border-color: #2196f3;\n  background-color: #e3f2fd;\n}\n\n.box.highlighted[_ngcontent-%COMP%] {\n  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);\n}\n\n.box.primary[_ngcontent-%COMP%] {\n  font-weight: bold;\n  color: #1976d2;\n}\n\nimg[_ngcontent-%COMP%] {\n  margin-right: 16px;\n}',
    ],
    encapsulation: 0,
    dependencies: [NgIf, NgFor, NgClass, NgStyle],
  });
}
