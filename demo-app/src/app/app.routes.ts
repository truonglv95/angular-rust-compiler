import { Routes } from '@angular/router';
import { ButtonTestComponent } from './src/components/materials/button/button';
import { AutocompleteTestComponent } from './src/components/materials/autocomplete/autocomplete';
import { InputTestComponent } from './src/components/materials/input/input';
import { BottomSheetTestComponent } from './src/components/materials/bottom-sheet/bottom-sheet';
import { ButtonToggleOverviewExample } from './src/components/materials/button-toggle/button-toggle';
import { BadgeOverviewExample } from './src/components/materials/badge/badge';
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
    path: '**',
    redirectTo: 'categories',
  },
];
