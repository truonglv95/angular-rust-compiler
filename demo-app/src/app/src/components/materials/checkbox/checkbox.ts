import { Component } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { MatCheckboxModule } from '@angular/material/checkbox';
import { MatRadioModule } from '@angular/material/radio';
import { MatCardModule } from '@angular/material/card';

@Component({
  selector: 'checkbox-test',
  templateUrl: 'checkbox.html',
  styleUrls: ['checkbox.css'],
  standalone: true,
  imports: [MatCardModule, MatCheckboxModule, MatRadioModule, FormsModule],
})
export class CheckboxTestComponent {
  checked = false;
  indeterminate = false;
  labelPosition: 'before' | 'after' = 'after';
  disabled = false;
}
