import { Routes } from '@angular/router';
import { ButtonTestComponent } from './src/components/materials/button/button';
import { AutocompleteTestComponent } from './src/components/materials/autocomplete/autocomplete';
import { InputTestComponent } from './src/components/materials/input/input';
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
    path: '**',
    redirectTo: 'categories',
  },
];
