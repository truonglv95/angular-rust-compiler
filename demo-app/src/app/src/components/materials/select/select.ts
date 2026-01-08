import { Component } from '@angular/core';
import { MatSelectModule } from '@angular/material/select';
import { MatFormFieldModule } from '@angular/material/form-field';
import { FormsModule } from '@angular/forms';
import { CommonModule } from '@angular/common';
import { MatInputModule } from '@angular/material/input';

interface Food {
  value: string;
  viewValue: string;
}

@Component({
  selector: 'select-test',
  templateUrl: 'select.html',
  styleUrls: ['select.css'],
  standalone: true,
  imports: [MatSelectModule, MatFormFieldModule, MatInputModule, FormsModule, CommonModule],
})
export class SelectTestComponent {
  foods: Food[] = [
    { value: 'steak-0', viewValue: 'Steak' },
    { value: 'pizza-1', viewValue: 'Pizza' },
    { value: 'tacos-2', viewValue: 'Tacos' },
  ];
  selectedFood = this.foods[2].value;
}
