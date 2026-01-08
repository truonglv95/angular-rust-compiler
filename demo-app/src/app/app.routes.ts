import { Routes } from '@angular/router';
import { ButtonTestComponent } from './src/components/materials/button/button';
import { AutocompleteTestComponent } from './src/components/materials/autocomplete/autocomplete';
import { InputTestComponent } from './src/components/materials/input/input';
import { BottomSheetTestComponent } from './src/components/materials/bottom-sheet/bottom-sheet';
import { ButtonToggleOverviewExample } from './src/components/materials/button-toggle/button-toggle';
import { BadgeOverviewExample } from './src/components/materials/badge/badge';
import { DialogTestComponent } from './src/components/materials/dialog/dialog';
import { DividerTestComponent } from './src/components/materials/divider/divider';
import { ExpansionTestComponent } from './src/components/materials/expansion/expansion';
import { CategoriesComponent } from './src/components/categories/categories';
import { FormFieldTestComponent } from './src/components/materials/form-field/form-field';

export const routes: Routes = [
  {
    path: 'categories',
    component: CategoriesComponent,
  },
  {
    path: 'form-field',
    component: FormFieldTestComponent,
  },
  {
    path: 'button',
    component: ButtonTestComponent,
  },
  {
    path: 'autocomplete',
    component: AutocompleteTestComponent,
  },
  {
    path: 'card',
    loadComponent: () =>
      import('./src/components/materials/card/card').then((m) => m.CardTestComponent),
  },
  {
    path: 'checkbox',
    loadComponent: () =>
      import('./src/components/materials/checkbox/checkbox').then((m) => m.CheckboxTestComponent),
  },
  {
    path: 'chips',
    loadComponent: () =>
      import('./src/components/materials/chips/chips').then((m) => m.ChipsTestComponent),
  },
  {
    path: 'datepicker',
    loadComponent: () =>
      import('./src/components/materials/datepicker/datepicker').then(
        (m) => m.DatepickerTestComponent,
      ),
  },
  {
    path: 'input',
    component: InputTestComponent,
  },
  {
    path: 'bottom-sheet',
    component: BottomSheetTestComponent,
  },
  {
    path: 'grid-list',
    loadComponent: () =>
      import('./src/components/materials/grid-list/grid-list').then((m) => m.GridListTestComponent),
  },
  {
    path: 'icon',
    loadComponent: () =>
      import('./src/components/materials/icon/icon').then((m) => m.IconTestComponent),
  },
  {
    path: 'list',
    loadComponent: () =>
      import('./src/components/materials/list/list').then((m) => m.ListTestComponent),
  },
  {
    path: 'menu',
    loadComponent: () =>
      import('./src/components/materials/menu/menu').then((m) => m.MenuTestComponent),
  },
  {
    path: 'paginator',
    loadComponent: () =>
      import('./src/components/materials/paginator/paginator').then(
        (m) => m.PaginatorTestComponent,
      ),
  },
  {
    path: 'progress-bar',
    loadComponent: () =>
      import('./src/components/materials/progress-bar/progress-bar').then(
        (m) => m.ProgressBarTestComponent,
      ),
  },
  {
    path: 'progress-spinner',
    loadComponent: () =>
      import('./src/components/materials/progress-spinner/progress-spinner').then(
        (m) => m.ProgressSpinnerTestComponent,
      ),
  },
  {
    path: 'radio',
    loadComponent: () =>
      import('./src/components/materials/radio/radio').then((m) => m.RadioTestComponent),
  },
  {
    path: 'ripple',
    loadComponent: () =>
      import('./src/components/materials/ripple/ripple').then((m) => m.RippleTestComponent),
  },
  {
    path: 'select',
    loadComponent: () =>
      import('./src/components/materials/select/select').then((m) => m.SelectTestComponent),
  },
  {
    path: 'sidenav',
    loadComponent: () =>
      import('./src/components/materials/sidenav/sidenav').then((m) => m.SidenavTestComponent),
  },
  {
    path: 'slide-toggle',
    loadComponent: () =>
      import('./src/components/materials/slide-toggle/slide-toggle').then(
        (m) => m.SlideToggleTestComponent,
      ),
  },
  {
    path: 'slider',
    loadComponent: () =>
      import('./src/components/materials/slider/slider').then((m) => m.SliderTestComponent),
  },
  {
    path: 'snack-bar',
    loadComponent: () =>
      import('./src/components/materials/snack-bar/snack-bar').then((m) => m.SnackBarTestComponent),
  },
  {
    path: 'sort',
    loadComponent: () =>
      import('./src/components/materials/sort/sort').then((m) => m.SortTestComponent),
  },
  {
    path: 'stepper',
    loadComponent: () =>
      import('./src/components/materials/stepper/stepper').then((m) => m.StepperTestComponent),
  },
  {
    path: 'table',
    loadComponent: () =>
      import('./src/components/materials/table/table').then((m) => m.TableTestComponent),
  },
  {
    path: 'tabs',
    loadComponent: () =>
      import('./src/components/materials/tabs/tabs').then((m) => m.TabsTestComponent),
  },
  {
    path: 'timepicker',
    loadComponent: () => import('./timepicker/timepicker').then((m) => m.TimepickerTestComponent),
  },
  {
    path: 'toolbar',
    loadComponent: () => import('./toolbar/toolbar').then((m) => m.ToolbarTestComponent),
  },
  {
    path: 'tooltip',
    loadComponent: () => import('./tooltip/tooltip').then((m) => m.TooltipTestComponent),
  },
  {
    path: 'tree',
    loadComponent: () => import('./tree/tree').then((m) => m.TreeTestComponent),
  },
  {
    path: 'button-toggle',
    component: ButtonToggleOverviewExample,
  },
  {
    path: 'badge',
    component: BadgeOverviewExample,
  },
  {
    path: 'dialog',
    component: DialogTestComponent,
  },
  {
    path: 'divider',
    component: DividerTestComponent,
  },
  {
    path: 'expansion',
    component: ExpansionTestComponent,
  },
  {
    path: '**',
    redirectTo: 'categories',
  },
];
