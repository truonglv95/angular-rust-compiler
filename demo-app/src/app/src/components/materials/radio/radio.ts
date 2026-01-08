import { Component } from '@angular/core';
import { MatRadioModule } from '@angular/material/radio';
import { FormsModule } from '@angular/forms';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'radio-test',
  templateUrl: 'radio.html',
  styleUrls: ['radio.css'],
  standalone: true,
  imports: [MatRadioModule, FormsModule, CommonModule],
})
export class RadioTestComponent {
  favoriteSeason: string = 'Spring';
  seasons: string[] = ['Winter', 'Spring', 'Summer', 'Autumn'];
}
