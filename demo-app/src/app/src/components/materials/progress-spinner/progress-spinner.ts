import { Component } from '@angular/core';
import { MatProgressSpinnerModule } from '@angular/material/progress-spinner';
import { MatSliderModule } from '@angular/material/slider';
import { FormsModule } from '@angular/forms';
import { MatRadioModule } from '@angular/material/radio';
import { MatCardModule } from '@angular/material/card';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-progress-spinner',
  standalone: true,
  imports: [
    MatProgressSpinnerModule,
    MatSliderModule,
    FormsModule,
    MatRadioModule,
    MatCardModule,
    CommonModule,
  ],
  templateUrl: './progress-spinner.html',
  styleUrl: './progress-spinner.css',
})
export class ProgressSpinnerTestComponent {
  mode: 'determinate' | 'indeterminate' = 'determinate';
  value = 50;
}
