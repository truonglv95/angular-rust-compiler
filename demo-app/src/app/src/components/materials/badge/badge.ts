import { Component } from '@angular/core';
import { MatBadgeModule } from '@angular/material/badge';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';

/**
 * @title Badge overview
 */
@Component({
  selector: 'badge-overview-example',
  templateUrl: 'badge.html',
  styleUrls: ['badge.css'],
  standalone: true,
  imports: [MatBadgeModule, MatButtonModule, MatIconModule],
})
export class BadgeOverviewExample {
  hidden = false;

  toggleBadgeVisibility() {
    this.hidden = !this.hidden;
  }
}
