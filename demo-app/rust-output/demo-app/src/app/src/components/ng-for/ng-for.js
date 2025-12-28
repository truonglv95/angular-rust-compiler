import { NgFor, NgIf } from '@angular/common';
import { ChangeDetectionStrategy, Component } from '@angular/core';
import * as i0 from '@angular/core';
function NgForTest_li_7_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'li');
    i0.ɵɵtext(1);
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const item_r1 = ctx.$implicit;
    const i_r2 = ctx.index;
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate2('', i_r2 + 1, '. ', item_r1);
  }
}
function NgForTest_div_11_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div', 6);
    i0.ɵɵtext(1);
    i0.ɵɵtemplate(2, NgForTest_div_11_span_2_Template, 2, 0, 'span', 7)(
      3,
      NgForTest_div_11_span_3_Template,
      2,
      0,
      'span',
      7,
    )(4, NgForTest_div_11_span_4_Template, 2, 0, 'span', 7)(
      5,
      NgForTest_div_11_span_5_Template,
      2,
      0,
      'span',
      7,
    );
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const num_r3 = ctx.$implicit;
    const isFirst_r4 = ctx.first;
    const isLast_r5 = ctx.last;
    const isEven_r6 = ctx.even;
    const isOdd_r7 = ctx.odd;
    i0.ɵɵclassProp('first', isFirst_r4)('last', isLast_r5)('even', isEven_r6)('odd', isOdd_r7);
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate1(' ', num_r3, ' ');
    i0.ɵɵadvance();
    i0.ɵɵproperty('ngIf', isFirst_r4);
    i0.ɵɵadvance();
    i0.ɵɵproperty('ngIf', isLast_r5);
    i0.ɵɵadvance();
    i0.ɵɵproperty('ngIf', isEven_r6);
    i0.ɵɵadvance();
    i0.ɵɵproperty('ngIf', isOdd_r7);
  }
}
function NgForTest_div_11_span_2_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'span');
    i0.ɵɵtext(1, '(First)');
    i0.ɵɵelementEnd();
  }
}
function NgForTest_div_11_span_3_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'span');
    i0.ɵɵtext(1, '(Last)');
    i0.ɵɵelementEnd();
  }
}
function NgForTest_div_11_span_4_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'span');
    i0.ɵɵtext(1, '[Even]');
    i0.ɵɵelementEnd();
  }
}
function NgForTest_div_11_span_5_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'span');
    i0.ɵɵtext(1, '[Odd]');
    i0.ɵɵelementEnd();
  }
}
function NgForTest_tr_31_Template(rf, ctx) {
  if (rf & 1) {
    const _r8 = i0.ɵɵgetCurrentView();
    i0.ɵɵelementStart(0, 'tr')(1, 'td');
    i0.ɵɵtext(2);
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(3, 'td');
    i0.ɵɵtext(4);
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(5, 'td');
    i0.ɵɵtext(6);
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(7, 'td')(8, 'span');
    i0.ɵɵtext(9);
    i0.ɵɵelementEnd()();
    i0.ɵɵelementStart(10, 'td')(11, 'button', 3);
    i0.ɵɵlistener('click', function NgForTest_tr_31_Template_button_click_11_listener() {
      const user_r9 = i0.ɵɵrestoreView(_r8).$implicit;
      const ctx_r9 = i0.ɵɵnextContext();
      return i0.ɵɵresetView(ctx_r9.toggleActive(user_r9));
    });
    i0.ɵɵtext(12, 'Toggle');
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(13, 'button', 3);
    i0.ɵɵlistener('click', function NgForTest_tr_31_Template_button_click_13_listener() {
      const user_r9 = i0.ɵɵrestoreView(_r8).$implicit;
      const ctx_r9 = i0.ɵɵnextContext();
      return i0.ɵɵresetView(ctx_r9.removeUser(user_r9.id));
    });
    i0.ɵɵtext(14, 'Remove');
    i0.ɵɵelementEnd()()();
  }
  if (rf & 2) {
    const user_r9 = ctx.$implicit;
    i0.ɵɵclassProp('active', user_r9.active)('inactive', !user_r9.active);
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate(user_r9.id);
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate(user_r9.name);
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate(user_r9.email);
    i0.ɵɵadvance(2);
    i0.ɵɵstyleProp('color', user_r9.active ? 'green' : 'red');
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate1(' ', user_r9.active ? 'Active' : 'Inactive', ' ');
  }
}
function NgForTest_div_35_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div', 8)(1, 'h4');
    i0.ɵɵtext(2);
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(3, 'ul');
    i0.ɵɵtemplate(4, NgForTest_div_35_li_4_Template, 2, 2, 'li', 1);
    i0.ɵɵelementEnd()();
  }
  if (rf & 2) {
    const category_r13 = ctx.$implicit;
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate(category_r13.name);
    i0.ɵɵadvance(2);
    i0.ɵɵproperty('ngForOf', category_r13.items);
  }
}
function NgForTest_div_35_li_4_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'li');
    i0.ɵɵtext(1);
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const item_r11 = ctx.$implicit;
    const itemIndex_r12 = ctx.index;
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate2(' ', itemIndex_r12 + 1, '. ', item_r11, ' ');
  }
}
function NgForTest_div_39_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div');
    i0.ɵɵtext(1);
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const item_r14 = ctx.$implicit;
    const total_r15 = ctx.count;
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate2('', item_r14, ' (Total items: ', total_r15, ')');
  }
}
export class NgForTest {
  // Basic array
  items = ['Item 1', 'Item 2', 'Item 3'];
  // Array of objects
  users = [
    {
      id: 1,
      name: 'Alice',
      email: 'alice@example.com',
      active: true,
    },
    {
      id: 2,
      name: 'Bob',
      email: 'bob@example.com',
      active: false,
    },
    {
      id: 3,
      name: 'Charlie',
      email: 'charlie@example.com',
      active: true,
    },
  ];
  // Nested arrays
  categories = [
    {
      name: 'Fruits',
      items: ['Apple', 'Banana', 'Orange'],
    },
    {
      name: 'Vegetables',
      items: ['Carrot', 'Broccoli', 'Spinach'],
    },
    {
      name: 'Dairy',
      items: ['Milk', 'Cheese', 'Yogurt'],
    },
  ];
  // Numbers array for index testing
  numbers = [10, 20, 30, 40, 50];
  // TrackBy function
  trackByUserId(index, user) {
    return user.id;
  }
  trackByIndex(index) {
    return index;
  }
  // Methods for dynamic operations
  addUser() {
    const newId = this.users.length + 1;
    this.users = [
      ...this.users,
      {
        id: newId,
        name: `User ${newId}`,
        email: `user${newId}@example.com`,
        active: true,
      },
    ];
  }
  removeUser(id) {
    this.users = this.users.filter((u) => u.id !== id);
  }
  toggleActive(user) {
    user.active = !user.active;
  }
  static ɵfac = function NgForTest_Factory(t) {
    return new (t || NgForTest)();
  };
  static ɵcmp = /* @__PURE__ */ i0.ɵɵdefineComponent({
    type: NgForTest,
    selectors: [['app-ng-for']],
    decls: 43,
    vars: 7,
    consts: [
      [1, 'ng-for-test'],
      [4, 'ngFor', 'ngForOf'],
      ['class', 'number-item', 3, 'first', 'last', 'even', 'odd', 4, 'ngFor', 'ngForOf'],
      [3, 'click'],
      [3, 'active', 'inactive', 4, 'ngFor', 'ngForOf', 'ngForTrackBy'],
      ['class', 'category', 4, 'ngFor', 'ngForOf'],
      [1, 'number-item'],
      [4, 'ngIf'],
      [1, 'category'],
    ],
    template: function NgForTest_Template(rf, ctx) {
      if (rf & 1) {
        i0.ɵɵelementStart(0, 'div', 0)(1, 'h2');
        i0.ɵɵtext(2, 'NgFor Test Cases');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(3, 'section')(4, 'h3');
        i0.ɵɵtext(5, '1. Basic *ngFor with index');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(6, 'ul');
        i0.ɵɵtemplate(7, NgForTest_li_7_Template, 2, 2, 'li', 1);
        i0.ɵɵelementEnd()();
        i0.ɵɵelementStart(8, 'section')(9, 'h3');
        i0.ɵɵtext(10, '2. Loop variables (first, last, even, odd)');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(11, NgForTest_div_11_Template, 6, 13, 'div', 2);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(12, 'section')(13, 'h3');
        i0.ɵɵtext(14, '3. Objects with property binding');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(15, 'button', 3);
        i0.ɵɵlistener('click', function NgForTest_Template_button_click_15_listener() {
          return ctx.addUser();
        });
        i0.ɵɵtext(16, 'Add User');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(17, 'table')(18, 'thead')(19, 'tr')(20, 'th');
        i0.ɵɵtext(21, 'ID');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(22, 'th');
        i0.ɵɵtext(23, 'Name');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(24, 'th');
        i0.ɵɵtext(25, 'Email');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(26, 'th');
        i0.ɵɵtext(27, 'Status');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(28, 'th');
        i0.ɵɵtext(29, 'Actions');
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(30, 'tbody');
        i0.ɵɵtemplate(31, NgForTest_tr_31_Template, 15, 10, 'tr', 4);
        i0.ɵɵelementEnd()()();
        i0.ɵɵelementStart(32, 'section')(33, 'h3');
        i0.ɵɵtext(34, '4. Nested *ngFor (Categories with items)');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(35, NgForTest_div_35_Template, 5, 2, 'div', 5);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(36, 'section')(37, 'h3');
        i0.ɵɵtext(38, '5. Count variable');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(39, NgForTest_div_39_Template, 2, 2, 'div', 1);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(40, 'section')(41, 'h3');
        i0.ɵɵtext(42);
        i0.ɵɵelementEnd()()();
      }
      if (rf & 2) {
        i0.ɵɵadvance(7);
        i0.ɵɵproperty('ngForOf', ctx.items);
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('ngForOf', ctx.numbers);
        i0.ɵɵadvance(20);
        i0.ɵɵproperty('ngForOf', ctx.users)('ngForTrackBy', ctx.trackByUserId);
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('ngForOf', ctx.categories);
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('ngForOf', ctx.items);
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate1('6. Current Users Count: ', ctx.users.length);
      }
    },
    standalone: true,
    styles: [
      '.ng-for-test[_ngcontent-%COMP%] {\n  padding: 20px;\n  font-family: Arial, sans-serif;\n}\n\nsection[_ngcontent-%COMP%] {\n  margin-bottom: 24px;\n  padding: 16px;\n  border: 1px solid #ddd;\n  border-radius: 8px;\n}\n\nh3[_ngcontent-%COMP%] {\n  margin-top: 0;\n  color: #333;\n}\n\nbutton[_ngcontent-%COMP%] {\n  margin-right: 8px;\n  margin-bottom: 8px;\n  padding: 8px 16px;\n  cursor: pointer;\n}\n\n.number-item[_ngcontent-%COMP%] {\n  padding: 8px 12px;\n  margin: 4px 0;\n  border-radius: 4px;\n  transition: all 0.2s ease;\n}\n\n.number-item.first[_ngcontent-%COMP%] {\n  background-color: #e3f2fd;\n  border-left: 4px solid #2196f3;\n}\n\n.number-item.last[_ngcontent-%COMP%] {\n  background-color: #fce4ec;\n  border-left: 4px solid #e91e63;\n}\n\n.number-item.even[_ngcontent-%COMP%] {\n  background-color: #f5f5f5;\n}\n\n.number-item.odd[_ngcontent-%COMP%] {\n  background-color: #fff;\n}\n\ntable[_ngcontent-%COMP%] {\n  width: 100%;\n  border-collapse: collapse;\n  margin-top: 16px;\n}\n\nth[_ngcontent-%COMP%], \ntd[_ngcontent-%COMP%] {\n  padding: 12px;\n  text-align: left;\n  border-bottom: 1px solid #ddd;\n}\n\nth[_ngcontent-%COMP%] {\n  background-color: #f5f5f5;\n  font-weight: 600;\n}\n\ntr.active[_ngcontent-%COMP%] {\n  background-color: #e8f5e9;\n}\n\ntr.inactive[_ngcontent-%COMP%] {\n  background-color: #ffebee;\n}\n\n.category[_ngcontent-%COMP%] {\n  margin-bottom: 16px;\n  padding: 12px;\n  background-color: #fafafa;\n  border-radius: 8px;\n}\n\n.category[_ngcontent-%COMP%] h4[_ngcontent-%COMP%] {\n  margin: 0 0 8px 0;\n  color: #1976d2;\n}\n\n.ngfor-test[_ngcontent-%COMP%] {\n  padding: 4px 0;\n}',
    ],
    encapsulation: 0,
    changeDetection: 0,
    dependencies: [NgFor, NgIf],
  });
}
