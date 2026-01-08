import { Component, ChangeDetectionStrategy } from '@angular/core';
import { MatCardModule } from '@angular/material/card';
import { MatListModule } from '@angular/material/list';
import { RouterLink } from '@angular/router';
import { CommonModule } from '@angular/common';

interface Category {
  name: string;
  summary: string;
  items: { name: string; link?: string }[];
}

@Component({
  selector: 'app-categories',
  standalone: true,
  imports: [CommonModule, MatCardModule, MatListModule, RouterLink],
  templateUrl: './categories.html',
  styleUrl: './categories.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CategoriesComponent {
  categories: Category[] = [
    {
      name: 'Form Controls',
      summary: 'Controls that collect and validate user input.',
      items: [
        { name: 'Autocomplete', link: '/autocomplete' },
        { name: 'Checkbox' },
        { name: 'Datepicker' },
        { name: 'Form field' },
        { name: 'Input', link: '/input' },
        { name: 'Radio button' },
        { name: 'Select' },
        { name: 'Slider' },
        { name: 'Slide toggle' },
      ],
    },
    {
      name: 'Navigation',
      summary: 'Menus, sidenavs and toolbars that organize your content.',
      items: [{ name: 'Menu', link: '/menu' }, { name: 'Sidenav' }, { name: 'Toolbar' }],
    },
    {
      name: 'Layout',
      summary: 'Essential building blocks for presenting your content.',
      items: [
        { name: 'Card' },
        { name: 'Divider' },
        { name: 'Expansion Panel' },
        { name: 'Grid list', link: '/grid-list' },
        { name: 'List', link: '/list' },
        { name: 'Stepper' },
        { name: 'Tabs' },
        { name: 'Tree' },
      ],
    },
    {
      name: 'Buttons & Indicators',
      summary: 'Buttons, toggles, status indicators and progress bars.',
      items: [
        { name: 'Button', link: '/button' },
        { name: 'Button toggle' },
        { name: 'Badge' },
        { name: 'Chips' },
        { name: 'Icon', link: '/icon' },
        { name: 'Progress spinner' },
        { name: 'Progress bar' },
        { name: 'Ripple' },
      ],
    },
    {
      name: 'Popups & Modals',
      summary: 'Dialogs, snackbars, tooltips and bottom sheets.',
      items: [
        { name: 'BottomSheet', link: '/bottom-sheet' },
        { name: 'Dialog' },
        { name: 'SnackBar' },
        { name: 'Tooltip' },
      ],
    },
    {
      name: 'Data Table',
      summary: 'Tools for displaying and interacting with tabular data.',
      items: [{ name: 'Paginator', link: '/paginator' }, { name: 'Sort' }, { name: 'Table' }],
    },
  ];
}
