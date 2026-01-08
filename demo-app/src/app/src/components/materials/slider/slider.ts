import { Component } from '@angular/core';
import { MatSliderModule } from '@angular/material/slider';
import { FormsModule } from '@angular/forms';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'slider-test',
  templateUrl: 'slider.html',
  styleUrls: ['slider.css'],
  standalone: true,
  imports: [MatSliderModule, FormsModule, CommonModule],
})
export class SliderTestComponent {
  value = 50;
  min = 0;
  max = 100;

  formatLabel(value: number): string {
    if (value >= 1000) {
      return Math.round(value / 1000) + 'k';
    }
    return `${value}`;
  }
}
