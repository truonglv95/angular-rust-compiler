import { Component } from '@angular/core';
import { MatButtonModule } from '@angular/material/button';
import { MatCardModule } from '@angular/material/card';

@Component({
  selector: 'card-test',
  templateUrl: 'card.html',
  styleUrls: ['card.css'],
  standalone: true,
  imports: [MatCardModule, MatButtonModule],
})
export class CardTestComponent {}
