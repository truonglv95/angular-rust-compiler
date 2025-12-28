import { Component } from '@angular/core';
import { NgIf, NgFor, JsonPipe } from '@angular/common';
import { FormsModule } from '@angular/forms';
import * as i1 from '@angular/forms';
import * as i0 from '@angular/core';
const _c0 = (a0, a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12) => ({
  name: a0,
  email: a1,
  age: a2,
  message: a3,
  country: a4,
  agreeTerms: a5,
  newsletter: a6,
  gender: a7,
  volume: a8,
  brightness: a9,
  date: a10,
  time: a11,
  color: a12,
});
function TwoWayBindingTest_option_48_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'option', 22);
    i0.ɵɵtext(1);
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const country_r1 = ctx.$implicit;
    i0.ɵɵproperty('value', country_r1.code);
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate1(
      // Select
      ' ',
      country_r1.name,
      ' ',
    );
  }
}
export class TwoWayBindingTest {
  // Basic two-way binding
  name = 'Angular';
  email = '';
  age = 25;
  // Textarea
  message = 'Hello World!';
  selectedCountry = 'vn';
  countries = [
    {
      code: 'vn',
      name: 'Vietnam',
    },
    {
      code: 'us',
      name: 'United States',
    },
    {
      code: 'jp',
      name: 'Japan',
    },
    {
      code: 'kr',
      name: 'Korea',
    },
  ];
  // Checkbox
  agreeTerms = false;
  receiveNewsletter = true;
  // Radio
  gender = 'other';
  // Range slider
  volume = 50;
  brightness = 75;
  // Date/Time
  selectedDate = '';
  selectedTime = '';
  // Color picker
  favoriteColor = '#3f51b5';
  // Computed
  get nameLength() {
    return this.name.length;
  }
  get volumeLabel() {
    if (this.volume < 30) return 'Low';
    if (this.volume < 70) return 'Medium';
    return 'High';
  }
  // Methods
  resetForm() {
    this.name = '';
    this.email = '';
    this.age = 25;
    this.message = '';
    this.selectedCountry = 'vn';
    this.agreeTerms = false;
    this.receiveNewsletter = true;
    this.gender = 'other';
    this.volume = 50;
    this.brightness = 75;
    this.selectedDate = '';
    this.selectedTime = '';
    this.favoriteColor = '#3f51b5';
  }
  submitForm() {
    console.log('Form submitted:', {
      name: this.name,
      email: this.email,
      age: this.age,
      message: this.message,
      country: this.selectedCountry,
      agreeTerms: this.agreeTerms,
      newsletter: this.receiveNewsletter,
      gender: this.gender,
    });
  }
  static ɵfac = function TwoWayBindingTest_Factory(t) {
    return new (t || TwoWayBindingTest)();
  };
  static ɵcmp = /* @__PURE__ */ i0.ɵɵdefineComponent({
    type: TwoWayBindingTest,
    selectors: [['app-two-way-binding-test']],
    decls: 125,
    vars: 54,
    consts: [
      [1, 'two-way-binding-test'],
      [1, 'form-actions'],
      [3, 'click'],
      [1, 'field'],
      ['type', 'text', 'placeholder', 'Enter name', 3, 'ngModelChange', 'ngModel'],
      ['type', 'email', 'placeholder', 'Enter email', 3, 'ngModelChange', 'ngModel'],
      ['type', 'number', 'min', '0', 'max', '120', 3, 'ngModelChange', 'ngModel'],
      ['rows', '4', 'cols', '50', 3, 'ngModelChange', 'ngModel'],
      [3, 'ngModelChange', 'ngModel'],
      [3, 'value', 4, 'ngFor', 'ngForOf'],
      ['type', 'checkbox', 3, 'ngModelChange', 'ngModel'],
      ['type', 'radio', 'value', 'male', 3, 'ngModelChange', 'ngModel'],
      ['type', 'radio', 'value', 'female', 3, 'ngModelChange', 'ngModel'],
      ['type', 'radio', 'value', 'other', 3, 'ngModelChange', 'ngModel'],
      ['type', 'range', 'min', '0', 'max', '100', 3, 'ngModelChange', 'ngModel'],
      [1, 'slider-preview'],
      [1, 'slider-preview', 'brightness'],
      ['type', 'date', 3, 'ngModelChange', 'ngModel'],
      ['type', 'time', 3, 'ngModelChange', 'ngModel'],
      ['type', 'color', 3, 'ngModelChange', 'ngModel'],
      [1, 'color-preview'],
      [1, 'summary'],
      [3, 'value'],
    ],
    template: function TwoWayBindingTest_Template(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵelementStart(0, 'div', 0)(1, 'h2');
        i0.ɵɵtext(2, 'Two-Way Binding Test Cases [(ngModel)]');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(3, 'div', 1)(4, 'button', 2);
        i0.ɵɵlistener('click', function TwoWayBindingTest_Template_button_click_4_listener() {
          return ctx.resetForm();
        });
        i0.ɵɵtext(5, 'Reset All');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(6, 'button', 2);
        i0.ɵɵlistener('click', function TwoWayBindingTest_Template_button_click_6_listener() {
          return ctx.submitForm();
        });
        i0.ɵɵtext(7, 'Submit Form');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(8, 'section')(9, 'h3');
        i0.ɵɵtext(10, '1. Text Input');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(11, 'div', 3)(12, 'label');
        i0.ɵɵtext(13, 'Name:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(14, 'input', 4);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.name, $event) || (ctx.name = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(15, 'p');
        i0.ɵɵtext(16);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(17, 'div', 3)(18, 'label');
        i0.ɵɵtext(19, 'Email:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(20, 'input', 5);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.email, $event) || (ctx.email = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(21, 'p');
        i0.ɵɵtext(22);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(23, 'section')(24, 'h3');
        i0.ɵɵtext(25, '2. Number Input');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(26, 'div', 3)(27, 'label');
        i0.ɵɵtext(28, 'Age:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(29, 'input', 6);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.age, $event) || (ctx.age = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(30, 'p');
        i0.ɵɵtext(31);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(32, 'section')(33, 'h3');
        i0.ɵɵtext(34, '3. Textarea');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(35, 'div', 3)(36, 'label');
        i0.ɵɵtext(37, 'Message:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(38, 'textarea', 7);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_textarea_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.message, $event) || (ctx.message = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(39, 'p');
        i0.ɵɵtext(40);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(41, 'section')(42, 'h3');
        i0.ɵɵtext(43, '4. Select Dropdown');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(44, 'div', 3)(45, 'label');
        i0.ɵɵtext(46, 'Country:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(47, 'select', 8);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_select_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.selectedCountry, $event) || (ctx.selectedCountry = $event);
            return $event;
          },
        );
        i0.ɵɵtemplate(48, TwoWayBindingTest_option_48_Template, 2, 2, 'option', 9);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(49, 'p');
        i0.ɵɵtext(50);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(51, 'section')(52, 'h3');
        i0.ɵɵtext(53, '5. Checkboxes');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(54, 'div', 3)(55, 'label')(56, 'input', 10);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.agreeTerms, $event) || (ctx.agreeTerms = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵtext(57, ' I agree to the terms and conditions ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(58, 'p');
        i0.ɵɵtext(59);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(60, 'div', 3)(61, 'label')(62, 'input', 10);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.receiveNewsletter, $event) ||
              (ctx.receiveNewsletter = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵtext(63, ' Receive newsletter ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(64, 'p');
        i0.ɵɵtext(65);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(66, 'section')(67, 'h3');
        i0.ɵɵtext(68, '6. Radio Buttons');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(69, 'div', 3)(70, 'label')(71, 'input', 11);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.gender, $event) || (ctx.gender = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵtext(72, ' Male ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(73, 'label')(74, 'input', 12);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.gender, $event) || (ctx.gender = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵtext(75, ' Female ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(76, 'label')(77, 'input', 13);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.gender, $event) || (ctx.gender = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵtext(78, ' Other ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(79, 'p');
        i0.ɵɵtext(80);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(81, 'section')(82, 'h3');
        i0.ɵɵtext(83, '7. Range Slider');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(84, 'div', 3)(85, 'label');
        i0.ɵɵtext(86);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(87, 'input', 14);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.volume, $event) || (ctx.volume = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(88, 'div', 15);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(89, 'div', 3)(90, 'label');
        i0.ɵɵtext(91);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(92, 'input', 14);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.brightness, $event) || (ctx.brightness = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(93, 'div', 16);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(94, 'section')(95, 'h3');
        i0.ɵɵtext(96, '8. Date and Time');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(97, 'div', 3)(98, 'label');
        i0.ɵɵtext(99, 'Date:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(100, 'input', 17);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.selectedDate, $event) || (ctx.selectedDate = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(101, 'p');
        i0.ɵɵtext(102);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(103, 'div', 3)(104, 'label');
        i0.ɵɵtext(105, 'Time:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(106, 'input', 18);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.selectedTime, $event) || (ctx.selectedTime = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(107, 'p');
        i0.ɵɵtext(108);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(109, 'section')(110, 'h3');
        i0.ɵɵtext(111, '9. Color Picker');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(112, 'div', 3)(113, 'label');
        i0.ɵɵtext(114, 'Favorite Color:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(115, 'input', 19);
        i0.ɵɵtwoWayListener(
          'ngModelChange',
          function TwoWayBindingTest_Template_input_ngModelChange_0_twoWayListener($event) {
            i0.ɵɵtwoWayBindingSet(ctx.favoriteColor, $event) || (ctx.favoriteColor = $event);
            return $event;
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(116, 'span', 20);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(117, 'p');
        i0.ɵɵtext(118);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(119, 'section', 21)(120, 'h3');
        i0.ɵɵtext(121, 'Form Summary');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(122, 'pre');
        i0.ɵɵtext(123);
        i0.ɵɵpipe(124, 'json');
        i0.ɵɵelementEnd()()();
      }
      if (rf & 2) {
        i0.ɵɵadvance(14);
        i0.ɵɵtwoWayProperty('ngModel', ctx.name);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate2('Current value: ', ctx.name, ' (', ctx.nameLength, ' characters)');
        i0.ɵɵadvance(4);
        i0.ɵɵtwoWayProperty('ngModel', ctx.email);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Current value: ', ctx.email);
        i0.ɵɵadvance(7);
        i0.ɵɵtwoWayProperty('ngModel', ctx.age);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Current value: ', ctx.age);
        i0.ɵɵadvance(7);
        i0.ɵɵtwoWayProperty('ngModel', ctx.message);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Current value: ', ctx.message);
        i0.ɵɵadvance(7);
        i0.ɵɵtwoWayProperty('ngModel', ctx.selectedCountry);
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngForOf', ctx.countries);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Selected code: ', ctx.selectedCountry);
        i0.ɵɵadvance(6);
        i0.ɵɵtwoWayProperty('ngModel', ctx.agreeTerms);
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate1('Agreed: ', ctx.agreeTerms);
        i0.ɵɵadvance(3);
        i0.ɵɵtwoWayProperty('ngModel', ctx.receiveNewsletter);
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate1('Subscribed: ', ctx.receiveNewsletter);
        i0.ɵɵadvance(6);
        i0.ɵɵtwoWayProperty('ngModel', ctx.gender);
        i0.ɵɵadvance(3);
        i0.ɵɵtwoWayProperty('ngModel', ctx.gender);
        i0.ɵɵadvance(3);
        i0.ɵɵtwoWayProperty('ngModel', ctx.gender);
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate1('Selected: ', ctx.gender);
        i0.ɵɵadvance(6);
        i0.ɵɵtextInterpolate2('Volume: ', ctx.volume, '% (', ctx.volumeLabel, ')');
        i0.ɵɵadvance();
        i0.ɵɵtwoWayProperty('ngModel', ctx.volume);
        i0.ɵɵadvance();
        i0.ɵɵstyleProp('width', ctx.volume, '%');
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate1('Brightness: ', ctx.brightness, '%');
        i0.ɵɵadvance();
        i0.ɵɵtwoWayProperty('ngModel', ctx.brightness);
        i0.ɵɵadvance();
        i0.ɵɵstyleProp('width', ctx.brightness, '%');
        i0.ɵɵadvance(7);
        i0.ɵɵtwoWayProperty('ngModel', ctx.selectedDate);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Selected: ', ctx.selectedDate);
        i0.ɵɵadvance(4);
        i0.ɵɵtwoWayProperty('ngModel', ctx.selectedTime);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Selected: ', ctx.selectedTime);
        i0.ɵɵadvance(7);
        i0.ɵɵtwoWayProperty('ngModel', ctx.favoriteColor);
        i0.ɵɵadvance();
        i0.ɵɵstyleProp('background-color', ctx.favoriteColor);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Selected: ', ctx.favoriteColor);
        i0.ɵɵadvance(5);
        i0.ɵɵtextInterpolate(
          i0.ɵɵpipeBind1(
            124,
            38,
            i0.ɵɵpureFunctionV(40, _c0, [
              ctx.name,
              ctx.email,
              ctx.age,
              ctx.message,
              ctx.selectedCountry,
              ctx.agreeTerms,
              ctx.receiveNewsletter,
              ctx.gender,
              ctx.volume,
              ctx.brightness,
              ctx.selectedDate,
              ctx.selectedTime,
              ctx.favoriteColor,
            ]),
          ),
        );
      }
    },
    standalone: true,
    styles: [
      ".two-way-binding-test[_ngcontent-%COMP%] {\n  padding: 20px;\n  font-family: Arial, sans-serif;\n}\n\nsection[_ngcontent-%COMP%] {\n  margin-bottom: 24px;\n  padding: 16px;\n  border: 1px solid #ddd;\n  border-radius: 8px;\n}\n\nh3[_ngcontent-%COMP%] {\n  margin-top: 0;\n  color: #333;\n}\n\n.form-actions[_ngcontent-%COMP%] {\n  margin-bottom: 20px;\n}\n\n.form-actions[_ngcontent-%COMP%] button[_ngcontent-%COMP%] {\n  margin-right: 10px;\n  padding: 10px 20px;\n  cursor: pointer;\n}\n\n.field[_ngcontent-%COMP%] {\n  margin-bottom: 16px;\n}\n\n.field[_ngcontent-%COMP%] label[_ngcontent-%COMP%] {\n  display: block;\n  margin-bottom: 4px;\n  font-weight: 500;\n}\n\n.field[_ngcontent-%COMP%] input[type='text'][_ngcontent-%COMP%], \n.field[_ngcontent-%COMP%] input[type='email'][_ngcontent-%COMP%], \n.field[_ngcontent-%COMP%] input[type='number'][_ngcontent-%COMP%], \n.field[_ngcontent-%COMP%] input[type='date'][_ngcontent-%COMP%], \n.field[_ngcontent-%COMP%] input[type='time'][_ngcontent-%COMP%], \n.field[_ngcontent-%COMP%] select[_ngcontent-%COMP%], \n.field[_ngcontent-%COMP%] textarea[_ngcontent-%COMP%] {\n  padding: 8px 12px;\n  border: 1px solid #ccc;\n  border-radius: 4px;\n  font-size: 14px;\n  width: 300px;\n  max-width: 100%;\n}\n\n.field[_ngcontent-%COMP%] input[type='range'][_ngcontent-%COMP%] {\n  width: 300px;\n}\n\n.slider-preview[_ngcontent-%COMP%] {\n  height: 8px;\n  background: linear-gradient(90deg, #4caf50, #8bc34a);\n  border-radius: 4px;\n  margin-top: 8px;\n  transition: width 0.2s ease;\n}\n\n.slider-preview.brightness[_ngcontent-%COMP%] {\n  background: linear-gradient(90deg, #ffc107, #ffeb3b);\n}\n\n.color-preview[_ngcontent-%COMP%] {\n  display: inline-block;\n  width: 30px;\n  height: 30px;\n  border-radius: 4px;\n  vertical-align: middle;\n  margin-left: 10px;\n  border: 1px solid #ccc;\n}\n\n.summary[_ngcontent-%COMP%] {\n  background-color: #f5f5f5;\n}\n\n.summary[_ngcontent-%COMP%] pre[_ngcontent-%COMP%] {\n  background-color: #fff;\n  padding: 16px;\n  border-radius: 4px;\n  overflow-x: auto;\n}\n\nlabel[_ngcontent-%COMP%] input[type='checkbox'][_ngcontent-%COMP%], \nlabel[_ngcontent-%COMP%] input[type='radio'][_ngcontent-%COMP%] {\n  margin-right: 8px;\n}",
    ],
    encapsulation: 0,
    dependencies: [
      NgIf,
      NgFor,
      FormsModule,
      i1.ɵNgNoValidate,
      i1.ɵNgSelectMultipleOption,
      i1.DefaultValueAccessor,
      i1.NumberValueAccessor,
      i1.RangeValueAccessor,
      i1.CheckboxControlValueAccessor,
      i1.SelectControlValueAccessor,
      i1.RadioControlValueAccessor,
      i1.NgControlStatus,
      i1.NgControlStatusGroup,
      i1.MinValidator,
      i1.MaxValidator,
      i1.NgModel,
      i1.NgForm,
      JsonPipe,
    ],
  });
}
