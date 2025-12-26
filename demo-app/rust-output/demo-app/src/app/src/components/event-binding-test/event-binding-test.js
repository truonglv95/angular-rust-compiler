import { NgIf, NgFor } from '@angular/common';
import { Component } from '@angular/core';
import { FormsModule } from '@angular/forms';
import * as i0 from '@angular/core';
function EventBindingTest_p_16_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1);
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const ctx_r1 = i0.ɵɵnextContext();
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate1('Last click: ', ctx_r1.lastClickTime, '');
  }
}
function EventBindingTest_p_65_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1);
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const ctx_r2 = i0.ɵɵnextContext();
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate1('Selected: ', ctx_r2.selectedOption, '');
  }
}
function EventBindingTest_div_83_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div');
    i0.ɵɵtext(1, 'No events logged yet');
    i0.ɵɵelementEnd();
  }
}
function EventBindingTest_div_84_Template(rf, ctx) {
  if (rf & 1) {
    // Click handlers
    i0.ɵɵelementStart(0, 'div', 20)(1, 'span', 21);
    i0.ɵɵtext(2);
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(3, 'span', 22);
    i0.ɵɵtext(4);
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(5, 'span', 23);
    i0.ɵɵtext(6);
    i0.ɵɵelementEnd()();
  }
  if (rf & 2) {
    const log_r4 = ctx.$implicit;
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate1('[', log_r4.type, ']');
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate(log_r4.details);
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate(log_r4.timestamp.toLocaleTimeString());
  }
}
export class EventBindingTest {
  // Click events
  clickCount = 0;
  lastClickTime = '';
  // Mouse events
  mousePosition = {
    x: 0,
    y: 0,
  };
  isHovering = false;
  // Keyboard events
  inputValue = '';
  lastKeyPressed = '';
  keyPressCount = 0;
  // Focus events
  isFocused = false;
  // Form events
  formValue = '';
  selectedOption = '';
  checkboxValue = false;
  // Event log
  eventLog = [];
  maxLogEntries = 10;
  onClick() {
    this.clickCount++;
    this.lastClickTime = new Date().toLocaleTimeString();
    this.logEvent('click', `Button clicked (count: ${this.clickCount})`);
  }
  onDoubleClick() {
    this.logEvent('dblclick', 'Double click detected!');
  }
  onRightClick(event) {
    event.preventDefault();
    this.logEvent('contextmenu', 'Right click detected!');
  }
  // Mouse handlers
  onMouseEnter() {
    this.isHovering = true;
    this.logEvent('mouseenter', 'Mouse entered element');
  }
  onMouseLeave() {
    this.isHovering = false;
    this.logEvent('mouseleave', 'Mouse left element');
  }
  onMouseMove(event) {
    this.mousePosition = {
      x: event.clientX,
      y: event.clientY,
    };
  }
  onMouseDown(event) {
    this.logEvent('mousedown', `Mouse button ${event.button} down`);
  }
  onMouseUp(event) {
    this.logEvent('mouseup', `Mouse button ${event.button} up`);
  }
  // Keyboard handlers
  onKeyDown(event) {
    this.lastKeyPressed = event.key;
    this.keyPressCount++;
    this.logEvent('keydown', `Key pressed: ${event.key}`);
  }
  onKeyUp(event) {
    this.logEvent('keyup', `Key released: ${event.key}`);
  }
  onInput(event) {
    const target = event.target;
    this.inputValue = target.value;
  }
  onEnterKey() {
    this.logEvent('keydown.enter', 'Enter key pressed!');
  }
  onEscapeKey() {
    this.inputValue = '';
    this.logEvent('keydown.escape', 'Escape key - input cleared!');
  }
  // Focus handlers
  onFocus() {
    this.isFocused = true;
    this.logEvent('focus', 'Input focused');
  }
  onBlur() {
    this.isFocused = false;
    this.logEvent('blur', 'Input blurred');
  }
  // Form handlers
  onSubmit(event) {
    event.preventDefault();
    this.logEvent('submit', `Form submitted with value: ${this.formValue}`);
  }
  onSelectChange(event) {
    const target = event.target;
    this.selectedOption = target.value;
    this.logEvent('change', `Selected: ${target.value}`);
  }
  onCheckboxChange(event) {
    const target = event.target;
    this.checkboxValue = target.checked;
    this.logEvent('change', `Checkbox: ${target.checked ? 'checked' : 'unchecked'}`);
  }
  // Helper methods
  logEvent(type, details) {
    this.eventLog.unshift({
      type,
      timestamp: new Date(),
      details,
    });
    if (this.eventLog.length > this.maxLogEntries) {
      this.eventLog.pop();
    }
  }
  clearLog() {
    this.eventLog = [];
  }
  resetAll() {
    this.clickCount = 0;
    this.lastClickTime = '';
    this.inputValue = '';
    this.lastKeyPressed = '';
    this.keyPressCount = 0;
    this.formValue = '';
    this.selectedOption = '';
    this.checkboxValue = false;
    this.eventLog = [];
  }
  static ɵfac = function EventBindingTest_Factory(t) {
    return new (t || EventBindingTest)();
  };
  static ɵcmp = /* @__PURE__ */ i0.ɵɵdefineComponent({
    type: EventBindingTest,
    selectors: [['app-event-binding-test']],
    decls: 85,
    vars: 21,
    consts: [
      ['logBtn', ''],
      [1, 'event-binding-test'],
      [3, 'click'],
      [3, 'dblclick'],
      [3, 'contextmenu'],
      [4, 'ngIf'],
      [1, 'mouse-area', 3, 'mouseenter', 'mouseleave', 'mousemove', 'mousedown', 'mouseup'],
      [
        'type',
        'text',
        'placeholder',
        'Type something...',
        3,
        'value',
        'input',
        'keydown',
        'keyup',
        'keydown.enter',
        'keydown.escape',
      ],
      ['type', 'text', 'placeholder', 'Focus on me...', 3, 'focus', 'blur'],
      [3, 'submit'],
      [
        'type',
        'text',
        'placeholder',
        'Form input...',
        'name',
        'formInput',
        3,
        'ngModel',
        'ngModelChange',
      ],
      ['type', 'submit'],
      [3, 'change'],
      ['value', ''],
      ['value', 'option1'],
      ['value', 'option2'],
      ['value', 'option3'],
      ['type', 'checkbox', 3, 'checked', 'change'],
      [1, 'event-log'],
      ['class', 'log-entry', 4, 'ngFor', 'ngForOf'],
      [1, 'log-entry'],
      [1, 'log-type'],
      [1, 'log-details'],
      [1, 'log-time'],
    ],
    template: function EventBindingTest_Template(rf, ctx) {
      if (rf & 1) {
        const _r1 = i0.ɵɵgetCurrentView();
        i0.ɵɵelementStart(0, 'div', 1)(1, 'h2');
        i0.ɵɵtext(2, 'Event Binding Test Cases');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(3, 'button', 2);
        i0.ɵɵlistener('click', function EventBindingTest_Template_button_click_3_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.resetAll());
        });
        i0.ɵɵtext(4, 'Reset All');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(5, 'section')(6, 'h3');
        i0.ɵɵtext(7, '1. Click Events');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(8, 'button', 2);
        i0.ɵɵlistener('click', function EventBindingTest_Template_button_click_8_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onClick());
        });
        i0.ɵɵtext(9, 'Click Me');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(10, 'button', 3);
        i0.ɵɵlistener('dblclick', function EventBindingTest_Template_button_dblclick_10_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onDoubleClick());
        });
        i0.ɵɵtext(11, 'Double Click Me');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(12, 'button', 4);
        i0.ɵɵlistener(
          'contextmenu',
          function EventBindingTest_Template_button_contextmenu_12_listener() {
            i0.ɵɵrestoreView(_r1);
            return i0.ɵɵresetView(ctx.onRightClick(ctx.$event));
          },
        );
        i0.ɵɵtext(13, 'Right Click Me');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(14, 'p');
        i0.ɵɵtext(15);
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(16, EventBindingTest_p_16_Template, 2, 1, 'p', 5);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(17, 'section')(18, 'h3');
        i0.ɵɵtext(19, '2. Mouse Events');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(20, 'div', 6);
        i0.ɵɵlistener(
          'mouseenter',
          function EventBindingTest_Template_div_mouseenter_20_listener() {
            i0.ɵɵrestoreView(_r1);
            return i0.ɵɵresetView(ctx.onMouseEnter());
          },
        );
        i0.ɵɵlistener(
          'mouseleave',
          function EventBindingTest_Template_div_mouseleave_20_listener() {
            i0.ɵɵrestoreView(_r1);
            return i0.ɵɵresetView(ctx.onMouseLeave());
          },
        );
        i0.ɵɵlistener('mousemove', function EventBindingTest_Template_div_mousemove_20_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onMouseMove(ctx.$event));
        });
        i0.ɵɵlistener('mousedown', function EventBindingTest_Template_div_mousedown_20_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onMouseDown(ctx.$event));
        });
        i0.ɵɵlistener('mouseup', function EventBindingTest_Template_div_mouseup_20_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onMouseUp(ctx.$event));
        });
        i0.ɵɵelementStart(21, 'p');
        i0.ɵɵtext(22, 'Mouse over this area');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(23, 'p');
        i0.ɵɵtext(24);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(25, 'p');
        i0.ɵɵtext(26);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(27, 'section')(28, 'h3');
        i0.ɵɵtext(29, '3. Keyboard Events');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(30, 'input', 7);
        i0.ɵɵlistener('input', function EventBindingTest_Template_input_input_30_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onInput(ctx.$event));
        });
        i0.ɵɵlistener('keydown', function EventBindingTest_Template_input_keydown_30_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onKeyDown(ctx.$event));
        });
        i0.ɵɵlistener('keyup', function EventBindingTest_Template_input_keyup_30_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onKeyUp(ctx.$event));
        });
        i0.ɵɵlistener(
          'keydown.enter',
          function EventBindingTest_Template_input_keydown_enter_30_listener() {
            i0.ɵɵrestoreView(_r1);
            return i0.ɵɵresetView(ctx.onEnterKey());
          },
        );
        i0.ɵɵlistener(
          'keydown.escape',
          function EventBindingTest_Template_input_keydown_escape_30_listener() {
            i0.ɵɵrestoreView(_r1);
            return i0.ɵɵresetView(ctx.onEscapeKey());
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(31, 'p');
        i0.ɵɵtext(32);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(33, 'p');
        i0.ɵɵtext(34);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(35, 'p');
        i0.ɵɵtext(36);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(37, 'p')(38, 'em');
        i0.ɵɵtext(39, 'Hint: Press Enter to log, Escape to clear');
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(40, 'section')(41, 'h3');
        i0.ɵɵtext(42, '4. Focus Events');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(43, 'input', 8);
        i0.ɵɵlistener('focus', function EventBindingTest_Template_input_focus_43_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onFocus());
        });
        i0.ɵɵlistener('blur', function EventBindingTest_Template_input_blur_43_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onBlur());
        });
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(44, 'p');
        i0.ɵɵtext(45);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(46, 'section')(47, 'h3');
        i0.ɵɵtext(48, '5. Form Events');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(49, 'form', 9);
        i0.ɵɵlistener('submit', function EventBindingTest_Template_form_submit_49_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onSubmit(ctx.$event));
        });
        i0.ɵɵelementStart(50, 'input', 10);
        i0.ɵɵlistener(
          'ngModelChange',
          function EventBindingTest_Template_input_ngModelChange_50_listener() {
            i0.ɵɵrestoreView(_r1);
            return i0.ɵɵresetView(ctx.formValue);
          },
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(51, 'button', 11);
        i0.ɵɵtext(52, 'Submit');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(53, 'div')(54, 'label');
        i0.ɵɵtext(55, 'Select option:');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(56, 'select', 12);
        i0.ɵɵlistener('change', function EventBindingTest_Template_select_change_56_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onSelectChange(ctx.$event));
        });
        i0.ɵɵelementStart(57, 'option', 13);
        i0.ɵɵtext(58, '-- Select --');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(59, 'option', 14);
        i0.ɵɵtext(60, 'Option 1');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(61, 'option', 15);
        i0.ɵɵtext(62, 'Option 2');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(63, 'option', 16);
        i0.ɵɵtext(64, 'Option 3');
        i0.ɵɵelementEnd()();
        i0.ɵɵtemplate(65, EventBindingTest_p_65_Template, 2, 1, 'p', 5);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(66, 'div')(67, 'label')(68, 'input', 17);
        i0.ɵɵlistener('change', function EventBindingTest_Template_input_change_68_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.onCheckboxChange(ctx.$event));
        });
        i0.ɵɵelementEnd();
        i0.ɵɵtext(69, ' Check me ');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(70, 'p');
        i0.ɵɵtext(71);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(72, 'section')(73, 'h3');
        i0.ɵɵtext(74, '6. Using $event object');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(75, 'button', 2, 0);
        i0.ɵɵlistener('click', function EventBindingTest_Template_button_click_75_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(
            ctx.logEvent(
              'click',
              'Button at position: ' + ctx.$event.clientX + ',' + ctx.$event.clientY,
            ),
          );
        });
        i0.ɵɵtext(76, ' Click to log position ');
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(77, 'section')(78, 'h3');
        i0.ɵɵtext(79);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(80, 'button', 2);
        i0.ɵɵlistener('click', function EventBindingTest_Template_button_click_80_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.clearLog());
        });
        i0.ɵɵtext(81, 'Clear Log');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(82, 'div', 18);
        i0.ɵɵtemplate(83, EventBindingTest_div_83_Template, 2, 0, 'div', 5)(
          84,
          EventBindingTest_div_84_Template,
          7,
          3,
          'div',
          19,
        );
        i0.ɵɵelementEnd()()();
      }
      if (rf & 2) {
        i0.ɵɵadvance(15);
        i0.ɵɵtextInterpolate1('Click count: ', ctx.clickCount, '');
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', ctx.lastClickTime);
        i0.ɵɵadvance(4);
        i0.ɵɵclassProp('hovering', ctx.isHovering);
        i0.ɵɵadvance(4);
        i0.ɵɵtextInterpolate2('Position: X=', ctx.mousePosition.x, ', Y=', ctx.mousePosition.y, '');
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Hovering: ', ctx.isHovering ? 'Yes' : 'No', '');
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('value', ctx.inputValue);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Input value: ', ctx.inputValue, '');
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Last key: ', ctx.lastKeyPressed, '');
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Total key presses: ', ctx.keyPressCount, '');
        i0.ɵɵadvance(7);
        i0.ɵɵclassProp('focused', ctx.isFocused);
        i0.ɵɵadvance(2);
        i0.ɵɵtextInterpolate1('Input is ', ctx.isFocused ? 'focused' : 'not focused', '');
        i0.ɵɵadvance(5);
        i0.ɵɵadvance(15);
        i0.ɵɵproperty('ngIf', ctx.selectedOption);
        i0.ɵɵadvance(3);
        i0.ɵɵproperty('checked', ctx.checkboxValue);
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate1('Checkbox is: ', ctx.checkboxValue ? 'checked' : 'unchecked', '');
        i0.ɵɵadvance(8);
        i0.ɵɵtextInterpolate1('Event Log (Last ', ctx.maxLogEntries, ' events)');
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('ngIf', ctx.eventLog.length === 0);
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngForOf', ctx.eventLog);
      }
    },
    standalone: true,
    styles: [
      ".event-binding-test[_ngcontent-%COMP%] {\n  padding: 20px;\n  font-family: Arial, sans-serif;\n}\n\nsection[_ngcontent-%COMP%] {\n  margin-bottom: 24px;\n  padding: 16px;\n  border: 1px solid #ddd;\n  border-radius: 8px;\n}\n\nh3[_ngcontent-%COMP%] {\n  margin-top: 0;\n  color: #333;\n}\n\nbutton[_ngcontent-%COMP%] {\n  margin-right: 8px;\n  margin-bottom: 8px;\n  padding: 8px 16px;\n  cursor: pointer;\n  border: 1px solid #ccc;\n  border-radius: 4px;\n  background-color: #fff;\n  transition: all 0.2s ease;\n}\n\nbutton[_ngcontent-%COMP%]:hover {\n  background-color: #e0e0e0;\n}\n\n.mouse-area[_ngcontent-%COMP%] {\n  padding: 30px;\n  background-color: #f0f0f0;\n  border: 2px dashed #ccc;\n  border-radius: 8px;\n  text-align: center;\n  transition: all 0.2s ease;\n}\n\n.mouse-area.hovering[_ngcontent-%COMP%] {\n  background-color: #e3f2fd;\n  border-color: #2196f3;\n}\n\ninput[type='text'][_ngcontent-%COMP%] {\n  padding: 8px 12px;\n  border: 1px solid #ccc;\n  border-radius: 4px;\n  font-size: 14px;\n  width: 300px;\n  transition: border-color 0.2s ease;\n}\n\ninput[type='text'][_ngcontent-%COMP%]:focus {\n  outline: none;\n  border-color: #2196f3;\n}\n\ninput[type='text'].focused[_ngcontent-%COMP%] {\n  border-color: #4caf50;\n  box-shadow: 0 0 0 2px rgba(76, 175, 80, 0.2);\n}\n\nform[_ngcontent-%COMP%] {\n  margin-bottom: 16px;\n}\n\nform[_ngcontent-%COMP%] input[_ngcontent-%COMP%] {\n  margin-right: 8px;\n}\n\nselect[_ngcontent-%COMP%] {\n  padding: 8px 12px;\n  border: 1px solid #ccc;\n  border-radius: 4px;\n  font-size: 14px;\n}\n\n.event-log[_ngcontent-%COMP%] {\n  max-height: 200px;\n  overflow-y: auto;\n  background-color: #263238;\n  color: #fff;\n  padding: 12px;\n  border-radius: 8px;\n  font-family: monospace;\n  font-size: 12px;\n}\n\n.log-entry[_ngcontent-%COMP%] {\n  padding: 4px 0;\n  border-bottom: 1px solid #37474f;\n}\n\n.log-entry[_ngcontent-%COMP%]:last-child {\n  border-bottom: none;\n}\n\n.log-type[_ngcontent-%COMP%] {\n  color: #4fc3f7;\n  margin-right: 8px;\n}\n\n.log-details[_ngcontent-%COMP%] {\n  color: #fff;\n}\n\n.log-time[_ngcontent-%COMP%] {\n  color: #90a4ae;\n  float: right;\n  font-size: 11px;\n}",
    ],
    dependencies: [NgIf, NgFor, FormsModule],
  });
}
