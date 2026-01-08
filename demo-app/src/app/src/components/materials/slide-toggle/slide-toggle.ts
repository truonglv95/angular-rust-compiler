import { Component } from '@angular/core';
import { MatSlideToggleModule } from '@angular/material/slide-toggle';
import { FormsModule } from '@angular/forms';
import { MatCardModule } from '@angular/material/card';
import { MatRadioModule } from '@angular/material/radio';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'slide-toggle-test',
  templateUrl: 'slide-toggle.html',
  styleUrls: ['slide-toggle.css'],
  standalone: true,
  imports: [MatSlideToggleModule, FormsModule, MatCardModule, MatRadioModule, CommonModule],
})
export class SlideToggleTestComponent {
  isChecked = true;
  formGroup = {
    enableWifi: true,
    acceptTerms: false,
  };
}
