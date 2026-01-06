import { Routes } from '@angular/router';
import { ButtonTestComponent } from './src/components/materials/button/button';
import { AutocompleteTestComponent } from './src/components/materials/autocomplete/autocomplete';
import { InputTestComponent } from './src/components/materials/input/input';
import { BottomSheetTestComponent } from './src/components/materials/bottom-sheet/bottom-sheet';
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
    path: 'input',
    component: InputTestComponent,
  },
  {
    path: 'bottom-sheet',
    component: BottomSheetTestComponent,
  },
  {
    path: '**',
    redirectTo: 'categories',
  },
];
