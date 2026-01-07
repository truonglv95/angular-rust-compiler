import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatChipsModule } from '@angular/material/chips';

@Component({
  selector: 'app-chips-test',
  standalone: true,
  imports: [CommonModule, MatChipsModule],
  templateUrl: './chips.html',
  styleUrls: ['./chips.css'],
})
export class ChipsTestComponent {
  fish = ['One fish', 'Two fish', 'Red fish', 'Blue fish'];
}
