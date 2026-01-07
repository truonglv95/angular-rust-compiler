import { Routes } from '@angular/router';
import { ButtonTestComponent } from './src/components/materials/button/button';
import { AutocompleteTestComponent } from './src/components/materials/autocomplete/autocomplete';
import { InputTestComponent } from './src/components/materials/input/input';
import { BottomSheetTestComponent } from './src/components/materials/bottom-sheet/bottom-sheet';
import { ButtonToggleOverviewExample } from './src/components/materials/button-toggle/button-toggle';
import { BadgeOverviewExample } from './src/components/materials/badge/badge';
import { DialogTestComponent } from './src/components/materials/dialog/dialog';
import { CategoriesComponent } from './src/components/categories/categories';

export const routes: Routes = [
  {
    path: 'categories',
    component: CategoriesComponent,
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
    path: '**',
    redirectTo: 'categories',
  },
];
