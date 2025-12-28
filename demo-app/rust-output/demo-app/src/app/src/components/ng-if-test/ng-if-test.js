import { NgIf } from '@angular/common';
import { Component } from '@angular/core';
import * as i0 from '@angular/core';
function NgIfTest_p_8_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, '✅ This content is visible (isShow = true)');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_p_9_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, '❌ Alternative content (isShow = false)');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_div_15_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div')(1, 'p');
    i0.ɵɵtext(2);
    i0.ɵɵelementEnd()();
  }
  if (rf & 2) {
    const ctx_r1 = i0.ɵɵnextContext();
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate1('👋 Welcome back, ', ctx_r1.userName, '!');
  }
}
function NgIfTest_ng_template_16_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, '🔒 Please log in to continue');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_div_23_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelement(0, 'div');
  }
}
function NgIfTest_ng_template_24_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, '⏳ Loading...');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_ng_template_26_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, '✅ Content loaded successfully!');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_div_33_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div')(1, 'p');
    i0.ɵɵtext(2);
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(3, 'p');
    i0.ɵɵtext(4);
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(5, 'p');
    i0.ɵɵtext(6);
    i0.ɵɵelementEnd()();
  }
  if (rf & 2) {
    const currentUser_r3 = ctx.ngIf;
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate1('User: ', currentUser_r3.name);
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate1('Role: ', currentUser_r3.role);
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate1('Premium: ', currentUser_r3.premium ? 'Yes' : 'No');
  }
}
function NgIfTest_p_34_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, 'No user data available');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_div_38_Template(rf, ctx) {
  if (rf & 1) {
    const _r4 = i0.ɵɵgetCurrentView();
    i0.ɵɵelementStart(0, 'div');
    i0.ɵɵtemplate(1, NgIfTest_div_38_div_1_Template, 3, 0, 'div', 6)(
      2,
      NgIfTest_div_38_div_2_Template,
      3,
      0,
      'div',
      6,
    )(3, NgIfTest_div_38_div_3_Template, 3, 0, 'div', 6)(
      4,
      NgIfTest_div_38_div_4_Template,
      3,
      0,
      'div',
      6,
    );
    i0.ɵɵelementStart(5, 'div')(6, 'button', 5);
    i0.ɵɵlistener('click', function NgIfTest_div_38_Template_button_click_6_listener() {
      i0.ɵɵrestoreView(_r4);
      const ctx_r4 = i0.ɵɵnextContext();
      return i0.ɵɵresetView(ctx_r4.setUserRole('admin'));
    });
    i0.ɵɵtext(7, 'Set Admin');
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(8, 'button', 5);
    i0.ɵɵlistener('click', function NgIfTest_div_38_Template_button_click_8_listener() {
      i0.ɵɵrestoreView(_r4);
      const ctx_r4 = i0.ɵɵnextContext();
      return i0.ɵɵresetView(ctx_r4.setUserRole('user'));
    });
    i0.ɵɵtext(9, 'Set User');
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(10, 'button', 5);
    i0.ɵɵlistener('click', function NgIfTest_div_38_Template_button_click_10_listener() {
      i0.ɵɵrestoreView(_r4);
      const ctx_r4 = i0.ɵɵnextContext();
      return i0.ɵɵresetView(ctx_r4.setUserRole('guest'));
    });
    i0.ɵɵtext(11, 'Set Guest');
    i0.ɵɵelementEnd();
    i0.ɵɵelementStart(12, 'button', 5);
    i0.ɵɵlistener('click', function NgIfTest_div_38_Template_button_click_12_listener() {
      i0.ɵɵrestoreView(_r4);
      const ctx_r4 = i0.ɵɵnextContext();
      return i0.ɵɵresetView(ctx_r4.togglePremium());
    });
    i0.ɵɵtext(13, 'Toggle Premium');
    i0.ɵɵelementEnd()()();
  }
  if (rf & 2) {
    const ctx_r4 = i0.ɵɵnextContext();
    i0.ɵɵadvance();
    i0.ɵɵproperty('ngIf', ctx_r4.user.role === 'admin');
    i0.ɵɵadvance();
    i0.ɵɵproperty('ngIf', ctx_r4.user.role === 'user');
    i0.ɵɵadvance();
    i0.ɵɵproperty('ngIf', ctx_r4.user.role === 'guest');
    i0.ɵɵadvance();
    i0.ɵɵproperty('ngIf', ctx_r4.user.premium);
  }
}
function NgIfTest_div_38_div_1_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div')(1, 'p');
    i0.ɵɵtext(2, '🔑 Admin Panel Access');
    i0.ɵɵelementEnd()();
  }
}
function NgIfTest_div_38_div_2_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div')(1, 'p');
    i0.ɵɵtext(2, '👤 User Dashboard');
    i0.ɵɵelementEnd()();
  }
}
function NgIfTest_div_38_div_3_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div')(1, 'p');
    i0.ɵɵtext(2, '👁️ Guest View Only');
    i0.ɵɵelementEnd()();
  }
}
function NgIfTest_div_38_div_4_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div')(1, 'p');
    i0.ɵɵtext(2, '⭐ Premium Features Enabled');
    i0.ɵɵelementEnd()();
  }
}
function NgIfTest_p_49_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, 'Counter is at zero');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_p_50_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, 'Counter in progress...');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_p_51_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, '🎉 Maximum reached!');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_div_59_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div')(1, 'p');
    i0.ɵɵtext(2);
    i0.ɵɵelementEnd()();
  }
  if (rf & 2) {
    const ctx_r5 = i0.ɵɵnextContext();
    i0.ɵɵadvance(2);
    i0.ɵɵtextInterpolate1('You have ', ctx_r5.items.length, ' item(s)');
  }
}
function NgIfTest_ng_template_60_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'p');
    i0.ɵɵtext(1, '📭 No items in the list');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_button_67_Template(rf, ctx) {
  if (rf & 1) {
    const _r7 = i0.ɵɵgetCurrentView();
    i0.ɵɵelementStart(0, 'button', 5);
    i0.ɵɵlistener('click', function NgIfTest_button_67_Template_button_click_0_listener() {
      i0.ɵɵrestoreView(_r7);
      const ctx_r7 = i0.ɵɵnextContext();
      return i0.ɵɵresetView(ctx_r7.clearError());
    });
    i0.ɵɵtext(1, 'Clear Error');
    i0.ɵɵelementEnd();
  }
}
function NgIfTest_div_68_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div', 12);
    i0.ɵɵtext(1);
    i0.ɵɵelementEnd();
  }
  if (rf & 2) {
    const ctx_r8 = i0.ɵɵnextContext();
    i0.ɵɵadvance();
    i0.ɵɵtextInterpolate1('⚠️ ', ctx_r8.errorMessage);
  }
}
function NgIfTest_div_69_Template(rf, ctx) {
  if (rf & 1) {
    i0.ɵɵelementStart(0, 'div');
    i0.ɵɵtext(1, '✅ No errors');
    i0.ɵɵelementEnd();
  }
}
export class NgIfTest {
  // Basic boolean
  isShow = true;
  isLoggedIn = false;
  isLoading = false;
  // Nullable values
  userName = 'John Doe';
  errorMessage = null;
  // Numeric conditions
  count = 0;
  maxCount = 5;
  // Object for complex conditions
  user = {
    name: 'Admin User',
    role: 'admin',
    premium: true,
  };
  // Array for empty check
  items = ['Item 1', 'Item 2'];
  // Methods
  toggleShow() {
    this.isShow = !this.isShow;
  }
  toggleLogin() {
    this.isLoggedIn = !this.isLoggedIn;
    if (this.isLoggedIn) {
      this.userName = 'John Doe';
    } else {
      this.userName = null;
    }
  }
  simulateLoading() {
    this.isLoading = true;
    this.errorMessage = null;
    setTimeout(() => {
      this.isLoading = false;
    }, 2e3);
  }
  simulateError() {
    this.errorMessage = 'Something went wrong! Please try again.';
  }
  clearError() {
    this.errorMessage = null;
  }
  increment() {
    if (this.count < this.maxCount) {
      this.count++;
    }
  }
  decrement() {
    if (this.count > 0) {
      this.count--;
    }
  }
  toggleUser() {
    if (this.user) {
      this.user = null;
    } else {
      this.user = {
        name: 'Admin User',
        role: 'admin',
        premium: true,
      };
    }
  }
  setUserRole(role) {
    if (this.user) {
      this.user = {
        ...this.user,
        role,
      };
    }
  }
  togglePremium() {
    if (this.user) {
      this.user = {
        ...this.user,
        premium: !this.user.premium,
      };
    }
  }
  addItem() {
    this.items = [...this.items, `Item ${this.items.length + 1}`];
  }
  clearItems() {
    this.items = [];
  }
  static ɵfac = function NgIfTest_Factory(t) {
    return new (t || NgIfTest)();
  };
  static ɵcmp = /* @__PURE__ */ i0.ɵɵdefineComponent({
    type: NgIfTest,
    selectors: [['app-ng-if-test']],
    decls: 70,
    vars: 24,
    consts: [
      ['loggedOutTemplate', ''],
      ['loadingTpl', ''],
      ['contentTpl', ''],
      ['noItemsTpl', ''],
      [1, 'ng-if-test'],
      [3, 'click'],
      [4, 'ngIf'],
      [4, 'ngIf', 'ngIfElse'],
      [4, 'ngIf', 'ngIfThen', 'ngIfElse'],
      [3, 'click', 'disabled'],
      [3, 'click', 4, 'ngIf'],
      ['class', 'error-message', 4, 'ngIf'],
      [1, 'error-message'],
    ],
    template: function NgIfTest_Template(rf, ctx) {
      if (rf & 1) {
        const _r1 = i0.ɵɵgetCurrentView();
        i0.ɵɵelementStart(0, 'div', 4)(1, 'h2');
        i0.ɵɵtext(2, 'NgIf Test Cases');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(3, 'section')(4, 'h3');
        i0.ɵɵtext(5, '1. Basic *ngIf Toggle');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(6, 'button', 5);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_6_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.toggleShow());
        });
        i0.ɵɵtext(7, 'Toggle Show');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(8, NgIfTest_p_8_Template, 2, 0, 'p', 6)(
          9,
          NgIfTest_p_9_Template,
          2,
          0,
          'p',
          6,
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(10, 'section')(11, 'h3');
        i0.ɵɵtext(12, '2. *ngIf with else');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(13, 'button', 5);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_13_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.toggleLogin());
        });
        i0.ɵɵtext(14);
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(15, NgIfTest_div_15_Template, 3, 1, 'div', 7)(
          16,
          NgIfTest_ng_template_16_Template,
          2,
          0,
          'ng-template',
          null,
          0,
          i0.ɵɵtemplateRefExtractor,
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(18, 'section')(19, 'h3');
        i0.ɵɵtext(20, '3. *ngIf with then/else templates');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(21, 'button', 5);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_21_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.simulateLoading());
        });
        i0.ɵɵtext(22, 'Load Data');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(23, NgIfTest_div_23_Template, 1, 0, 'div', 8)(
          24,
          NgIfTest_ng_template_24_Template,
          2,
          0,
          'ng-template',
          null,
          1,
          i0.ɵɵtemplateRefExtractor,
        )(
          26,
          NgIfTest_ng_template_26_Template,
          2,
          0,
          'ng-template',
          null,
          2,
          i0.ɵɵtemplateRefExtractor,
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(28, 'section')(29, 'h3');
        i0.ɵɵtext(30, '4. *ngIf with nullable and "as" syntax');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(31, 'button', 5);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_31_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.toggleUser());
        });
        i0.ɵɵtext(32);
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(33, NgIfTest_div_33_Template, 7, 3, 'div', 6)(
          34,
          NgIfTest_p_34_Template,
          2,
          0,
          'p',
          6,
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(35, 'section')(36, 'h3');
        i0.ɵɵtext(37, '5. Nested *ngIf with complex conditions');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(38, NgIfTest_div_38_Template, 14, 4, 'div', 6);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(39, 'section')(40, 'h3');
        i0.ɵɵtext(41, '6. Numeric conditions');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(42, 'div')(43, 'button', 9);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_43_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.decrement());
        });
        i0.ɵɵtext(44, '-');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(45, 'span');
        i0.ɵɵtext(46);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(47, 'button', 9);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_47_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.increment());
        });
        i0.ɵɵtext(48, '+');
        i0.ɵɵelementEnd()();
        i0.ɵɵtemplate(49, NgIfTest_p_49_Template, 2, 0, 'p', 6)(
          50,
          NgIfTest_p_50_Template,
          2,
          0,
          'p',
          6,
        )(51, NgIfTest_p_51_Template, 2, 0, 'p', 6);
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(52, 'section')(53, 'h3');
        i0.ɵɵtext(54, '7. Array length conditions');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(55, 'button', 5);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_55_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.addItem());
        });
        i0.ɵɵtext(56, 'Add Item');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(57, 'button', 5);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_57_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.clearItems());
        });
        i0.ɵɵtext(58, 'Clear All');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(59, NgIfTest_div_59_Template, 3, 1, 'div', 7)(
          60,
          NgIfTest_ng_template_60_Template,
          2,
          0,
          'ng-template',
          null,
          3,
          i0.ɵɵtemplateRefExtractor,
        );
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(62, 'section')(63, 'h3');
        i0.ɵɵtext(64, '8. Error handling pattern');
        i0.ɵɵelementEnd();
        i0.ɵɵelementStart(65, 'button', 5);
        i0.ɵɵlistener('click', function NgIfTest_Template_button_click_65_listener() {
          i0.ɵɵrestoreView(_r1);
          return i0.ɵɵresetView(ctx.simulateError());
        });
        i0.ɵɵtext(66, 'Trigger Error');
        i0.ɵɵelementEnd();
        i0.ɵɵtemplate(67, NgIfTest_button_67_Template, 2, 0, 'button', 10)(
          68,
          NgIfTest_div_68_Template,
          2,
          1,
          'div',
          11,
        )(69, NgIfTest_div_69_Template, 2, 0, 'div', 6);
        i0.ɵɵelementEnd()();
      }
      if (rf & 2) {
        const loggedOutTemplate_r10 = i0.ɵɵreference(17);
        const loadingTpl_r11 = i0.ɵɵreference(25);
        const contentTpl_r12 = i0.ɵɵreference(27);
        const noItemsTpl_r13 = i0.ɵɵreference(61);
        i0.ɵɵadvance(8);
        i0.ɵɵproperty('ngIf', ctx.isShow);
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', !ctx.isShow);
        i0.ɵɵadvance(5);
        i0.ɵɵtextInterpolate(ctx.isLoggedIn ? 'Logout' : 'Login');
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', ctx.isLoggedIn)('ngIfElse', loggedOutTemplate_r10);
        i0.ɵɵadvance(8);
        i0.ɵɵproperty('ngIf', ctx.isLoading)('ngIfThen', loadingTpl_r11)(
          'ngIfElse',
          contentTpl_r12,
        );
        i0.ɵɵadvance(9);
        i0.ɵɵtextInterpolate(ctx.user ? 'Remove User' : 'Add User');
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', ctx.user);
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', !ctx.user);
        i0.ɵɵadvance(4);
        i0.ɵɵproperty('ngIf', ctx.user);
        i0.ɵɵadvance(5);
        i0.ɵɵproperty('disabled', ctx.count === 0);
        i0.ɵɵadvance(3);
        i0.ɵɵtextInterpolate2(' ', ctx.count, ' / ', ctx.maxCount, ' ');
        i0.ɵɵadvance();
        i0.ɵɵproperty('disabled', ctx.count >= ctx.maxCount);
        i0.ɵɵadvance(2);
        i0.ɵɵproperty('ngIf', ctx.count === 0);
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', ctx.count > 0 && ctx.count < ctx.maxCount);
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', ctx.count >= ctx.maxCount);
        i0.ɵɵadvance(8);
        i0.ɵɵproperty('ngIf', ctx.items.length > 0)('ngIfElse', noItemsTpl_r13);
        i0.ɵɵadvance(8);
        i0.ɵɵproperty('ngIf', ctx.errorMessage);
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', ctx.errorMessage);
        i0.ɵɵadvance();
        i0.ɵɵproperty('ngIf', !ctx.errorMessage && !ctx.isLoading);
      }
    },
    standalone: true,
    styles: [
      '.ng-if-test[_ngcontent-%COMP%] {\n  padding: 20px;\n  font-family: Arial, sans-serif;\n}\n\nsection[_ngcontent-%COMP%] {\n  margin-bottom: 24px;\n  padding: 16px;\n  border: 1px solid #ddd;\n  border-radius: 8px;\n}\n\nh3[_ngcontent-%COMP%] {\n  margin-top: 0;\n  color: #333;\n}\n\nbutton[_ngcontent-%COMP%] {\n  margin-right: 8px;\n  margin-bottom: 8px;\n  padding: 8px 16px;\n  cursor: pointer;\n  border: 1px solid #ccc;\n  border-radius: 4px;\n  background-color: #fff;\n  transition: all 0.2s ease;\n}\n\nbutton[_ngcontent-%COMP%]:hover {\n  background-color: #f0f0f0;\n}\n\nbutton[_ngcontent-%COMP%]:disabled {\n  opacity: 0.5;\n  cursor: not-allowed;\n}\n\n.error-message[_ngcontent-%COMP%] {\n  padding: 12px;\n  background-color: #ffebee;\n  color: #c62828;\n  border-left: 4px solid #c62828;\n  border-radius: 4px;\n  margin-top: 8px;\n}\n\np[_ngcontent-%COMP%] {\n  margin: 8px 0;\n}',
    ],
    encapsulation: 0,
    dependencies: [NgIf],
  });
}
